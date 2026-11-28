//! `POST /api/v1/mcp`: the MCP transport (FR-091).
//!
//! Authenticates an API key, then hands the JSON-RPC message to `tm-mcp`,
//! whose tools call back into [`Dispatcher`]. The dispatcher runs each REST
//! call through the `/api/v1` router built with [`AuthMode::Internal`], as the
//! key's identity, so the tenant guard, the role rule and validation are the
//! API's own. See docs/adr/0015-api-keys-and-mcp.md.

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use http_body_util::BodyExt;
use serde_json::Value;
use tm_mcp::{ApiRequest, ApiResponse, Backend};
use tm_storage::Store;
use tower::ServiceExt;

use crate::api_keys;
use crate::auth::Identity;
use crate::error::ApiError;

/// Largest REST response a tool reads back (exports can be large).
const MAX_RESPONSE: usize = 16 * 1024 * 1024;

/// State of the MCP route.
#[derive(Clone)]
pub struct McpState {
    /// For API key lookups.
    pub store: Store,
    /// `--dev-mode`: a request without `Authorization` runs as the dev identity.
    pub dev: bool,
    /// The `/api/v1` router with internal authentication.
    pub internal: Router,
    /// Protected resource metadata, when an authorization server is configured.
    pub oauth: Option<std::sync::Arc<OAuthResource>>,
}

/// What `/mcp` tells clients about OAuth (RFC 9728). Metadata only: `/mcp` still accepts
/// API keys, not the tokens of this authorization server (ADR 0024).
#[derive(Debug, Clone)]
pub struct OAuthResource {
    /// Public base URL of the deployment, without a trailing slash.
    pub public_url: String,
    /// Issuer URL of the authorization server.
    pub authorization_server: String,
}

impl OAuthResource {
    /// The `resource` identifier: the MCP endpoint's URL.
    pub fn resource(&self) -> String {
        format!("{}/api/v1/mcp", self.public_url)
    }

    fn metadata_url(&self) -> String {
        format!(
            "{}/.well-known/oauth-protected-resource/api/v1/mcp",
            self.public_url
        )
    }
}

/// `GET /.well-known/oauth-protected-resource[/api/v1/mcp]` (RFC 9728). Not found unless an
/// authorization server is configured.
pub async fn protected_resource(State(state): State<McpState>) -> Response {
    match &state.oauth {
        Some(o) => Json(serde_json::json!({
            "resource": o.resource(),
            "authorization_servers": [o.authorization_server],
            "bearer_methods_supported": ["header"],
        }))
        .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// `POST /mcp`
pub async fn post(State(state): State<McpState>, headers: HeaderMap, body: Bytes) -> Response {
    let oauth = state.oauth.clone();
    let mut response = post_inner(state, headers, body).await;
    // A 401 points clients at the protected resource metadata (RFC 9728 section 5.1).
    if let (StatusCode::UNAUTHORIZED, Some(o)) = (response.status(), oauth) {
        if let Ok(v) = format!("Bearer resource_metadata=\"{}\"", o.metadata_url()).parse() {
            response.headers_mut().insert(header::WWW_AUTHENTICATE, v);
        }
    }
    response
}

async fn post_inner(state: McpState, headers: HeaderMap, body: Bytes) -> Response {
    if let Err(e) = check_origin(&headers) {
        return e.into_response();
    }
    let identity = match headers
        .get(header::AUTHORIZATION)
        .map(|v| v.to_str().unwrap_or_default())
    {
        Some(value) => match api_keys::authenticate(&state.store, value).await {
            Ok(key) if key.project_id.is_some() => {
                return ApiError::new(
                    StatusCode::UNAUTHORIZED,
                    "an upload key cannot use the MCP endpoint; create an MCP key in Settings",
                )
                .into_response()
            }
            Ok(key) => key.identity,
            Err(e) => return e.into_response(),
        },
        None if state.dev => Identity::dev(),
        None => {
            return ApiError::new(
                StatusCode::UNAUTHORIZED,
                "expected \"Authorization: Bearer tm_…\" with an API key from Settings",
            )
            .into_response()
        }
    };
    let dispatcher = Dispatcher {
        router: state.internal,
        identity,
    };
    let text = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    let mcp_headers = tm_mcp::RequestHeaders {
        protocol_version: text("mcp-protocol-version"),
        method: text("mcp-method"),
        name: text("mcp-name"),
    };
    let reply = tm_mcp::handle_http(&dispatcher, &mcp_headers, &body).await;
    let status = StatusCode::from_u16(reply.status).unwrap_or(StatusCode::OK);
    match reply.body {
        Some(body) => (status, Json(body)).into_response(),
        None => status.into_response(),
    }
}

/// `GET /mcp`: this server sends no stream.
pub async fn get() -> Response {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [(header::ALLOW, "POST")],
        "this MCP server answers POST only; it sends no event stream",
    )
        .into_response()
}

/// Rejects a browser request from another origin (MCP requires an Origin
/// check against DNS rebinding). Clients such as VS Code send no `Origin`.
fn check_origin(headers: &HeaderMap) -> Result<(), ApiError> {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|o| o.to_str().ok()) else {
        return Ok(());
    };
    let origin_host = origin
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(origin);
    let host = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    if !host.is_empty() && origin_host.eq_ignore_ascii_case(host) {
        return Ok(());
    }
    Err(ApiError::new(
        StatusCode::FORBIDDEN,
        "cross-origin requests to the MCP endpoint are not allowed",
    ))
}

/// Runs a tool's REST calls in-process as the key's identity.
pub struct Dispatcher {
    router: Router,
    identity: Identity,
}

#[async_trait::async_trait]
impl Backend for Dispatcher {
    async fn call(&self, request: ApiRequest) -> ApiResponse {
        match self.dispatch(request).await {
            Ok(response) => response,
            Err(detail) => ApiResponse {
                status: 500,
                body: serde_json::json!({ "detail": detail }),
            },
        }
    }
}

impl Dispatcher {
    async fn dispatch(&self, request: ApiRequest) -> Result<ApiResponse, String> {
        let mut uri = format!("/api/v1{}", request.path);
        if !request.query.is_empty() {
            let query = serde_urlencoded::to_string(&request.query).map_err(|e| e.to_string())?;
            uri.push('?');
            uri.push_str(&query);
        }
        let method = match request.method {
            tm_mcp::Method::Get => axum::http::Method::GET,
            tm_mcp::Method::Post => axum::http::Method::POST,
            tm_mcp::Method::Put => axum::http::Method::PUT,
        };
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(version) = request.if_match {
            builder = builder.header(header::IF_MATCH, version.to_string());
        }
        let body = match request.body {
            tm_mcp::Body::None => Body::empty(),
            tm_mcp::Body::Json(value) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(value.to_string())
            }
            tm_mcp::Body::Text { content_type, text } => {
                builder = builder.header(header::CONTENT_TYPE, content_type);
                Body::from(text)
            }
        };
        let mut http = builder.body(body).map_err(|e| e.to_string())?;
        http.extensions_mut().insert(self.identity.clone());

        let response = self
            .router
            .clone()
            .oneshot(http)
            .await
            .map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        let bytes = http_body_util::Limited::new(response.into_body(), MAX_RESPONSE)
            .collect()
            .await
            .map_err(|_| "the response was too large to return to the client".to_owned())?
            .to_bytes();
        let body = serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
        Ok(ApiResponse { status, body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(origin: Option<&str>, host: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::HOST, HeaderValue::from_str(host).unwrap());
        if let Some(o) = origin {
            h.insert(header::ORIGIN, HeaderValue::from_str(o).unwrap());
        }
        h
    }

    #[test]
    fn origin_must_be_absent_or_same_host() {
        assert!(check_origin(&headers(None, "tm.example.com")).is_ok());
        assert!(check_origin(&headers(Some("https://tm.example.com"), "tm.example.com")).is_ok());
        assert!(check_origin(&headers(Some("https://evil.example"), "tm.example.com")).is_err());
    }
}
