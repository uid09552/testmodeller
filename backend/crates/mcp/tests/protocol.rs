//! MCP framing and tool aggregation against a recording fake backend.

use std::sync::Mutex;

use serde_json::{json, Value};
use tm_mcp::{catalogue, handle, ApiRequest, ApiResponse, Backend, Body, Method};

const P: &str = "11111111-1111-4111-8111-111111111111";
const M: &str = "22222222-2222-4222-8222-222222222222";
const F: &str = "33333333-3333-4333-8333-333333333333";

/// Answers from a fixed table and records every call.
struct Fake {
    calls: Mutex<Vec<ApiRequest>>,
    answer: fn(&ApiRequest) -> ApiResponse,
}

impl Fake {
    fn new(answer: fn(&ApiRequest) -> ApiResponse) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            answer,
        }
    }
    fn calls(&self) -> Vec<ApiRequest> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl Backend for Fake {
    async fn call(&self, request: ApiRequest) -> ApiResponse {
        let response = (self.answer)(&request);
        self.calls.lock().unwrap().push(request);
        response
    }
}

fn ok(body: Value) -> ApiResponse {
    ApiResponse { status: 200, body }
}

async fn rpc(backend: &Fake, message: Value) -> Option<Value> {
    handle(backend, message.to_string().as_bytes()).await
}

async fn call(backend: &Fake, name: &str, arguments: Value) -> Value {
    let result = rpc(
        backend,
        json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }),
    )
    .await
    .unwrap()["result"]
        .clone();
    if result["isError"] == false {
        conforms(name, &result["structuredContent"]);
    }
    result
}

/// Checks `structuredContent` against the tool's `outputSchema`: an object,
/// with every required key, and declared property types where given.
fn conforms(name: &str, content: &Value) {
    let tool = catalogue().into_iter().find(|t| t["name"] == name).unwrap();
    let schema = &tool["outputSchema"];
    assert!(
        content.is_object(),
        "{name}: structuredContent must be an object"
    );
    for key in schema["required"].as_array().into_iter().flatten() {
        let key = key.as_str().unwrap();
        assert!(
            content.get(key).is_some(),
            "{name}: missing required \"{key}\" in {content}"
        );
    }
    for (key, prop) in schema["properties"].as_object().into_iter().flatten() {
        let (Some(value), Some(ty)) = (content.get(key), prop["type"].as_str()) else {
            continue;
        };
        let ok = match ty {
            "array" => value.is_array(),
            "object" => value.is_object(),
            "string" => value.is_string(),
            "boolean" => value.is_boolean(),
            _ => true,
        };
        assert!(ok, "{name}: \"{key}\" should be {ty}, got {value}");
    }
}

#[tokio::test]
async fn initialize_negotiates_the_version_and_offers_tools() {
    let fake = Fake::new(|_| ok(json!({})));
    let r = rpc(&fake, json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": { "name": "vscode" } } }))
        .await
        .unwrap();
    assert_eq!(r["result"]["protocolVersion"], "2025-03-26");
    assert!(r["result"]["capabilities"]["tools"].is_object());

    let r = rpc(&fake, json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize", "params": { "protocolVersion": "1999-01-01" } }))
        .await
        .unwrap();
    assert_eq!(r["result"]["protocolVersion"], tm_mcp::PROTOCOL_VERSION);
}

#[tokio::test]
async fn notifications_get_no_response_and_errors_are_json_rpc() {
    let fake = Fake::new(|_| ok(json!({})));
    assert!(rpc(
        &fake,
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
    )
    .await
    .is_none());

    let r = handle(&fake, b"{not json").await.unwrap();
    assert_eq!(r["error"]["code"], -32700);
    let r = rpc(
        &fake,
        json!([{ "jsonrpc": "2.0", "id": 1, "method": "ping" }]),
    )
    .await
    .unwrap();
    assert_eq!(r["error"]["code"], -32600);
    let r = rpc(
        &fake,
        json!({ "jsonrpc": "2.0", "id": 1, "method": "resources/list" }),
    )
    .await
    .unwrap();
    assert_eq!(r["error"]["code"], -32601);
    let r = rpc(
        &fake,
        json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "nope" } }),
    )
    .await
    .unwrap();
    assert_eq!(r["error"]["code"], -32602);
    assert!(fake.calls().is_empty());
}

#[test]
fn the_catalogue_matches_the_spec_and_marks_writes() {
    let tools = catalogue();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "list_projects",
            "search",
            "get_model",
            "save_model",
            "generate_test_cases",
            "list_test_cases",
            "export",
            "import_test_results",
            "get_coverage",
            "get_traceability",
            "get_threat_model",
            "request_ai_proposals",
            "list_proposals",
            "decide_proposal",
        ]
    );
    let writes: Vec<&str> = tools
        .iter()
        .filter(|t| t["annotations"]["readOnlyHint"] == false)
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        writes,
        [
            "save_model",
            "generate_test_cases",
            "import_test_results",
            "request_ai_proposals",
            "decide_proposal"
        ]
    );
    let destructive: Vec<&str> = tools
        .iter()
        .filter(|t| t["annotations"]["destructiveHint"] == true)
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(destructive, ["save_model", "decide_proposal"]);
    let repeatable_writes: Vec<&str> = tools
        .iter()
        .filter(|t| t["annotations"]["readOnlyHint"] == false)
        .filter(|t| t["annotations"]["idempotentHint"] == true)
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(repeatable_writes, ["import_test_results"]);
    for t in &tools {
        assert_eq!(t["inputSchema"]["type"], "object", "{}", t["name"]);
        assert_eq!(t["outputSchema"]["type"], "object", "{}", t["name"]);
    }
}

#[tokio::test]
async fn list_projects_pages_and_attaches_each_tree() {
    let fake = Fake::new(
        |r| match (r.path.as_str(), r.query.iter().any(|(k, _)| k == "cursor")) {
            ("/projects", false) => {
                ok(json!({ "items": [{ "id": P, "name": "Shop" }], "nextCursor": "c1" }))
            }
            ("/projects", true) => {
                ok(json!({ "items": [{ "id": F, "name": "Shop 2" }], "nextCursor": null }))
            }
            (p, _) if p.ends_with("/tree") => ok(json!({ "components": [] })),
            _ => ApiResponse {
                status: 404,
                body: json!({}),
            },
        },
    );
    let r = call(&fake, "list_projects", json!({})).await;
    assert_eq!(r["isError"], false);
    let projects = r["structuredContent"]["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2);
    assert_eq!(projects[1]["project"]["name"], "Shop 2");
    assert_eq!(fake.calls().len(), 4);
}

#[tokio::test]
async fn ids_must_be_uuids_before_any_call() {
    let fake = Fake::new(|_| ok(json!({})));
    let r = call(&fake, "get_model", json!({ "modelId": "../projects" })).await;
    assert_eq!(r["isError"], true);
    assert!(r["content"][0]["text"].as_str().unwrap().contains("UUID"));
    let r = call(&fake, "export", json!({ "projectId": P, "format": "pdf" })).await;
    assert_eq!(r["isError"], true);
    assert!(fake.calls().is_empty());
}

#[tokio::test]
async fn a_refused_call_is_a_tool_error_with_the_problem_detail() {
    let fake = Fake::new(|_| ApiResponse {
        status: 403,
        body: json!({ "detail": "the User role may not POST this resource" }),
    });
    let r = call(&fake, "generate_test_cases", json!({ "modelId": M })).await;
    assert_eq!(r["isError"], true);
    let text = r["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("403") && text.contains("Editor role"),
        "{text}"
    );
}

#[tokio::test]
async fn get_model_works_for_read_only_keys_without_validation() {
    let fake = Fake::new(|r| match r.method {
        Method::Post => ApiResponse {
            status: 403,
            body: json!({ "detail": "no" }),
        },
        _ if r.path.ends_with("/coverage") => ok(json!({ "percent": 50 })),
        _ => ok(json!({ "id": M, "name": "Login" })),
    });
    let r = call(&fake, "get_model", json!({ "modelId": M })).await;
    assert_eq!(r["isError"], false);
    assert_eq!(r["structuredContent"]["model"]["name"], "Login");
    assert!(r["structuredContent"].get("validation").is_none());
    assert_eq!(r["structuredContent"]["coverage"]["percent"], 50);
}

#[tokio::test]
async fn save_model_creates_with_short_state_keys() {
    let fake = Fake::new(|r| {
        ok(json!({ "echo": match &r.body { Body::Json(b) => b.clone(), _ => Value::Null } }))
    });
    let r = call(
        &fake,
        "save_model",
        json!({
            "featureId": F, "name": "Login",
            "states": [
                { "id": "start", "name": "Start", "kind": "initial" },
                { "id": "done", "name": "Done", "kind": "final" }
            ],
            "transitions": [{ "from": "start", "to": "done", "event": "submit" }]
        }),
    )
    .await;
    assert_eq!(r["isError"], false, "{r}");
    let calls = fake.calls();
    assert_eq!(calls[0].method, Method::Post);
    assert_eq!(calls[0].path, format!("/features/{F}/models"));
    let Body::Json(body) = &calls[0].body else {
        panic!()
    };
    let start = body["states"][0]["id"].as_str().unwrap();
    assert!(start.parse::<uuid::Uuid>().is_ok());
    assert_eq!(body["transitions"][0]["from"], start);

    let r = call(
        &fake,
        "save_model",
        json!({
            "featureId": F, "name": "x", "states": [{ "id": "a", "name": "A", "kind": "initial" }],
            "transitions": [{ "from": "a", "to": "missing", "event": "e" }]
        }),
    )
    .await;
    assert_eq!(r["isError"], true);
}

#[tokio::test]
async fn save_model_replaces_with_the_current_version_and_keeps_variables() {
    let fake = Fake::new(|r| match r.method {
        Method::Get => ok(
            json!({ "id": M, "name": "Old", "version": 7, "variables": [{ "name": "attempts" }], "status": "draft" }),
        ),
        _ => ok(json!({ "id": M })),
    });
    let r = call(
        &fake,
        "save_model",
        json!({
            "modelId": M,
            "states": [{ "id": "a", "name": "A", "kind": "initial" }],
            "transitions": []
        }),
    )
    .await;
    assert_eq!(r["isError"], false, "{r}");
    let put = &fake.calls()[1];
    assert_eq!(put.method, Method::Put);
    assert_eq!(put.if_match, Some(7));
    let Body::Json(body) = &put.body else {
        panic!()
    };
    assert_eq!(body["name"], "Old");
    assert_eq!(body["variables"][0]["name"], "attempts");
    assert!(
        body.get("layout").is_none(),
        "an absent layout keeps the stored one"
    );
}

#[tokio::test]
async fn coverage_needs_exactly_one_scope() {
    let fake = Fake::new(|_| ok(json!({})));
    assert_eq!(
        call(&fake, "get_coverage", json!({})).await["isError"],
        true
    );
    assert_eq!(
        call(
            &fake,
            "get_coverage",
            json!({ "modelId": M, "featureId": F })
        )
        .await["isError"],
        true
    );
    assert_eq!(
        call(&fake, "get_coverage", json!({ "featureId": F })).await["isError"],
        false
    );
    assert_eq!(fake.calls()[0].path, format!("/features/{F}/coverage"));
}

#[tokio::test]
async fn import_sends_the_file_as_raw_text() {
    let fake = Fake::new(|_| ok(json!({ "matched": 1 })));
    let r = call(
        &fake,
        "import_test_results",
        json!({ "projectId": P, "format": "junit", "content": "<testsuite/>", "dryRun": true }),
    )
    .await;
    assert_eq!(r["isError"], false);
    let c = &fake.calls()[0];
    assert_eq!(
        c.body,
        Body::Text {
            content_type: "application/xml",
            text: "<testsuite/>".into()
        }
    );
    assert!(c.query.contains(&("dryRun".into(), "true".into())));
}

#[tokio::test]
async fn ai_proposals_wait_for_the_job_and_never_accept() {
    let fake = Fake::new(|r| match (r.method, r.path.as_str()) {
        (Method::Post, "/ai/proposals") => ApiResponse {
            status: 202,
            body: json!({ "id": M, "status": "queued" }),
        },
        (Method::Get, p) if p.starts_with("/jobs/") => {
            ok(json!({ "id": M, "status": "succeeded", "proposalIds": [P] }))
        }
        (Method::Get, p) if p.starts_with("/proposals/") => {
            ok(json!({ "id": P, "status": "pending" }))
        }
        _ => ApiResponse {
            status: 500,
            body: json!({}),
        },
    });
    let r = call(
        &fake,
        "request_ai_proposals",
        json!({ "featureId": F, "kind": "test-cases" }),
    )
    .await;
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(r["structuredContent"]["proposals"][0]["status"], "pending");
    assert!(fake.calls().iter().all(|c| !c.path.ends_with("/accept")));
}

#[tokio::test]
async fn decide_proposal_posts_the_decision() {
    let fake = Fake::new(|_| ok(json!({ "status": "rejected" })));
    call(
        &fake,
        "decide_proposal",
        json!({ "proposalId": P, "decision": "reject" }),
    )
    .await;
    assert_eq!(fake.calls()[0].path, format!("/proposals/{P}/reject"));
    let r = call(
        &fake,
        "decide_proposal",
        json!({ "proposalId": P, "decision": "reject", "payload": {} }),
    )
    .await;
    assert_eq!(r["isError"], true);
}
