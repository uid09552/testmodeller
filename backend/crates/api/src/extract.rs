//! Request extractors mapping rejections to problem responses.

use axum::extract::{FromRequest, FromRequestParts};
use axum::http::request::Parts;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use tm_storage::{Cursor, PageRequest};

use crate::error::ApiError;

/// JSON body.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub struct ApiJson<T>(pub T);

/// Path parameters.
#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(ApiError))]
pub struct ApiPath<T>(pub T);

/// Query parameters.
#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(ApiError))]
pub struct ApiQuery<T>(pub T);

/// Parses an optional JSON body; an empty body yields `None`.
pub fn optional_json<T: DeserializeOwned>(body: &[u8]) -> Result<Option<T>, ApiError> {
    if body.iter().all(u8::is_ascii_whitespace) {
        return Ok(None);
    }
    serde_json::from_slice(body)
        .map(Some)
        .map_err(|e| ApiError::bad_request(format!("invalid JSON body: {e}")))
}

/// Required `If-Match` header carrying the entity version (NFR-006).
pub struct IfMatch(pub i32);

impl<S: Send + Sync> FromRequestParts<S> for IfMatch {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let raw = parts
            .headers
            .get(axum::http::header::IF_MATCH)
            .ok_or_else(|| {
                ApiError::bad_request("If-Match header with the entity version is required")
            })?
            .to_str()
            .map_err(|_| ApiError::bad_request("invalid If-Match header"))?;
        let value = raw.trim().trim_start_matches("W/").trim_matches('"');
        value
            .parse::<i32>()
            .ok()
            .filter(|v| *v >= 1)
            .map(IfMatch)
            .ok_or_else(|| ApiError::bad_request("If-Match must be a positive integer version"))
    }
}

/// `limit` and `cursor` query parameters.
#[derive(Debug, Deserialize)]
pub struct Paging {
    limit: Option<i64>,
    cursor: Option<String>,
}

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

impl Paging {
    /// Creates paging parameters from separately parsed query fields.
    pub fn new(limit: Option<i64>, cursor: Option<String>) -> Self {
        Self { limit, cursor }
    }

    /// Validates and converts to a storage page request.
    pub fn to_request(&self) -> Result<PageRequest, ApiError> {
        let limit = self.limit.unwrap_or(DEFAULT_LIMIT);
        if !(1..=MAX_LIMIT).contains(&limit) {
            return Err(ApiError::bad_request(format!(
                "limit must be between 1 and {MAX_LIMIT}"
            )));
        }
        let after = match self.cursor.as_deref().filter(|c| !c.is_empty()) {
            Some(c) => {
                Some(Cursor::decode(c).ok_or_else(|| ApiError::bad_request("invalid cursor"))?)
            }
            None => None,
        };
        Ok(PageRequest { limit, after })
    }
}
