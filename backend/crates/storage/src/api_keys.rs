//! Per-user API keys for the MCP endpoint (FR-090).
//!
//! Only the SHA-256 hash of a key is stored; hashing happens in the API crate,
//! so this module never sees a key. A key is scoped to the `subject` and tenant
//! that created it. See docs/adr/0015-api-keys-and-mcp.md.

use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use sqlx::Row;
use uuid::Uuid;

use crate::{Result, StorageError, Store};

/// Most active (not revoked, not expired) keys one user may hold.
pub const MAX_ACTIVE_KEYS: i64 = 10;

/// A stored key, without its hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiKeyRecord {
    /// Key id.
    pub id: Uuid,
    /// Tenant the key acts in.
    pub tenant: String,
    /// `sub` of the user who owns it.
    pub subject: String,
    /// Label chosen by the user.
    pub name: String,
    /// First characters of the key, to recognise it.
    pub prefix: String,
    /// `User` or `Editor`, taken from the creator's token.
    pub role: String,
    /// When it was created.
    pub created_at: DateTime<Utc>,
    /// When it stops working, if ever.
    pub expires_at: Option<DateTime<Utc>>,
    /// Last successful use, updated at most once a minute.
    pub last_used_at: Option<DateTime<Utc>>,
}

/// What a new key is stored with.
#[derive(Debug, Clone)]
pub struct NewApiKey<'a> {
    /// Tenant the key acts in.
    pub tenant: &'a str,
    /// Owner's `sub`.
    pub subject: &'a str,
    /// Label.
    pub name: &'a str,
    /// Displayed prefix.
    pub prefix: &'a str,
    /// SHA-256 of the key.
    pub hash: &'a [u8],
    /// `User` or `Editor`.
    pub role: &'a str,
    /// Expiry.
    pub expires_at: Option<DateTime<Utc>>,
}

const COLUMNS: &str =
    "id, tenant_id, subject, name, prefix, role, created_at, expires_at, last_used_at";

/// Active means neither revoked nor expired.
const ACTIVE: &str = "revoked_at IS NULL AND (expires_at IS NULL OR expires_at > now())";

fn record(row: &PgRow) -> Result<ApiKeyRecord, sqlx::Error> {
    Ok(ApiKeyRecord {
        id: row.try_get("id")?,
        tenant: row.try_get("tenant_id")?,
        subject: row.try_get("subject")?,
        name: row.try_get("name")?,
        prefix: row.try_get("prefix")?,
        role: row.try_get("role")?,
        created_at: row.try_get("created_at")?,
        expires_at: row.try_get("expires_at")?,
        last_used_at: row.try_get("last_used_at")?,
    })
}

impl Store {
    /// The owner's active keys, newest first.
    pub async fn list_api_keys(&self, tenant: &str, subject: &str) -> Result<Vec<ApiKeyRecord>> {
        let rows = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM api_keys
             WHERE tenant_id = $1 AND subject = $2 AND {ACTIVE}
             ORDER BY created_at DESC, id"
        ))
        .bind(tenant)
        .bind(subject)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(record).collect::<Result<_, _>>()?)
    }

    /// Stores a key, refusing once the owner holds [`MAX_ACTIVE_KEYS`].
    ///
    /// The count and the insert run in one transaction that locks the owner's
    /// rows, so two concurrent requests cannot both slip under the limit.
    pub async fn create_api_key(&self, key: NewApiKey<'_>) -> Result<ApiKeyRecord> {
        let mut tx = self.pool.begin().await?;
        // Serialises key creation per owner for the rest of the transaction.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1 || '/' || $2))")
            .bind(key.tenant)
            .bind(key.subject)
            .execute(&mut *tx)
            .await?;
        let active: i64 = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM api_keys WHERE tenant_id = $1 AND subject = $2 AND {ACTIVE}"
        ))
        .bind(key.tenant)
        .bind(key.subject)
        .fetch_one(&mut *tx)
        .await?;
        if active >= MAX_ACTIVE_KEYS {
            return Err(StorageError::Invalid(format!(
                "at most {MAX_ACTIVE_KEYS} active API keys per user; revoke one first"
            )));
        }
        let row = sqlx::query(&format!(
            "INSERT INTO api_keys (id, tenant_id, subject, name, prefix, key_hash, role, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING {COLUMNS}"
        ))
        .bind(Uuid::new_v4())
        .bind(key.tenant)
        .bind(key.subject)
        .bind(key.name)
        .bind(key.prefix)
        .bind(key.hash)
        .bind(key.role)
        .bind(key.expires_at)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(record(&row)?)
    }

    /// Revokes one of the owner's keys. Another owner's key is "not found".
    pub async fn revoke_api_key(&self, tenant: &str, subject: &str, id: Uuid) -> Result<()> {
        let done = sqlx::query(
            "UPDATE api_keys SET revoked_at = now()
             WHERE id = $1 AND tenant_id = $2 AND subject = $3 AND revoked_at IS NULL",
        )
        .bind(id)
        .bind(tenant)
        .bind(subject)
        .execute(&self.pool)
        .await?;
        if done.rows_affected() == 0 {
            return Err(StorageError::NotFound("API key"));
        }
        Ok(())
    }

    /// The active key with this hash, recording the use.
    ///
    /// `last_used_at` is written at most once a minute per key, so a busy
    /// client does not turn every tool call into a row update.
    pub async fn use_api_key(&self, hash: &[u8]) -> Result<Option<ApiKeyRecord>> {
        let row = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM api_keys WHERE key_hash = $1 AND {ACTIVE}"
        ))
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else { return Ok(None) };
        let key = record(&row)?;
        sqlx::query(
            "UPDATE api_keys SET last_used_at = now()
             WHERE id = $1 AND (last_used_at IS NULL OR last_used_at < now() - interval '1 minute')",
        )
        .bind(key.id)
        .execute(&self.pool)
        .await?;
        Ok(Some(key))
    }
}
