//! HTTP-level coverage of the org tree, models, test cases and export/import (#33).
//! Requires Docker.

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};

#[tokio::test]
async fn project_component_feature_crud_and_cascade() {
    let app = common::app().await;
    let (st, p) = app
        .send("POST", "/projects", Some(json!({ "name": "Shop" })))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    let pid = p["id"].as_str().unwrap().to_owned();

    let (st, c) = app
        .send(
            "POST",
            &format!("/projects/{pid}/components"),
            Some(json!({ "name": "Checkout" })),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    let cid = c["id"].as_str().unwrap().to_owned();

    let (st, f) = app
        .send(
            "POST",
            &format!("/components/{cid}/features"),
            Some(json!({ "name": "Pay", "scenarioDescription": "As a buyer" })),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "{f}");
    let fid = f["id"].as_str().unwrap().to_owned();
    assert_eq!(f["scenarioDescription"], "As a buyer");

    // Rename (needs the version); empty names are refused.
    let fpath = format!("/features/{fid}");
    let v = f["version"].as_i64().unwrap();
    let (st, r) = app
        .send_if_match("PATCH", &fpath, v, json!({ "name": "Pay now" }))
        .await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert_eq!(r["name"], "Pay now");
    let (st, _) = app
        .send_if_match("PATCH", &fpath, v, json!({ "name": "Again" }))
        .await;
    assert_eq!(st, StatusCode::PRECONDITION_FAILED, "stale version");
    let (st, _) = app
        .send_if_match(
            "PATCH",
            &fpath,
            r["version"].as_i64().unwrap(),
            json!({ "name": "" }),
        )
        .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // The tree shows it; an unknown id is 404.
    let (st, tree) = app
        .send("GET", &format!("/projects/{pid}/tree"), None)
        .await;
    assert_eq!(st, StatusCode::OK);
    assert!(tree.to_string().contains("Pay now"));
    let (st, _) = app
        .send("GET", &format!("/projects/{}", uuid::Uuid::new_v4()), None)
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Deleting a project removes what is below it.
    let (st, _) = app.send("DELETE", &format!("/projects/{pid}"), None).await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (st, _) = app.send("GET", &format!("/features/{fid}"), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn model_put_needs_the_current_version() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let path = format!("/models/{}", t.model);
    let (_, m) = app.send("GET", &path, None).await;
    let body = json!({
        "name": "Renamed",
        "variables": m["variables"],
        "states": m["states"],
        "transitions": m["transitions"],
    });
    let v = m["version"].as_i64().unwrap();

    let (st, _) = app.send("PUT", &path, Some(body.clone())).await;
    assert!(
        st == StatusCode::PRECONDITION_REQUIRED || st == StatusCode::BAD_REQUEST,
        "no If-Match: {st}"
    );
    let (st, saved) = app.send_if_match("PUT", &path, v, body.clone()).await;
    assert_eq!(st, StatusCode::OK, "{saved}");
    assert!(saved["version"].as_i64().unwrap() > v);

    // The old version no longer matches.
    let (st, _) = app.send_if_match("PUT", &path, v, body).await;
    assert_eq!(st, StatusCode::PRECONDITION_FAILED);
}

#[tokio::test]
async fn test_case_crud_and_stale_version() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let (st, tc) = app
        .send(
            "POST",
            &format!("/features/{}/test-cases", t.feature),
            Some(json!({
                "name": "Lockout",
                "steps": [{ "action": "Enter a wrong password 3 times", "expected": "Locked" }]
            })),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "{tc}");
    let id = tc["id"].as_str().unwrap().to_owned();
    assert_eq!(tc["steps"].as_array().unwrap().len(), 1);

    let path = format!("/test-cases/{id}");
    let v = tc["version"].as_i64().unwrap();
    let edit = json!({
        "name": "Lockout after 3",
        "steps": [{ "action": "Fail 3 times", "expected": "Locked" }]
    });
    let (st, saved) = app.send_if_match("PUT", &path, v, edit.clone()).await;
    assert_eq!(st, StatusCode::OK, "{saved}");
    assert_eq!(saved["name"], "Lockout after 3");
    let (st, _) = app.send_if_match("PUT", &path, v, edit).await;
    assert_eq!(st, StatusCode::PRECONDITION_FAILED, "stale version");

    let (st, _) = app.send("DELETE", &path, None).await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (st, _) = app.send("GET", &path, None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn json_export_imports_into_another_project_with_the_same_shape() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    app.send(
        "POST",
        &format!("/features/{}/test-cases", t.feature),
        Some(json!({ "name": "Case", "steps": [{ "action": "a", "expected": "b" }] })),
    )
    .await;

    let (st, text) = app
        .get_text(&format!("/projects/{}/export?format=json", t.project))
        .await;
    assert_eq!(st, StatusCode::OK);
    let doc: Value = serde_json::from_str(&text).unwrap();

    let (_, target) = app
        .send("POST", "/projects", Some(json!({ "name": "Target" })))
        .await;
    let tid = target["id"].as_str().unwrap();
    let (st, counts) = app
        .send("POST", &format!("/projects/{tid}/import"), Some(doc))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{counts}");
    assert_eq!(counts["components"], 1);
    assert_eq!(counts["models"], 1);
    assert_eq!(counts["testCases"], 1);

    // Ids are fresh: re-exporting differs only in ids, not in counts.
    let (_, again) = app
        .get_text(&format!("/projects/{tid}/export?format=json"))
        .await;
    let again: Value = serde_json::from_str(&again).unwrap();
    assert_eq!(again["components"].as_array().unwrap().len(), 1);

    // Garbage is a 422, not a 500.
    let (st, _) = app
        .send(
            "POST",
            &format!("/projects/{tid}/import"),
            Some(json!({ "format": "other" })),
        )
        .await;
    assert!(st.is_client_error(), "{st}");
}
