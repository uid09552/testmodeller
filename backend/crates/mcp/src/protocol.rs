//! JSON-RPC 2.0 framing and the MCP lifecycle methods.
//!
//! Streamable HTTP with JSON responses: one request in, one response out, no
//! server-initiated messages. Batches were removed from MCP in 2025-06-18 and
//! are rejected.

use serde_json::{json, Map, Value};

use crate::tools::{self, ToolOutcome};
use crate::Backend;

/// The protocol version this server prefers for the `initialize` handshake.
pub const PROTOCOL_VERSION: &str = "2025-06-18";
/// Handshake versions a client may negotiate; the newest is preferred.
pub const SUPPORTED_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
/// The stateless revision: no `initialize`, no sessions, version and capabilities in every
/// request (ADR 0024).
pub const STATELESS_VERSION: &str = "2026-07-28";

const META_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
const META_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";
const META_SERVER_INFO: &str = "io.modelcontextprotocol/serverInfo";
const HEADER_MISMATCH: i64 = -32020;
const UNSUPPORTED_VERSION: i64 = -32022;
/// How long a client may reuse `server/discover` and `tools/list` results.
const TTL_MS: i64 = 60_000;

/// The MCP request headers of the HTTP transport (2026-07-28).
#[derive(Debug, Clone, Default)]
pub struct RequestHeaders {
    /// `MCP-Protocol-Version`.
    pub protocol_version: Option<String>,
    /// `Mcp-Method`.
    pub method: Option<String>,
    /// `Mcp-Name`.
    pub name: Option<String>,
}

/// What the transport sends back: an HTTP status and, unless it is `202`, a JSON body.
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    /// HTTP status.
    pub status: u16,
    /// JSON-RPC response; `None` for an accepted notification.
    pub body: Option<Value>,
}

impl Reply {
    fn ok(body: Value) -> Self {
        Self {
            status: 200,
            body: Some(body),
        }
    }
    fn failed(status: u16, body: Value) -> Self {
        Self {
            status,
            body: Some(body),
        }
    }
}

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

const INSTRUCTIONS: &str = "TestModeller: model-based testing. Data is organised as \
Project > Component > Feature > Model (a state machine). Start with tm_list_projects to find ids. \
Test cases belong to a feature and are generated from a model's states and transitions. \
AI proposals are never accepted automatically: tm_request_ai_proposals creates pending proposals, \
and only tm_decide_proposal, after the user agrees, accepts or rejects one.";

fn error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message.into() } })
}

fn result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// Handles one JSON-RPC message. Returns `None` for a notification (or a
/// client's response), which the transport answers with `202`.
pub async fn handle(backend: &dyn Backend, raw: &[u8]) -> Option<Value> {
    handle_http(backend, &RequestHeaders::default(), raw)
        .await
        .body
}

/// Like [`handle`], for the HTTP transport: reads the MCP headers and answers with a status.
///
/// A request that names the stateless revision (in `_meta` or `MCP-Protocol-Version`) is
/// checked and answered as 2026-07-28 requires; any other request keeps the handshake
/// behaviour of the earlier revisions.
pub async fn handle_http(backend: &dyn Backend, headers: &RequestHeaders, raw: &[u8]) -> Reply {
    let message: Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(e) => {
            return Reply::failed(
                400,
                error(Value::Null, PARSE_ERROR, format!("invalid JSON: {e}")),
            )
        }
    };
    let Value::Object(message) = message else {
        return Reply::failed(
            400,
            error(
                Value::Null,
                INVALID_REQUEST,
                "expected a single JSON-RPC object; batches are not supported",
            ),
        );
    };
    let id_of = |m: &Map<String, Value>| m.get("id").cloned().unwrap_or(Value::Null);
    if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Reply::failed(
            400,
            error(
                id_of(&message),
                INVALID_REQUEST,
                "\"jsonrpc\" must be \"2.0\"",
            ),
        );
    }
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        // A response to a server request; this server sends none, so ignore it.
        return Reply {
            status: 202,
            body: None,
        };
    };
    let params = match message.get("params") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(p)) => p.clone(),
        Some(_) => {
            return Reply::failed(
                400,
                error(
                    id_of(&message),
                    INVALID_PARAMS,
                    "\"params\" must be an object",
                ),
            )
        }
    };
    let meta = params.get("_meta").and_then(Value::as_object);
    let version = meta
        .and_then(|m| m.get(META_VERSION))
        .and_then(Value::as_str)
        .or(headers.protocol_version.as_deref());

    match version {
        Some(STATELESS_VERSION) => {
            stateless(
                backend,
                headers,
                method,
                message.get("id").cloned(),
                &params,
            )
            .await
        }
        Some(v) if !SUPPORTED_VERSIONS.contains(&v) => Reply::failed(
            400,
            json!({ "jsonrpc": "2.0", "id": id_of(&message), "error": {
                "code": UNSUPPORTED_VERSION,
                "message": format!("protocol version \"{v}\" is not supported"),
                "data": { "supported": all_versions(), "requested": v },
            } }),
        ),
        _ if method == "server/discover" => match message.get("id").cloned() {
            Some(id) => Reply::ok(result(id, discover())),
            None => Reply {
                status: 202,
                body: None,
            },
        },
        _ => {
            // The handshake revisions. Without an id it is a notification.
            let Some(id) = message.get("id").cloned() else {
                return Reply {
                    status: 202,
                    body: None,
                };
            };
            Reply::ok(match method {
                "initialize" => result(id, initialize(&params)),
                "ping" => result(id, json!({})),
                "tools/list" => result(id, json!({ "tools": tools::catalogue() })),
                "tools/call" => call_tool(backend, id, &params).await,
                other => error(
                    id,
                    METHOD_NOT_FOUND,
                    format!("method \"{other}\" is not supported"),
                ),
            })
        }
    }
}

/// Every version a client may use: the stateless revision and the handshake ones.
fn all_versions() -> Vec<&'static str> {
    std::iter::once(STATELESS_VERSION)
        .chain(SUPPORTED_VERSIONS)
        .collect()
}

fn server_info() -> Value {
    json!({ "name": "testmodeller", "title": "TestModeller", "version": env!("CARGO_PKG_VERSION") })
}

/// The `server/discover` result (2026-07-28).
fn discover() -> Value {
    json!({
        "resultType": "complete",
        "supportedVersions": all_versions(),
        "capabilities": { "tools": { "listChanged": false } },
        "instructions": INSTRUCTIONS,
        "ttlMs": TTL_MS,
        "cacheScope": "private",
        "_meta": { META_SERVER_INFO: server_info() },
    })
}

/// A request of the stateless revision: checks the headers and `_meta`, answers with
/// `resultType` and the HTTP status the revision prescribes.
async fn stateless(
    backend: &dyn Backend,
    headers: &RequestHeaders,
    method: &str,
    id: Option<Value>,
    params: &Map<String, Value>,
) -> Reply {
    let id_value = id.clone().unwrap_or(Value::Null);
    let fail = |status: u16, code: i64, message: &str| {
        Reply::failed(status, error(id_value.clone(), code, message))
    };
    if headers.protocol_version.as_deref() != Some(STATELESS_VERSION) {
        return fail(
            400,
            HEADER_MISMATCH,
            "the MCP-Protocol-Version header must be present and match the version in _meta",
        );
    }
    let has_capabilities = params
        .get("_meta")
        .and_then(Value::as_object)
        .is_some_and(|m| m.get(META_CAPABILITIES).is_some_and(Value::is_object));
    if !has_capabilities {
        return fail(
            400,
            INVALID_PARAMS,
            "_meta must carry io.modelcontextprotocol/protocolVersion and io.modelcontextprotocol/clientCapabilities",
        );
    }
    if headers.method.as_deref() != Some(method) {
        return fail(
            400,
            HEADER_MISMATCH,
            "the Mcp-Method header must equal the request method",
        );
    }
    if method == "tools/call"
        && headers.name.as_deref() != params.get("name").and_then(Value::as_str)
    {
        return fail(
            400,
            HEADER_MISMATCH,
            "the Mcp-Name header must equal params.name",
        );
    }
    let Some(id) = id else {
        return Reply {
            status: 202,
            body: None,
        };
    };
    let mut reply = match method {
        "server/discover" => result(id, discover()),
        "tools/list" => result(
            id,
            json!({ "tools": tools::catalogue(), "ttlMs": TTL_MS, "cacheScope": "private" }),
        ),
        "tools/call" => call_tool(backend, id, params).await,
        other => {
            return Reply::failed(
                404,
                error(
                    id,
                    METHOD_NOT_FOUND,
                    format!("method \"{other}\" is not supported"),
                ),
            )
        }
    };
    if let Some(r) = reply.get_mut("result").and_then(Value::as_object_mut) {
        r.entry("resultType").or_insert(json!("complete"));
        r.entry("_meta")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .map(|m| m.insert(META_SERVER_INFO.into(), server_info()));
    }
    let status = if reply.get("error").is_some() {
        400
    } else {
        200
    };
    Reply {
        status,
        body: Some(reply),
    }
}

fn initialize(params: &Map<String, Value>) -> Value {
    let requested = params.get("protocolVersion").and_then(Value::as_str);
    let version = requested
        .filter(|v| SUPPORTED_VERSIONS.contains(v))
        .unwrap_or(PROTOCOL_VERSION);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": server_info(),
        "instructions": INSTRUCTIONS,
    })
}

async fn call_tool(backend: &dyn Backend, id: Value, params: &Map<String, Value>) -> Value {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return error(id, INVALID_PARAMS, "\"name\" is required");
    };
    let args = match params.get("arguments") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(a)) => a.clone(),
        Some(_) => return error(id, INVALID_PARAMS, "\"arguments\" must be an object"),
    };
    match tools::run(backend, name, &args).await {
        // Unknown tool is a protocol error; everything else is a tool result.
        ToolOutcome::Unknown => error(id, INVALID_PARAMS, format!("unknown tool \"{name}\"")),
        ToolOutcome::Done(value) => result(
            id,
            json!({
                "content": [{ "type": "text", "text": pretty(&value) }],
                "structuredContent": wrap(value),
                "isError": false,
            }),
        ),
        ToolOutcome::Failed(message) => result(
            id,
            json!({ "content": [{ "type": "text", "text": message }], "isError": true }),
        ),
    }
}

/// `structuredContent` must be an object.
fn wrap(value: Value) -> Value {
    match value {
        Value::Object(_) => value,
        other => json!({ "result": other }),
    }
}

fn pretty(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    }
}
