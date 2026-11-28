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
            "tm_list_projects",
            "tm_search",
            "tm_get_model",
            "tm_save_model",
            "tm_generate_test_cases",
            "tm_list_test_cases",
            "tm_export",
            "tm_import_test_results",
            "tm_get_coverage",
            "tm_get_traceability",
            "tm_get_threat_model",
            "tm_request_ai_proposals",
            "tm_get_ai_job",
            "tm_list_proposals",
            "tm_decide_proposal",
            "tm_get_project_status",
            "tm_triage_failures",
            "tm_update_test_case",
            "tm_compare_model_versions",
            "tm_patch_model",
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
            "tm_save_model",
            "tm_generate_test_cases",
            "tm_import_test_results",
            "tm_request_ai_proposals",
            "tm_decide_proposal",
            "tm_update_test_case",
            "tm_patch_model"
        ]
    );
    let destructive: Vec<&str> = tools
        .iter()
        .filter(|t| t["annotations"]["destructiveHint"] == true)
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        destructive,
        [
            "tm_save_model",
            "tm_decide_proposal",
            "tm_update_test_case",
            "tm_patch_model"
        ]
    );
    let repeatable_writes: Vec<&str> = tools
        .iter()
        .filter(|t| t["annotations"]["readOnlyHint"] == false)
        .filter(|t| t["annotations"]["idempotentHint"] == true)
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        repeatable_writes,
        ["tm_import_test_results", "tm_update_test_case"]
    );
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
    let r = call(&fake, "tm_list_projects", json!({})).await;
    assert_eq!(r["isError"], false);
    let projects = r["structuredContent"]["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2);
    assert_eq!(projects[1]["project"]["name"], "Shop 2");
    assert_eq!(fake.calls().len(), 4);
}

#[tokio::test]
async fn ids_must_be_uuids_before_any_call() {
    let fake = Fake::new(|_| ok(json!({})));
    let r = call(&fake, "tm_get_model", json!({ "modelId": "../projects" })).await;
    assert_eq!(r["isError"], true);
    assert!(r["content"][0]["text"].as_str().unwrap().contains("UUID"));
    let r = call(
        &fake,
        "tm_export",
        json!({ "projectId": P, "format": "pdf" }),
    )
    .await;
    assert_eq!(r["isError"], true);
    assert!(fake.calls().is_empty());
}

#[tokio::test]
async fn a_refused_call_is_a_tool_error_with_the_problem_detail() {
    let fake = Fake::new(|_| ApiResponse {
        status: 403,
        body: json!({ "detail": "the User role may not POST this resource" }),
    });
    let r = call(&fake, "tm_generate_test_cases", json!({ "modelId": M })).await;
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
    let r = call(&fake, "tm_get_model", json!({ "modelId": M })).await;
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
        "tm_save_model",
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
        "tm_save_model",
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
        "tm_save_model",
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
        call(&fake, "tm_get_coverage", json!({})).await["isError"],
        true
    );
    assert_eq!(
        call(
            &fake,
            "tm_get_coverage",
            json!({ "modelId": M, "featureId": F })
        )
        .await["isError"],
        true
    );
    assert_eq!(
        call(&fake, "tm_get_coverage", json!({ "featureId": F })).await["isError"],
        false
    );
    assert_eq!(fake.calls()[0].path, format!("/features/{F}/coverage"));
}

#[tokio::test]
async fn import_sends_the_file_as_raw_text() {
    let fake = Fake::new(|_| ok(json!({ "matched": 1 })));
    let r = call(
        &fake,
        "tm_import_test_results",
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
        "tm_request_ai_proposals",
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
        "tm_decide_proposal",
        json!({ "proposalId": P, "decision": "reject" }),
    )
    .await;
    assert_eq!(fake.calls()[0].path, format!("/proposals/{P}/reject"));
    let r = call(
        &fake,
        "tm_decide_proposal",
        json!({ "proposalId": P, "decision": "reject", "payload": {} }),
    )
    .await;
    assert_eq!(r["isError"], true);
}

#[tokio::test]
async fn lists_stop_at_the_limit_and_say_so() {
    // Every page has two projects and another page after it.
    let fake = Fake::new(|r| match r.path.as_str() {
        "/projects" => ok(json!({
            "items": [{ "id": P, "name": "A", "description": "long" }, { "id": F, "name": "B" }],
            "nextCursor": "more"
        })),
        p if p.ends_with("/tree") => ok(json!({ "components": [] })),
        _ => ApiResponse {
            status: 404,
            body: json!({}),
        },
    });
    let r = call(&fake, "tm_list_projects", json!({ "limit": 3 })).await;
    assert_eq!(r["isError"], false, "{r}");
    let s = &r["structuredContent"];
    assert_eq!(s["projects"].as_array().unwrap().len(), 3);
    assert_eq!(s["truncated"], true);
    // Concise by default: no description.
    assert_eq!(s["projects"][0]["project"], json!({ "id": P, "name": "A" }));
    // Two pages were enough for three items.
    assert_eq!(
        fake.calls()
            .iter()
            .filter(|c| c.path == "/projects")
            .count(),
        2
    );

    let r = call(
        &fake,
        "tm_list_projects",
        json!({ "limit": 1, "detail": "full" }),
    )
    .await;
    assert_eq!(
        r["structuredContent"]["projects"][0]["project"]["description"],
        "long"
    );

    for bad in [
        json!({ "limit": 0 }),
        json!({ "limit": 501 }),
        json!({ "detail": "all" }),
    ] {
        let r = call(&fake, "tm_list_projects", bad.clone()).await;
        assert_eq!(r["isError"], true, "{bad}");
    }
}

#[tokio::test]
async fn concise_test_cases_keep_identity_and_last_result() {
    let fake = Fake::new(|_| {
        ok(json!([{
            "id": P, "name": "Lockout", "status": "draft", "featureId": F, "threatIds": [M],
            "steps": [{ "action": "a", "expected": "b" }], "lastResult": { "status": "failed", "message": "x" }
        }]))
    });
    let r = call(&fake, "tm_list_test_cases", json!({ "modelId": M })).await;
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(
        r["structuredContent"]["testCases"][0],
        json!({ "id": P, "name": "Lockout", "status": "draft", "featureId": F,
                "threatIds": [M], "lastResult": "failed" })
    );
    assert_eq!(r["structuredContent"]["truncated"], false);
}

#[tokio::test]
async fn an_ai_job_can_be_followed_up_and_never_accepts() {
    let fake = Fake::new(|r| match r.path.as_str() {
        p if p == format!("/jobs/{M}") => {
            ok(json!({ "id": M, "status": "succeeded", "proposalIds": [P] }))
        }
        p if p == format!("/jobs/{F}") => ok(json!({ "id": F, "status": "running" })),
        p if p.starts_with("/proposals/") => ok(json!({ "id": P, "status": "pending" })),
        _ => ApiResponse {
            status: 404,
            body: json!({}),
        },
    });
    let r = call(&fake, "tm_get_ai_job", json!({ "jobId": M })).await;
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(r["structuredContent"]["proposals"][0]["status"], "pending");

    let r = call(&fake, "tm_get_ai_job", json!({ "jobId": F })).await;
    assert!(r["structuredContent"].get("proposals").is_none());
    assert!(r["structuredContent"]["note"]
        .as_str()
        .unwrap()
        .contains("still running"));

    let r = call(&fake, "tm_get_ai_job", json!({ "jobId": "nope" })).await;
    assert_eq!(r["isError"], true);
    assert!(fake.calls().iter().all(|c| c.method == Method::Get));
}

const T1: &str = "44444444-4444-4444-8444-444444444441";
const T2: &str = "44444444-4444-4444-8444-444444444442";
const TR: &str = "55555555-5555-4555-8555-555555555551";
const S1: &str = "66666666-6666-4666-8666-666666666661";
const S2: &str = "66666666-6666-4666-8666-666666666662";

fn trace_answer(r: &ApiRequest) -> ApiResponse {
    if r.path.ends_with("/traceability") {
        let failed = |id: &str, name: &str| {
            json!({ "id": id, "name": name, "featureId": F, "componentId": F,
                "lastResult": { "status": "failed", "message": "boom", "runId": "r1" },
                "elements": [{ "modelId": M, "transitionId": TR }] })
        };
        return ok(json!({
            "items": [{ "backlogUrl": "https://x/1", "testCases": [
                failed(T1, "Lockout"),
                { "id": T2, "name": "Fine", "featureId": F, "componentId": F,
                  "lastResult": { "status": "passed" }, "elements": [] } ] }],
            "untraced": [{ "id": "44444444-4444-4444-8444-444444444443", "name": "Loose",
                "featureId": F, "componentId": F,
                "lastResult": { "status": "error" }, "elements": [] }],
            "summary": { "backlogItems": 1, "testCases": 3, "untraced": 1, "unimplemented": 0, "noElements": 0 }
        }));
    }
    if r.path.ends_with("/tree") {
        return ok(
            json!({ "components": [{ "features": [{ "models": [{ "id": M, "name": "Login" }] }] }] }),
        );
    }
    if r.path.ends_with("/proposals") {
        return ok(json!({ "items": [{ "id": T1 }], "nextCursor": null }));
    }
    if r.path.starts_with("/models/") && r.path.ends_with("/validate") {
        return ok(json!([]));
    }
    if r.path.starts_with("/models/") && r.path.ends_with("/coverage") {
        return ok(json!({ "transitions": { "covered": 1, "total": 2 } }));
    }
    if r.path.starts_with("/models/") {
        return ok(json!({ "id": M, "name": "Login", "version": 3,
            "states": [{ "id": S1, "name": "Start", "kind": "initial" }, { "id": S2, "name": "Locked", "kind": "final" }],
            "transitions": [{ "id": TR, "from": S1, "to": S2, "event": "3 bad passwords" }] }));
    }
    ok(json!({}))
}

#[tokio::test]
async fn project_status_gathers_models_gaps_proposals_and_failures() {
    let fake = Fake::new(trace_answer);
    let r = call(&fake, "tm_get_project_status", json!({ "projectId": P })).await;
    assert_eq!(r["isError"], false, "{r}");
    let c = &r["structuredContent"];
    assert_eq!(c["models"][0]["name"], "Login");
    assert_eq!(c["models"][0]["validation"], json!([]));
    assert_eq!(c["traceability"]["testCases"], 3);
    assert_eq!(c["pendingProposals"], 1);
    let failed: Vec<&str> = c["failedTestCases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(failed, ["Lockout", "Loose"]);
    assert!(fake.calls().iter().all(|c| c.method != Method::Put));
}

#[tokio::test]
async fn triage_maps_failures_onto_transitions_and_lists_unassigned() {
    let fake = Fake::new(trace_answer);
    let r = call(&fake, "tm_triage_failures", json!({ "projectId": P })).await;
    assert_eq!(r["isError"], false, "{r}");
    let c = &r["structuredContent"];
    let e = &c["elements"][0];
    assert_eq!(e["type"], "transition");
    assert_eq!(e["id"], TR);
    assert_eq!(e["label"], "Start -> Locked on \"3 bad passwords\"");
    assert_eq!(e["failures"][0]["message"], "boom");
    assert_eq!(c["elements"].as_array().unwrap().len(), 1);
    assert_eq!(c["unassigned"][0]["name"], "Loose");
}

#[tokio::test]
async fn update_test_case_changes_only_the_given_fields() {
    let fake = Fake::new(|r| match r.method {
        Method::Get => ok(
            json!({ "id": T1, "version": 4, "name": "Old", "priority": "high",
            "tags": ["smoke"], "threatIds": [S1],
            "steps": [{ "order": 1, "action": "a", "expected": "b" }] }),
        ),
        _ => ok(json!({ "id": T1 })),
    });
    let r = call(
        &fake,
        "tm_update_test_case",
        json!({ "testCaseId": T1, "name": "New" }),
    )
    .await;
    assert_eq!(r["isError"], false, "{r}");
    let put = &fake.calls()[1];
    assert_eq!(put.method, Method::Put);
    assert_eq!(put.if_match, Some(4));
    let Body::Json(body) = &put.body else {
        panic!()
    };
    assert_eq!(body["name"], "New");
    assert_eq!(body["priority"], "high");
    assert_eq!(body["tags"], json!(["smoke"]));
    assert_eq!(body["threatIds"], json!([S1]));
    assert_eq!(body["steps"], json!([{ "action": "a", "expected": "b" }]));
}

#[tokio::test]
async fn patch_model_adds_and_removes_and_drops_dangling_transitions() {
    let fake = Fake::new(trace_answer);
    let r = call(
        &fake,
        "tm_patch_model",
        json!({
            "modelId": M,
            "removeStateIds": [S2],
            "addStates": [{ "id": "ok", "name": "Done", "kind": "final" }],
            "addTransitions": [{ "from": S1, "to": "ok", "event": "login" }]
        }),
    )
    .await;
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(r["structuredContent"]["removedTransitions"], 1);
    let put = fake
        .calls()
        .into_iter()
        .find(|c| c.method == Method::Put)
        .unwrap();
    assert_eq!(put.if_match, Some(3));
    let Body::Json(body) = &put.body else {
        panic!()
    };
    let states = body["states"].as_array().unwrap();
    assert_eq!(states.len(), 2);
    assert!(states.iter().all(|s| s["id"] != S2));
    let t = &body["transitions"].as_array().unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0]["event"], "login");
    assert_eq!(t[0]["from"], S1);

    // A transition to an unknown state is refused before any write.
    let fake = Fake::new(trace_answer);
    let r = call(
        &fake,
        "tm_patch_model",
        json!({ "modelId": M, "addTransitions": [{ "from": S1, "to": "nope", "event": "x" }] }),
    )
    .await;
    assert_eq!(r["isError"], true);
    assert!(fake.calls().iter().all(|c| c.method != Method::Put));
}

// ── The stateless revision 2026-07-28 (ADR 0024) ─────────────────────────────

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": { "name": "test" }
    })
}

fn stateless_headers(method: &str, name: Option<&str>) -> tm_mcp::RequestHeaders {
    tm_mcp::RequestHeaders {
        protocol_version: Some("2026-07-28".into()),
        method: Some(method.into()),
        name: name.map(str::to_owned),
    }
}

async fn http(fake: &Fake, headers: &tm_mcp::RequestHeaders, message: Value) -> tm_mcp::Reply {
    tm_mcp::handle_http(fake, headers, message.to_string().as_bytes()).await
}

#[tokio::test]
async fn discover_lists_versions_and_caching_without_a_handshake() {
    let fake = Fake::new(|_| ok(json!({})));
    let r = http(
        &fake,
        &stateless_headers("server/discover", None),
        json!({ "jsonrpc": "2.0", "id": 1, "method": "server/discover", "params": { "_meta": meta() } }),
    )
    .await;
    assert_eq!(r.status, 200);
    let res = &r.body.unwrap()["result"];
    assert_eq!(res["resultType"], "complete");
    assert_eq!(res["supportedVersions"][0], "2026-07-28");
    assert!(res["supportedVersions"]
        .as_array()
        .unwrap()
        .contains(&json!("2025-06-18")));
    assert!(res["ttlMs"].as_i64().unwrap() >= 0);
    assert!(["public", "private"].contains(&res["cacheScope"].as_str().unwrap()));
    assert_eq!(
        res["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "testmodeller"
    );
    // Old clients can still ask for discovery without any version.
    let r = http(
        &fake,
        &tm_mcp::RequestHeaders::default(),
        json!({ "jsonrpc": "2.0", "id": 1, "method": "server/discover" }),
    )
    .await;
    assert_eq!(r.status, 200);
}

#[tokio::test]
async fn stateless_lists_and_calls_carry_resulttype_and_cache_hints() {
    let fake = Fake::new(|_| ok(json!({ "items": [], "nextCursor": null })));
    let r = http(
        &fake,
        &stateless_headers("tools/list", None),
        json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": { "_meta": meta() } }),
    )
    .await;
    let res = r.body.unwrap()["result"].clone();
    assert_eq!(res["resultType"], "complete");
    assert_eq!(
        res["tools"].as_array().unwrap().len(),
        tm_mcp::catalogue().len()
    );
    assert!(res["ttlMs"].is_i64() && res["cacheScope"].is_string());

    let r = http(
        &fake,
        &stateless_headers("tools/call", Some("tm_list_projects")),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": { "_meta": meta(), "name": "tm_list_projects", "arguments": {} } }),
    )
    .await;
    assert_eq!(r.status, 200);
    let res = r.body.unwrap()["result"].clone();
    assert_eq!(res["resultType"], "complete");
    assert_eq!(res["isError"], false);
}

#[tokio::test]
async fn stateless_requests_are_checked_against_their_headers() {
    let fake = Fake::new(|_| ok(json!({})));
    let list = |m: Value| json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": { "_meta": m } });
    let code = |r: &tm_mcp::Reply| r.body.as_ref().unwrap()["error"]["code"].as_i64().unwrap();

    // No MCP-Protocol-Version header, or a different one, or no Mcp-Method.
    let mut h = stateless_headers("tools/list", None);
    h.protocol_version = None;
    let r = http(&fake, &h, list(meta())).await;
    assert_eq!((r.status, code(&r)), (400, -32020));
    let mut h = stateless_headers("tools/list", None);
    h.protocol_version = Some("2025-06-18".into());
    let r = http(&fake, &h, list(meta())).await;
    assert_eq!((r.status, code(&r)), (400, -32020));
    let r = http(&fake, &stateless_headers("tools/call", None), list(meta())).await;
    assert_eq!((r.status, code(&r)), (400, -32020));

    // tools/call needs Mcp-Name equal to the tool name.
    let call = json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": { "_meta": meta(), "name": "tm_list_projects" } });
    let r = http(
        &fake,
        &stateless_headers("tools/call", Some("tm_search")),
        call,
    )
    .await;
    assert_eq!((r.status, code(&r)), (400, -32020));

    // Required _meta missing.
    let r = http(
        &fake,
        &stateless_headers("tools/list", None),
        json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }),
    )
    .await;
    assert_eq!((r.status, code(&r)), (400, -32602));

    // Removed methods are unknown: initialize and ping.
    for m in ["initialize", "ping"] {
        let r = http(
            &fake,
            &stateless_headers(m, None),
            json!({ "jsonrpc": "2.0", "id": 1, "method": m, "params": { "_meta": meta() } }),
        )
        .await;
        assert_eq!((r.status, code(&r)), (404, -32601), "{m}");
    }

    // A notification is accepted without a body.
    let r = http(
        &fake,
        &stateless_headers("notifications/cancelled", None),
        json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": { "_meta": meta() } }),
    )
    .await;
    assert_eq!((r.status, r.body), (202, None));
}

#[tokio::test]
async fn an_unknown_version_is_refused_with_the_supported_ones() {
    let fake = Fake::new(|_| ok(json!({})));
    let r = http(
        &fake,
        &tm_mcp::RequestHeaders {
            protocol_version: Some("2031-01-01".into()),
            method: Some("tools/list".into()),
            name: None,
        },
        json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }),
    )
    .await;
    assert_eq!(r.status, 400);
    let e = &r.body.unwrap()["error"];
    assert_eq!(e["code"], -32022);
    assert_eq!(e["data"]["requested"], "2031-01-01");
    assert!(e["data"]["supported"]
        .as_array()
        .unwrap()
        .contains(&json!("2026-07-28")));
}

#[tokio::test]
async fn earlier_revisions_keep_their_handshake_with_or_without_the_version_header() {
    let fake = Fake::new(|_| ok(json!({})));
    let legacy = tm_mcp::RequestHeaders {
        protocol_version: Some("2025-06-18".into()),
        method: None,
        name: None,
    };
    let r = http(
        &fake,
        &legacy,
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": { "protocolVersion": "2025-06-18", "capabilities": {} } }),
    )
    .await;
    assert_eq!(r.status, 200);
    assert_eq!(r.body.unwrap()["result"]["protocolVersion"], "2025-06-18");
    let r = http(
        &fake,
        &legacy,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" }),
    )
    .await;
    assert_eq!(r.status, 200);
    let r = http(
        &fake,
        &tm_mcp::RequestHeaders::default(),
        json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list" }),
    )
    .await;
    assert!(r.body.unwrap()["result"].get("resultType").is_none());
}
