//! Model Context Protocol server for TestModeller (FR-091).
//!
//! This crate knows MCP — JSON-RPC framing, `initialize`, `tools/list`,
//! `tools/call` — and the tool catalogue, but nothing about HTTP or storage.
//! Every tool is an aggregate of REST operations it runs through a
//! [`Backend`]; the API crate implements that by dispatching in-process
//! through its own router, so tenancy, roles and validation are the API's.
//! See docs/specification/05-api.md#mcp-tools and docs/adr/0015-api-keys-and-mcp.md.

mod protocol;
mod tools;

pub use protocol::{
    handle, handle_http, Reply, RequestHeaders, PROTOCOL_VERSION, STATELESS_VERSION,
    SUPPORTED_VERSIONS,
};
pub use tools::catalogue;

use serde_json::Value;

/// HTTP method of a REST call a tool makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// `GET`
    Get,
    /// `POST`
    Post,
    /// `PUT`
    Put,
}

/// Body of a REST call.
#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    /// No body.
    None,
    /// A JSON document.
    Json(Value),
    /// Raw text with its content type (result files).
    Text {
        /// `Content-Type`.
        content_type: &'static str,
        /// The text.
        text: String,
    },
}

/// One REST call, relative to `/api/v1`.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiRequest {
    /// Method.
    pub method: Method,
    /// Path such as `/models/{id}`; ids in it are always validated UUIDs.
    pub path: String,
    /// Query parameters, unencoded.
    pub query: Vec<(String, String)>,
    /// Body.
    pub body: Body,
    /// `If-Match` version, for replacements.
    pub if_match: Option<i64>,
}

impl ApiRequest {
    /// A call without query, body or version.
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            query: Vec::new(),
            body: Body::None,
            if_match: None,
        }
    }

    /// Adds a query parameter.
    pub fn query(mut self, key: &str, value: impl Into<String>) -> Self {
        self.query.push((key.to_owned(), value.into()));
        self
    }

    /// Sets a JSON body.
    pub fn json(mut self, body: Value) -> Self {
        self.body = Body::Json(body);
        self
    }
}

/// What a REST call answered.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiResponse {
    /// HTTP status.
    pub status: u16,
    /// Parsed JSON body, or the text as a JSON string when it was not JSON.
    pub body: Value,
}

impl ApiResponse {
    /// Whether the status is 2xx.
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// Runs REST calls on behalf of the authenticated caller.
#[async_trait::async_trait]
pub trait Backend: Send + Sync {
    /// Executes one call as the caller.
    async fn call(&self, request: ApiRequest) -> ApiResponse;
}
