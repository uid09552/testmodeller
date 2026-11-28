//! Comparing two versions of a model (FR-014). Requires Docker.

mod common;

use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn compares_versions_and_the_current_model() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let path = format!("/models/{}", t.model);
    let (_, m) = app.send("GET", &path, None).await;
    let v1 = m["version"].as_i64().unwrap();

    // Version 2: rename a state and drop the last transition.
    let mut states = m["states"].clone();
    states[1]["name"] = json!("Out renamed");
    let mut transitions = m["transitions"].clone();
    transitions.as_array_mut().unwrap().pop();
    let (st, saved) = app
        .send_if_match(
            "PUT",
            &path,
            v1,
            json!({ "name": "Renamed", "variables": m["variables"], "states": states, "transitions": transitions }),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{saved}");
    let v2 = saved["version"].as_i64().unwrap();

    let (st, c) = app
        .send(
            "GET",
            &format!("{path}/versions/compare?from={v1}&to={v2}"),
            None,
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{c}");
    assert_eq!(c["meta"]["fields"], json!(["name"]));
    assert_eq!(c["states"]["changed"][0]["fields"], json!(["name"]));
    assert_eq!(c["transitions"]["removed"].as_array().unwrap().len(), 1);
    assert!(c["states"]["added"].as_array().unwrap().is_empty());

    // Without `to` it compares with the current model, which equals v2.
    let (_, cur) = app
        .send("GET", &format!("{path}/versions/compare?from={v2}"), None)
        .await;
    assert!(cur["transitions"]["removed"].as_array().unwrap().is_empty());
    let (st, _) = app
        .send("GET", &format!("{path}/versions/compare?from={v1}"), None)
        .await;
    assert_eq!(st, StatusCode::OK);

    // Same version twice, and an unknown one.
    let (st, _) = app
        .send(
            "GET",
            &format!("{path}/versions/compare?from={v1}&to={v1}"),
            None,
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, _) = app
        .send("GET", &format!("{path}/versions/compare?from=99"), None)
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Another tenant's model is not found.
    let other = common::tree(&app.store, "someone-else").await;
    let (st, _) = app
        .send(
            "GET",
            &format!("/models/{}/versions/compare?from=1", other.model),
            None,
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}
