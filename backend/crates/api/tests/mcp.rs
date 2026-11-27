//! API keys and the MCP endpoint end to end (FR-090, FR-091). Requires Docker.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tm_api::api_keys;
use tm_storage::NewApiKey;
use tower::ServiceExt;

/// POSTs a JSON-RPC message to `/mcp` with an optional key.
async fn mcp(app: &common::App, key: Option<&str>, message: Value) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri("/api/v1/mcp")
        .header("content-type", "application/json");
    if let Some(k) = key {
        req = req.header("authorization", format!("Bearer {k}"));
    }
    let res = app
        .router
        .clone()
        .oneshot(req.body(Body::from(message.to_string())).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn tool(app: &common::App, key: &str, name: &str, arguments: Value) -> Value {
    let (status, body) = mcp(
        app,
        Some(key),
        json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["result"].clone()
}

/// Stores a key directly, for roles and tenants dev mode cannot produce.
async fn stored_key(app: &common::App, tenant: &str, role: &str) -> String {
    let key = api_keys::generate();
    app.store
        .create_api_key(NewApiKey {
            tenant,
            subject: "someone",
            name: "test",
            prefix: &api_keys::display_prefix(&key),
            hash: &api_keys::hash(&key),
            role,
            expires_at: Some(Utc::now() + Duration::days(1)),
        })
        .await
        .unwrap();
    key
}

#[tokio::test]
async fn api_keys_drive_the_mcp_endpoint_within_their_tenant_and_role() {
    let app = common::app().await;
    let tree = common::tree(&app.store, "dev").await;

    // ── Key lifecycle over REST ─────────────────────────────────────────────
    let (status, created) = app
        .send(
            "POST",
            "/me/api-keys",
            Some(json!({ "name": "VS Code", "expiresInDays": 30 })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let key = created["key"].as_str().unwrap().to_owned();
    assert!(key.starts_with("tm_"));
    assert_eq!(created["role"], "Editor");
    assert!(key.starts_with(created["prefix"].as_str().unwrap()));

    let (_, listed) = app.send("GET", "/me/api-keys", None).await;
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert!(listed[0].get("key").is_none(), "the secret is never listed");

    for bad in [
        json!({ "name": "" }),
        json!({ "name": "x", "expiresInDays": 400 }),
    ] {
        let (status, _) = app.send("POST", "/me/api-keys", Some(bad)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    // ── MCP protocol ────────────────────────────────────────────────────────
    let (status, init) = mcp(&app, Some(&key), json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test" } } })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(init["result"]["serverInfo"]["name"], "testmodeller");
    let (status, _) = mcp(
        &app,
        Some(&key),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let (_, list) = mcp(
        &app,
        Some(&key),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    )
    .await;
    assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 14);

    // ── Tools run through the real API ──────────────────────────────────────
    let projects = tool(&app, &key, "list_projects", json!({})).await;
    assert_eq!(projects["isError"], false, "{projects}");
    assert_eq!(
        projects["structuredContent"]["projects"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let model = tool(&app, &key, "get_model", json!({ "modelId": tree.model })).await;
    assert_eq!(model["isError"], false, "{model}");
    assert_eq!(
        model["structuredContent"]["model"]["states"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert!(
        model["structuredContent"]["validation"].is_array(),
        "issues, empty when valid"
    );

    let generated = tool(
        &app,
        &key,
        "generate_test_cases",
        json!({ "modelId": tree.model, "save": true }),
    )
    .await;
    assert_eq!(generated["isError"], false, "{generated}");
    let cases = tool(
        &app,
        &key,
        "list_test_cases",
        json!({ "modelId": tree.model }),
    )
    .await;
    assert!(!cases["structuredContent"]["testCases"]
        .as_array()
        .unwrap()
        .is_empty());
    let gherkin = tool(
        &app,
        &key,
        "export",
        json!({ "projectId": tree.project, "format": "gherkin" }),
    )
    .await;
    assert!(
        gherkin["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Feature"),
        "{gherkin}"
    );

    let saved = tool(&app, &key, "save_model", json!({
        "featureId": tree.feature, "name": "From MCP",
        "states": [{ "id": "a", "name": "A", "kind": "initial" }, { "id": "b", "name": "B", "kind": "final" }],
        "transitions": [{ "from": "a", "to": "b", "event": "go" }]
    })).await;
    assert_eq!(saved["isError"], false, "{saved}");

    // ── A read-only key reads but cannot write ──────────────────────────────
    let reader = stored_key(&app, "dev", "User").await;
    let read = tool(&app, &reader, "get_model", json!({ "modelId": tree.model })).await;
    assert_eq!(read["isError"], false);
    let write = tool(
        &app,
        &reader,
        "generate_test_cases",
        json!({ "modelId": tree.model }),
    )
    .await;
    assert_eq!(write["isError"], true);
    assert!(write["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("403"));

    // ── Another tenant's key sees nothing of this tenant ────────────────────
    let stranger = stored_key(&app, "other", "Editor").await;
    let none = tool(&app, &stranger, "list_projects", json!({})).await;
    assert_eq!(
        none["structuredContent"]["projects"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let hidden = tool(
        &app,
        &stranger,
        "get_model",
        json!({ "modelId": tree.model }),
    )
    .await;
    assert_eq!(hidden["isError"], true);
    assert!(hidden["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("404"));

    // ── Bad, revoked and misplaced keys ─────────────────────────────────────
    let ping = json!({ "jsonrpc": "2.0", "id": 9, "method": "ping" });
    let (status, _) = mcp(&app, Some("tm_0000"), ping.clone()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = mcp(&app, Some(&api_keys::generate()), ping.clone()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let id = created["id"].as_str().unwrap();
    let (status, _) = app
        .send("DELETE", &format!("/me/api-keys/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = mcp(&app, Some(&key), ping.clone()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = app
        .send("DELETE", &format!("/me/api-keys/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Dev mode, like every other endpoint, runs without a header.
    let (status, _) = mcp(&app, None, ping).await;
    assert_eq!(status, StatusCode::OK);

    // No stream on GET.
    let res = app
        .router
        .clone()
        .oneshot(Request::get("/api/v1/mcp").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
}
