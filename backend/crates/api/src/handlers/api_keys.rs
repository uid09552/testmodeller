//! `/me/api-keys`: the caller's own API keys (FR-090).
//!
//! Scoped by the caller's `sub` and tenant, so the tenant guard has nothing to
//! resolve here, and exempt from the role rule: managing one's own keys is not
//! a write to project data (08-usermanagement.md).

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tm_storage::{ApiKeyRecord, NewApiKey, TenantScope};
use uuid::Uuid;

use crate::api_keys;
use crate::auth::Identity;
use crate::error::ApiError;
use crate::extract::{ApiJson, ApiPath};
use crate::tenant_guard::require_tenant;
use crate::AppState;

const DEFAULT_DAYS: i64 = 90;
const MAX_DAYS: i64 = 365;
const MAX_NAME: usize = 80;

/// A key as listed; the secret is never part of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyDto {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub role: String,
    pub project_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// A new key, with the secret shown this once.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyCreatedDto {
    #[serde(flatten)]
    pub key_info: ApiKeyDto,
    pub key: String,
}

/// `POST /me/api-keys` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApiKeyInput {
    pub name: String,
    pub expires_in_days: Option<i64>,
    /// Makes an upload key for this project (ADR 0018).
    #[serde(default)]
    pub project_id: Option<Uuid>,
}

impl From<ApiKeyRecord> for ApiKeyDto {
    fn from(r: ApiKeyRecord) -> Self {
        Self {
            id: r.id,
            name: r.name,
            prefix: r.prefix,
            role: r.role,
            project_id: r.project_id,
            created_at: r.created_at,
            expires_at: r.expires_at,
            last_used_at: r.last_used_at,
        }
    }
}

/// The caller's `sub`; a token without one cannot own keys.
fn owner(identity: &Identity) -> Result<&str, ApiError> {
    identity
        .subject
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::FORBIDDEN,
                "the token carries no \"sub\" claim, so it cannot own API keys",
            )
        })
}

/// `GET /me/api-keys`
pub async fn list(
    State(state): State<AppState>,
    identity: Identity,
) -> Result<Json<Vec<ApiKeyDto>>, ApiError> {
    let keys = state
        .store
        .list_api_keys(&identity.tenant, owner(&identity)?)
        .await?;
    Ok(Json(keys.into_iter().map(Into::into).collect()))
}

/// `POST /me/api-keys`
pub async fn create(
    State(state): State<AppState>,
    identity: Identity,
    ApiJson(input): ApiJson<ApiKeyInput>,
) -> Result<(StatusCode, Json<ApiKeyCreatedDto>), ApiError> {
    let subject = owner(&identity)?;
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Err(ApiError::unprocessable(format!(
            "name must be 1 to {MAX_NAME} characters"
        )));
    }
    let days = input.expires_in_days.unwrap_or(DEFAULT_DAYS);
    if !(1..=MAX_DAYS).contains(&days) {
        return Err(ApiError::unprocessable(format!(
            "expiresInDays must be between 1 and {MAX_DAYS}"
        )));
    }

    // An upload key writes test results of one project: that needs the Editor role, and the
    // project is named in the body, so the tenant guard cannot see it (AGENTS.md rule 10).
    if let Some(project) = input.project_id {
        if identity.role != crate::auth::Role::Editor {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "the Editor role is needed to create a key that uploads test results",
            ));
        }
        require_tenant(&state, &identity, TenantScope::Project, project).await?;
    }

    let key = api_keys::generate();
    let prefix = api_keys::display_prefix(&key);
    let hash = api_keys::hash(&key);
    let record = state
        .store
        .create_api_key(NewApiKey {
            tenant: &identity.tenant,
            subject,
            name,
            prefix: &prefix,
            hash: &hash,
            role: identity.role.as_str(),
            project_id: input.project_id,
            expires_at: Some(Utc::now() + Duration::days(days)),
        })
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(ApiKeyCreatedDto {
            key_info: record.into(),
            key,
        }),
    ))
}

/// `DELETE /me/api-keys/{id}`
pub async fn revoke(
    State(state): State<AppState>,
    identity: Identity,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .store
        .revoke_api_key(&identity.tenant, owner(&identity)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
