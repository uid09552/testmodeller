//! The generation criteria prime-paths, reach-element and random, and transition
//! weights over HTTP (ADR 0017). Requires Docker.

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};

async fn generate(app: &common::App, model: uuid::Uuid, body: Value) -> (StatusCode, Value) {
    app.send("POST", &format!("/models/{model}/generate"), Some(body))
        .await
}

#[tokio::test]
async fn the_new_criteria_generate_and_validate_their_options() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let m = t.model;

    // Prime paths of the chain Start -> Out -> In -> Done: one path through it.
    let (st, v) = generate(&app, m, json!({"criterion": "prime-paths", "seed": 1})).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["testCases"].as_array().unwrap().len(), 1);
    assert_eq!(v["coverage"]["transitions"]["covered"], 3);

    // Reach an element: needs a target, which must be in the model.
    let (st, v) = generate(&app, m, json!({"criterion": "reach-element"})).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{v}");
    let (st, v) = generate(
        &app,
        m,
        json!({"criterion": "reach-element", "targetId": uuid::Uuid::new_v4()}),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::NOT_FOUND,
        "an id that exists nowhere is not found: {v}"
    );
    let target = t.graph.states[2].id; // "In"
    let (st, v) = generate(
        &app,
        m,
        json!({"criterion": "reach-element", "targetId": target}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["testCases"].as_array().unwrap().len(), 1);

    // Random walks with a stop condition; options are range checked.
    let (st, v) = generate(
        &app,
        m,
        json!({"criterion": "random", "seed": 5, "stopAfterTests": 3}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert!(!v["testCases"].as_array().unwrap().is_empty());
    for bad in [
        json!({"stopAfterTests": 0}),
        json!({"stopAfterTests": 1001}),
        json!({"stopAtCoverage": 0}),
        json!({"stopAtCoverage": 101}),
    ] {
        let mut body = json!({"criterion": "random"});
        body.as_object_mut()
            .unwrap()
            .extend(bad.as_object().unwrap().clone());
        let (st, v) = generate(&app, m, body).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "{v}");
    }
}

#[tokio::test]
async fn transition_weights_are_stored_and_range_checked() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let tr = &t.graph.transitions[1];
    let path = format!("/transitions/{}", tr.id);
    let body = |w: i64| json!({"from": tr.from, "to": tr.to, "event": tr.event, "weight": w});

    let (st, v) = app.send("PATCH", &path, Some(body(7))).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["weight"], 7);
    let (_, v) = app.send("GET", &format!("/models/{}", t.model), None).await;
    let stored = v["transitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == json!(tr.id))
        .unwrap()
        .clone();
    assert_eq!(stored["weight"], 7);

    let (st, _) = app.send("PATCH", &path, Some(body(1001))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = app.send("PATCH", &path, Some(body(-1))).await;
    assert!(st.is_client_error());

    // Weight 0 on the only way forward leaves a random walk nowhere to go.
    let (st, _) = app
        .send(
            "PATCH",
            &format!("/transitions/{}", t.graph.transitions[0].id),
            Some(json!({
        "from": t.graph.transitions[0].from, "to": t.graph.transitions[0].to,
        "event": t.graph.transitions[0].event, "weight": 0})),
        )
        .await;
    assert_eq!(st, StatusCode::OK);
    let (st, v) = generate(&app, t.model, json!({"criterion": "random", "seed": 1})).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["testCases"], json!([]));
    assert!(!v["skippedPaths"].as_array().unwrap().is_empty());
}

/// AGENTS.md rule 10: the target of reach-element is tenant-checked.
#[tokio::test]
async fn another_tenants_target_is_not_found() {
    let app = common::app().await;
    let mine = common::tree(&app.store, "dev").await;
    let theirs = common::tree(&app.store, "someone-else").await;
    for target in [theirs.graph.states[1].id, theirs.graph.transitions[0].id] {
        let (st, v) = generate(
            &app,
            mine.model,
            json!({"criterion": "reach-element", "targetId": target}),
        )
        .await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{v}");
    }
}

#[tokio::test]
async fn sneak_paths_end_with_a_rejected_event_and_stay_fresh_when_saved() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    // Chain Start -open-> Out -login-> In -close-> Done: Start refuses login and close, Out
    // refuses open and close, In refuses open and login: six sneak tests.
    let (st, v) = generate(
        &app,
        t.model,
        json!({"criterion": "sneak-path", "save": true}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let cases = v["testCases"].as_array().unwrap();
    assert_eq!(cases.len(), 6, "{v}");
    for c in cases {
        let steps = c["steps"].as_array().unwrap();
        let last = steps.last().unwrap();
        assert!(last["action"].as_str().unwrap().contains("not allowed"));
        assert!(last["expected"].as_str().unwrap().contains("rejected"));
    }
    // A saved sneak test is not reported stale: its extra step has no transition.
    let (st, stale) = app
        .send("GET", &format!("/models/{}/stale-tests", t.model), None)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert!(stale.as_array().unwrap().is_empty(), "{stale}");
}

#[tokio::test]
async fn generated_steps_carry_variable_values_and_keep_them_through_an_edit() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let path = format!("/models/{}", t.model);
    let (_, m) = app.send("GET", &path, None).await;
    let mut transitions = m["transitions"].clone();
    transitions[0]["action"] = json!("tries = tries + 1");
    let (st, saved) = app
        .send_if_match(
            "PUT",
            &path,
            m["version"].as_i64().unwrap(),
            json!({ "name": m["name"], "variables": [{ "name": "tries", "type": "integer", "initial": 2 }],
                    "states": m["states"], "transitions": transitions }),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{saved}");

    let (st, v) = generate(
        &app,
        t.model,
        json!({"criterion": "transition", "save": true}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let tc = &v["testCases"][0];
    assert_eq!(tc["steps"][0]["data"], json!({ "tries": 2 }));
    assert_eq!(tc["steps"][1]["data"], json!({ "tries": 3 }));

    // Editing the test case with the steps as read keeps the data.
    let id = tc["id"].as_str().unwrap();
    let (_, read) = app.send("GET", &format!("/test-cases/{id}"), None).await;
    let (st, edited) = app
        .send_if_match(
            "PUT",
            &format!("/test-cases/{id}"),
            read["version"].as_i64().unwrap(),
            json!({ "name": "Renamed", "steps": read["steps"] }),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{edited}");
    assert_eq!(edited["steps"][1]["data"], json!({ "tries": 3 }));
}
