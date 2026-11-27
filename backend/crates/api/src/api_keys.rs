//! Per-user API keys: generation, hashing and authentication (FR-090).
//!
//! A key is `tm_` followed by 64 hex characters drawn from the OS random
//! source. Only its SHA-256 hash is stored: the key is random, so a slow
//! password hash would add nothing. Keys are accepted by `/mcp` only and are
//! never logged. See docs/adr/0015-api-keys-and-mcp.md.

use axum::http::StatusCode;
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
    let mut key = String::with_capacity(KEY_PREFIX.len() + 64);
    key.push_str(KEY_PREFIX);
    for byte in Uuid::new_v4()
        .as_bytes()
        .iter()
        .chain(Uuid::new_v4().as_bytes())
    {
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

/// The identity an `Authorization: Bearer tm_…` header acts as.
pub async fn authenticate(store: &Store, header_value: &str) -> Result<Identity, ApiError> {
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
    Ok(Identity {
        tenant: record.tenant,
        role: if record.role == Role::Editor.as_str() {
            Role::Editor
        } else {
            Role::User
        },
        subject: Some(record.subject),
    })
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
    fn only_tm_bearers_are_keys() {
        assert_eq!(bearer_key("Bearer tm_abc"), Some("tm_abc"));
        assert_eq!(bearer_key("Bearer eyJhbGciOi"), None);
        assert_eq!(bearer_key("tm_abc"), None);
    }
}
