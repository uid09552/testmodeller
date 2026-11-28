//! Requirement tags on model elements (ADR 0021). Requires Docker.

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};

/// The model of `t` as `PUT` input with `tag` on its first transition and on its first state.
async fn tag(app: &common::App, model: uuid::Uuid, tags: Value) -> (StatusCode, Value) {
    let path = format!("/models/{model}");
    let (_, m) = app.send("GET", &path, None).await;
    let mut states = m["states"].clone();
    let mut transitions = m["transitions"].clone();
    states[0]["requirements"] = tags.clone();
    transitions[1]["requirements"] = tags;
    app.send_if_match(
        "PUT",
        &path,
        m["version"].as_i64().unwrap(),
        json!({ "name": m["name"], "variables": m["variables"], "states": states, "transitions": transitions }),
    )
    .await
}

#[tokio::test]
async fn tags_are_stored_cleaned_and_drive_generation_coverage_and_traceability() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;

    // Blanks and duplicates are dropped; too many tags are refused.
    let (st, saved) = tag(
        &app,
        t.model,
        json!([" REQ-1 ", "REQ-1", "", "https://jira/TM-2"]),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{saved}");
    assert_eq!(
        saved["transitions"][1]["requirements"],
        json!(["REQ-1", "https://jira/TM-2"])
    );
    let many: Vec<String> = (0..51).map(|i| format!("R{i}")).collect();
    let (st, _) = tag(&app, t.model, json!(many)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (_, m) = app.send("GET", &format!("/models/{}", t.model), None).await;
    assert_eq!(
        m["states"][0]["requirements"],
        json!(["REQ-1", "https://jira/TM-2"])
    );

    // The criterion makes one path per tag (here both tags sit on the same elements).
    let (st, gen) = app
        .send(
            "POST",
            &format!("/models/{}/generate", t.model),
            Some(json!({ "criterion": "requirement", "save": true })),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{gen}");
    assert_eq!(gen["testCases"].as_array().unwrap().len(), 1);

    // Coverage counts tags; saved generated test cases cover them.
    let (_, cov) = app
        .send("GET", &format!("/models/{}/coverage", t.model), None)
        .await;
    assert_eq!(cov["requirements"], json!({ "covered": 2, "total": 2 }));

    // Traceability lists each tag with its elements and covering test case.
    let (st, trace) = app
        .send(
            "GET",
            &format!("/projects/{}/traceability", t.project),
            None,
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    let reqs = trace["requirements"].as_array().unwrap();
    assert_eq!(reqs.len(), 2);
    assert_eq!(reqs[0]["requirement"], "REQ-1");
    assert_eq!(reqs[0]["taggedElements"].as_array().unwrap().len(), 2);
    assert_eq!(reqs[0]["testCaseIds"].as_array().unwrap().len(), 1);
    assert_eq!(trace["summary"]["untestedRequirements"], 0);

    // Another tenant sees none of it.
    let other = common::tree(&app.store, "someone-else").await;
    let (st, _) = app
        .send(
            "GET",
            &format!("/projects/{}/traceability", other.project),
            None,
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Export and import keep the tags.
    let (_, text) = app
        .get_text(&format!("/projects/{}/export?format=json", t.project))
        .await;
    let doc: Value = serde_json::from_str(&text).unwrap();
    let (_, target) = app
        .send("POST", "/projects", Some(json!({ "name": "T" })))
        .await;
    let tid = target["id"].as_str().unwrap();
    let (st, _) = app
        .send("POST", &format!("/projects/{tid}/import"), Some(doc))
        .await;
    assert_eq!(st, StatusCode::CREATED);
    let (_, trace) = app
        .send("GET", &format!("/projects/{tid}/traceability"), None)
        .await;
    assert_eq!(trace["requirements"].as_array().unwrap().len(), 2);
}
