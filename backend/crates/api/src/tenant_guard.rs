//! Cross-tenant request rejection (FR-044, NFR-007).
//!
//! Almost every route names its subject with an id in the path. This
//! middleware resolves each id to the tenant that owns it and compares it with
//! the caller's tenant, so no handler has to remember to. See
//! docs/adr/0005-jwt-auth-and-tenancy.md.
//!
//! A mismatch answers 404, not 403: whether an id exists in another tenant is
//! not the caller's business.

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use tm_storage::TenantScope;
use uuid::Uuid;

use crate::auth::Identity;
use crate::error::ApiError;
use crate::AppState;

/// The entity kind a path segment introduces.
fn scope_for(segment: &str) -> Option<TenantScope> {
    match segment {
        "projects" => Some(TenantScope::Project),
        "components" => Some(TenantScope::Component),
        "features" => Some(TenantScope::Feature),
        "models" => Some(TenantScope::Model),
        "states" => Some(TenantScope::State),
        "transitions" => Some(TenantScope::Transition),
        "test-cases" => Some(TenantScope::TestCase),
        "proposals" => Some(TenantScope::Proposal),
        "jobs" => Some(TenantScope::Job),
        _ => None,
    }
}

/// Every id in the path, paired with the entity kind that owns it.
///
/// An id belongs to the nearest collection segment before it, which is what
/// makes `/states/{id}/test-cases/{tc}` resolve to a state and a test case
/// rather than to two states.
fn scoped_ids(path: &str) -> Result<Vec<(TenantScope, Uuid)>, &str> {
    let mut found = Vec::new();
    let mut collection: Option<&str> = None;
    for segment in path.trim_start_matches('/').split('/') {
        if segment.is_empty() {
            continue;
        }
        match segment.parse::<Uuid>() {
            Ok(id) => {
                let name = collection.unwrap_or_default();
                // Fail closed: an id under an unrecognised collection must not
                // pass unchecked.
                let scope = scope_for(name).ok_or(name)?;
                found.push((scope, id));
            }
            // `versions`, `restore`, a version number, and so on.
            Err(_) => collection = Some(segment),
        }
    }
    Ok(found)
}

/// Rejects a request whose path names an entity in another tenant.
///
/// Routes that carry no id — `/health`, `/settings/ai`, `/projects` and
/// `POST /ai/proposals` — pass through. The last one identifies its feature in
/// the body, so `handlers::ai` performs the same check itself.
pub async fn guard(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let identity = request
        .extensions()
        .get::<Identity>()
        .cloned()
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "the tenant guard ran before authentication",
            )
        })?;

    let path = request.uri().path().to_owned();
    let relative = path.strip_prefix("/api/v1").unwrap_or(&path);
    let ids = scoped_ids(relative).map_err(|name| {
        tracing::error!(%path, collection = %name, "no tenant scope for this route");
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "this route has no tenant scope",
        )
    })?;

    for (scope, id) in ids {
        let owner = state.store.tenant_of(scope, id).await?;
        if owner != identity.tenant {
            tracing::warn!(
                tenant = %identity.tenant,
                entity = scope.entity(),
                "rejected a cross-tenant request"
            );
            return Err(ApiError::new(
                StatusCode::NOT_FOUND,
                format!("{} not found", scope.entity()),
            ));
        }
    }
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(path: &str) -> Vec<TenantScope> {
        scoped_ids(path)
            .expect("path should resolve")
            .into_iter()
            .map(|(s, _)| s)
            .collect()
    }

    #[test]
    fn resolves_the_id_of_the_route_it_belongs_to() {
        let id = Uuid::new_v4();
        assert_eq!(kinds(&format!("/models/{id}")), [TenantScope::Model]);
        // The id belongs to the model, not to the nested collection.
        assert_eq!(kinds(&format!("/models/{id}/states")), [TenantScope::Model]);
        assert_eq!(
            kinds(&format!("/features/{id}/test-cases")),
            [TenantScope::Feature]
        );
    }

    #[test]
    fn resolves_both_ids_of_a_nested_route() {
        let state = Uuid::new_v4();
        let tc = Uuid::new_v4();
        assert_eq!(
            kinds(&format!("/states/{state}/test-cases/{tc}")),
            [TenantScope::State, TenantScope::TestCase]
        );
    }

    #[test]
    fn ignores_path_segments_that_are_not_ids() {
        let id = Uuid::new_v4();
        assert_eq!(
            kinds(&format!("/models/{id}/versions/3/restore")),
            [TenantScope::Model]
        );
    }

    #[test]
    fn routes_without_ids_need_no_lookup() {
        assert!(kinds("/health").is_empty());
        assert!(kinds("/projects").is_empty());
        assert!(kinds("/settings/ai").is_empty());
        assert!(kinds("/ai/proposals").is_empty());
    }

    #[test]
    fn an_id_under_an_unknown_collection_fails_closed() {
        // NFR-007: a route added without a scope must break loudly, not open.
        let id = Uuid::new_v4();
        assert!(scoped_ids(&format!("/widgets/{id}")).is_err());
    }
}
