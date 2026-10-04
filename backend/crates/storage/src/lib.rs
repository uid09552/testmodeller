//! PostgreSQL persistence (sqlx). Migrations live in `migrations/` and are embedded at compile time.
//! See docs/adr/0002-postgresql-storage.md.

mod ai;
mod export;
mod models;
mod organization;
mod test_cases;

use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::Row;
use tm_domain::Audit;
use uuid::Uuid;

pub use ai::Acceptance;
pub use ai::ProposalFilter;
pub use export::{
    ExportBundle, ExportComponent, ExportFeature, ExportScope, ImportComponent, ImportCounts,
    ImportFeature, ImportModel, ImportTestCase,
};
pub use models::{ModelMeta, StateRecord, TransitionRecord};
pub use organization::{
    FeatureFields, NamedFields, SearchHit, SearchType, TreeComponent, TreeFeature, TreeModel,
};
pub use test_cases::{NewAssignment, TestCaseFilter};

/// Storage errors.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Entity does not exist.
    #[error("{0} not found")]
    NotFound(&'static str),
    /// `If-Match` version does not match the stored version.
    #[error("version mismatch: entity was modified concurrently")]
    VersionMismatch,
    /// Operation conflicts with current state (e.g. non-empty parent).
    #[error("{0}")]
    Conflict(String),
    /// Input violates a domain rule.
    #[error("{0}")]
    Invalid(String),
    /// Database failure.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    /// Stored JSON could not be (de)serialized.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result alias for storage operations.
pub type Result<T, E = StorageError> = std::result::Result<T, E>;

/// A page of results with an opaque cursor for the next page.
#[derive(Debug, Clone, PartialEq)]
pub struct Page<T> {
    /// Items on this page.
    pub items: Vec<T>,
    /// Cursor for the next page, `None` on the last page.
    pub next_cursor: Option<String>,
}

/// Keyset pagination position: `(created_at, id)` of the last item seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    created_at: DateTime<Utc>,
    id: Uuid,
}

impl Cursor {
    /// Encodes as `<micros>_<uuid>`.
    pub fn encode(&self) -> String {
        format!("{}_{}", self.created_at.timestamp_micros(), self.id)
    }

    /// Decodes a cursor produced by [`Cursor::encode`].
    pub fn decode(s: &str) -> Option<Self> {
        let (micros, id) = s.split_once('_')?;
        Some(Self {
            created_at: DateTime::from_timestamp_micros(micros.parse().ok()?)?,
            id: Uuid::parse_str(id).ok()?,
        })
    }
}

/// Pagination request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    /// Maximum number of items.
    pub limit: i64,
    /// Position after which to continue.
    pub after: Option<Cursor>,
}

impl PageRequest {
    fn cursor_parts(&self) -> (Option<DateTime<Utc>>, Option<Uuid>) {
        (self.after.map(|c| c.created_at), self.after.map(|c| c.id))
    }
}

/// Builds a page from `limit + 1` fetched rows.
fn into_page<T>(
    mut items: Vec<T>,
    limit: i64,
    key: impl Fn(&T) -> (DateTime<Utc>, Uuid),
) -> Page<T> {
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    let next_cursor = if items.len() > limit {
        items.truncate(limit);
        items.last().map(|i| {
            let (created_at, id) = key(i);
            Cursor { created_at, id }.encode()
        })
    } else {
        None
    };
    Page { items, next_cursor }
}

/// SQL fragment for keyset pagination; expects `$cursor_ts` and `$cursor_id` placeholders.
fn keyset(ts: usize, id: usize) -> String {
    format!("(${ts}::timestamptz IS NULL OR (created_at, id) > (${ts}, ${id}::uuid))")
}

fn audit(row: &PgRow) -> Result<Audit, sqlx::Error> {
    Ok(Audit {
        id: row.try_get("id")?,
        version: row.try_get("version")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn parse_col<T>(row: &PgRow, col: &str) -> Result<T, sqlx::Error>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let raw: String = row.try_get(col)?;
    raw.parse().map_err(|e| sqlx::Error::ColumnDecode {
        index: col.to_owned(),
        source: Box::new(e),
    })
}

fn parse_opt_col<T>(row: &PgRow, col: &str) -> Result<Option<T>, sqlx::Error>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let raw: Option<String> = row.try_get(col)?;
    raw.map(|r| r.parse())
        .transpose()
        .map_err(|e| sqlx::Error::ColumnDecode {
            index: col.to_owned(),
            source: Box::new(e),
        })
}

/// Handle to the database. Cheap to clone.
#[derive(Debug, Clone)]
pub struct Store {
    pool: PgPool,
}

impl Store {
    /// Connects to `database_url` and runs pending migrations.
    pub async fn connect(database_url: &str, max_connections: u32) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    /// Applies embedded migrations.
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.into()))
    }

    /// Checks database connectivity.
    pub async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}

/// Maps "no row updated" after an optimistic update to `NotFound` or `VersionMismatch`.
async fn missing_or_conflict(
    conn: &mut sqlx::PgConnection,
    table: &'static str,
    entity: &'static str,
    id: Uuid,
) -> StorageError {
    let sql = format!("SELECT 1 FROM {table} WHERE id = $1");
    match sqlx::query(&sql).bind(id).fetch_optional(&mut *conn).await {
        Ok(Some(_)) => StorageError::VersionMismatch,
        Ok(None) => StorageError::NotFound(entity),
        Err(e) => e.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trips() {
        let c = Cursor {
            created_at: DateTime::from_timestamp_micros(1_700_000_000_123_456).unwrap(),
            id: Uuid::new_v4(),
        };
        assert_eq!(Cursor::decode(&c.encode()), Some(c));
        assert_eq!(Cursor::decode("garbage"), None);
    }
}
