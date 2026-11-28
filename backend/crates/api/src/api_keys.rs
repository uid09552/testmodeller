//! Per-user API keys: generation, hashing and authentication (FR-090).
//!
//! A key is `tm_` followed by 64 hex characters drawn from the OS random
//! source. Only its SHA-256 hash is stored: the key is random, so a slow
//! password hash would add nothing. A key is accepted by `/mcp` only, except an
//! upload key, which is accepted only by `POST /projects/{its project}/test-results`
//! (ADR 0018). Keys are never logged. See docs/adr/0015-api-keys-and-mcp.md.

use axum::extract::{Request, State};
use axum::http::{header, Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use sha2::{Digest, Sha256};
use tm_storage::Store;
use uuid::Uuid;

use crate::auth::{Identity, Role};
use crate::error::ApiError;

/// Every key starts with this, so a leaked one is recognisable in a scan.
pub const KEY_PREFIX: &str = "tm_";
/// Characters of the key shown in lists (`tm_` + 7 hex).
const DISPLAY_LEN: usize = 10;

/// A fresh key: `tm_` + 64 hex characters.
///
/// Two v4 UUIDs supply the randomness (both from the OS source); their version
/// and variant bits leave 244 random bits, well beyond guessing.
pub fn generate() -> String {
    // 256 bits from the OS (08-usermanagement.md, ADR 0015).
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the OS random number generator failed");
    let mut key = String::with_capacity(KEY_PREFIX.len() + 64);
    key.push_str(KEY_PREFIX);
    for byte in bytes {
        key.push_str(&format!("{byte:02x}"));
    }
    key
}

/// SHA-256 of a key, as stored.
pub fn hash(key: &str) -> [u8; 32] {
    Sha256::digest(key.as_bytes()).into()
}

/// The part of a key shown in lists.
pub fn display_prefix(key: &str) -> String {
    key.chars().take(DISPLAY_LEN).collect()
}

/// Whether an `Authorization` value looks like an API key rather than a JWT.
pub fn bearer_key(header_value: &str) -> Option<&str> {
    header_value
        .strip_prefix("Bearer ")
        .or_else(|| header_value.strip_prefix("bearer "))
        .map(str::trim)
        .filter(|k| k.starts_with(KEY_PREFIX))
}

fn unauthorized(detail: &str) -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, detail)
}

/// An authenticated key: who it acts as, and the project it is limited to.
#[derive(Debug, Clone)]
pub struct KeyIdentity {
    /// The identity the key acts as.
    pub identity: Identity,
    /// Set for an upload key (ADR 0018).
    pub project_id: Option<Uuid>,
}

/// The identity an `Authorization: Bearer tm_…` header acts as.
pub async fn authenticate(store: &Store, header_value: &str) -> Result<KeyIdentity, ApiError> {
    let key = bearer_key(header_value)
        .ok_or_else(|| unauthorized("expected \"Authorization: Bearer tm_…\" with an API key"))?;
    // A well-formed key has a fixed shape; anything else is not worth a lookup.
    let well_formed = key.len() == KEY_PREFIX.len() + 64
        && key[KEY_PREFIX.len()..]
            .bytes()
            .all(|b| b.is_ascii_hexdigit());
    if !well_formed {
        return Err(unauthorized("the API key is malformed"));
    }
    let record = store
        .use_api_key(&hash(key))
        .await?
        .ok_or_else(|| unauthorized("the API key is unknown, expired or revoked"))?;
    Ok(KeyIdentity {
        identity: Identity {
            tenant: record.tenant,
            // An upload key writes on its one route, which needs the Editor role.
            role: if record.role == Role::Editor.as_str() || record.project_id.is_some() {
                Role::Editor
            } else {
                Role::User
            },
            subject: Some(record.subject),
        },
        project_id: record.project_id,
    })
}

/// The identity of a request authenticated by an upload key. It travels as a request
/// extension, which cannot come from the wire, and is read by `auth::authenticate`
/// in place of a JWT.
#[derive(Debug, Clone)]
pub struct UploadIdentity(pub Identity);

/// The project of `POST /projects/{id}/test-results`, if `path` is that route.
fn upload_route_project(path: &str) -> Option<Uuid> {
    let relative = path.strip_prefix("/api/v1").unwrap_or(path);
    let mut parts = relative.trim_start_matches('/').split('/');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some("projects"), Some(id), Some("test-results"), None) => id.parse().ok(),
        _ => None,
    }
}

/// Lets an upload key through to `POST /projects/{its project}/test-results`, in front of
/// the JWT layer. Any other request passes unchanged and is judged by that layer, which
/// refuses a `tm_` bearer as "not a JWT"; so an API key never reaches another route.
pub async fn upload_layer(
    State(store): State<Store>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let key_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .filter(|v| bearer_key(v).is_some())
        .map(str::to_owned);
    let (Some(value), Some(project)) = (key_header, upload_route_project(request.uri().path()))
    else {
        return Ok(next.run(request).await);
    };
    if request.method() != Method::POST {
        return Ok(next.run(request).await);
    }
    let key = authenticate(&store, &value).await?;
    if key.project_id != Some(project) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "this API key may only upload test results of the project it was created for",
        ));
    }
    request
        .extensions_mut()
        .insert(UploadIdentity(key.identity));
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_prefixed_hex_and_unique() {
        let a = generate();
        let b = generate();
        assert_ne!(a, b);
        assert_eq!(a.len(), 67);
        assert!(a.starts_with("tm_"));
        assert!(a[3..].bytes().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(display_prefix(&a).len(), 10);
    }

    #[test]
    fn hash_is_stable_and_differs_per_key() {
        assert_eq!(hash("tm_x"), hash("tm_x"));
        assert_ne!(hash("tm_x"), hash("tm_y"));
    }

    #[test]
    fn the_upload_route_is_recognised_exactly() {
        let id = Uuid::new_v4();
        assert_eq!(
            upload_route_project(&format!("/projects/{id}/test-results")),
            Some(id)
        );
        assert_eq!(
            upload_route_project(&format!("/api/v1/projects/{id}/test-results")),
            Some(id)
        );
        for other in [
            format!("/projects/{id}"),
            format!("/projects/{id}/test-results/x"),
            format!("/projects/{id}/export"),
            "/projects/not-a-uuid/test-results".to_owned(),
            format!("/other/{id}/test-results"),
        ] {
            assert_eq!(upload_route_project(&other), None, "{other}");
        }
    }

    #[test]
    fn only_tm_bearers_are_keys() {
        assert_eq!(bearer_key("Bearer tm_abc"), Some("tm_abc"));
        assert_eq!(bearer_key("Bearer eyJhbGciOi"), None);
        assert_eq!(bearer_key("tm_abc"), None);
    }
}
