//! RFC 7807 problem responses.

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tm_storage::StorageError;

/// Field-level error detail.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FieldError {
    /// Offending field or element.
    pub field: String,
    /// Explanation.
    pub message: String,
}

/// Error returned by handlers, rendered as `application/problem+json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    /// HTTP status.
    pub status: StatusCode,
    /// Human-readable detail.
    pub detail: String,
    /// Field-level errors.
    pub errors: Vec<FieldError>,
}

/// Problem document (RFC 7807).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Problem {
    /// Problem type URI.
    #[serde(rename = "type")]
    pub problem_type: String,
    /// Short summary.
    pub title: String,
    /// HTTP status.
    pub status: u16,
    /// Detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Field errors.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<FieldError>,
}

impl ApiError {
    /// Creates an error with a status and detail.
    pub fn new(status: StatusCode, detail: impl Into<String>) -> Self {
        Self {
            status,
            detail: detail.into(),
            errors: Vec::new(),
        }
    }

    /// 400 Bad Request.
    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, detail)
    }

    /// 422 Unprocessable Entity.
    pub fn unprocessable(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, detail)
    }

    /// Adds field errors.
    pub fn with_errors(mut self, errors: Vec<FieldError>) -> Self {
        self.errors = errors;
        self
    }

    /// Problem document for this error.
    pub fn problem(&self) -> Problem {
        Problem {
            problem_type: "about:blank".into(),
            title: self.status.canonical_reason().unwrap_or("Error").to_owned(),
            status: self.status.as_u16(),
            detail: Some(self.detail.clone()),
            errors: self.errors.clone(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = serde_json::to_vec(&self.problem()).unwrap_or_default();
        (
            self.status,
            [(header::CONTENT_TYPE, "application/problem+json")],
            body,
        )
            .into_response()
    }
}

impl From<StorageError> for ApiError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::NotFound(what) => {
                Self::new(StatusCode::NOT_FOUND, format!("{what} not found"))
            }
            StorageError::VersionMismatch => Self::new(
                StatusCode::PRECONDITION_FAILED,
                "version mismatch: reload and retry",
            ),
            StorageError::Conflict(msg) => Self::new(StatusCode::CONFLICT, msg),
            StorageError::Invalid(msg) => Self::unprocessable(msg),
            StorageError::Database(_) | StorageError::Serialization(_) => {
                tracing::error!(error = %e, "storage failure");
                Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
            }
        }
    }
}

impl From<JsonRejection> for ApiError {
    fn from(r: JsonRejection) -> Self {
        Self::bad_request(r.body_text())
    }
}

impl From<PathRejection> for ApiError {
    fn from(r: PathRejection) -> Self {
        Self::bad_request(r.body_text())
    }
}

impl From<QueryRejection> for ApiError {
    fn from(r: QueryRejection) -> Self {
        Self::bad_request(r.body_text())
    }
}

/// Handler result alias.
pub type ApiResult<T> = Result<T, ApiError>;
