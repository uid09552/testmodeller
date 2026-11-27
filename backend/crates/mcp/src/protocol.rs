//! JSON-RPC 2.0 framing and the MCP lifecycle methods.
//!
//! Streamable HTTP with JSON responses: one request in, one response out, no
//! server-initiated messages. Batches were removed from MCP in 2025-06-18 and
//! are rejected.

use serde_json::{json, Map, Value};

use crate::tools::{self, ToolOutcome};
use crate::Backend;

/// The protocol version this server prefers.
pub const PROTOCOL_VERSION: &str = "2025-06-18";
/// Versions a client may negotiate; the newest is preferred.
pub const SUPPORTED_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

const INSTRUCTIONS: &str = "TestModeller: model-based testing. Data is organised as \
Project > Component > Feature > Model (a state machine). Start with list_projects to find ids. \
Test cases belong to a feature and are generated from a model's states and transitions. \
AI proposals are never accepted automatically: request_ai_proposals creates pending proposals, \
and only decide_proposal, after the user agrees, accepts or rejects one.";

fn error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message.into() } })
}

fn result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// Handles one JSON-RPC message. Returns `None` for a notification (or a
/// client's response), which the transport answers with `202`.
pub async fn handle(backend: &dyn Backend, raw: &[u8]) -> Option<Value> {
    let message: Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(e) => {
            return Some(error(
                Value::Null,
                PARSE_ERROR,
                format!("invalid JSON: {e}"),
            ))
        }
    };
    let Value::Object(message) = message else {
        return Some(error(
            Value::Null,
            INVALID_REQUEST,
            "expected a single JSON-RPC object; batches are not supported",
        ));
    };
    if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Some(error(
            message.get("id").cloned().unwrap_or(Value::Null),
            INVALID_REQUEST,
            "\"jsonrpc\" must be \"2.0\"",
        ));
    }
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        // A response to a server request; this server sends none, so ignore it.
        return None;
    };
    // Without an id it is a notification (e.g. notifications/initialized).
    let id = message.get("id").cloned()?;
    let params = match message.get("params") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(p)) => p.clone(),
        Some(_) => return Some(error(id, INVALID_PARAMS, "\"params\" must be an object")),
    };

    Some(match method {
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

fn initialize(params: &Map<String, Value>) -> Value {
    let requested = params.get("protocolVersion").and_then(Value::as_str);
    let version = requested
        .filter(|v| SUPPORTED_VERSIONS.contains(v))
        .unwrap_or(PROTOCOL_VERSION);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "testmodeller", "title": "TestModeller", "version": env!("CARGO_PKG_VERSION") },
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
