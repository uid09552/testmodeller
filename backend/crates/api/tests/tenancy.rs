//! Ids in a request body are tenant-checked like ids in the path (FR-044).
//!
//! The tenant guard sees only path ids, so every route that names a second
//! entity in its body checks it in the handler. Each test aims one of those
//! routes at an entity of another tenant and expects 404 and no change.

mod common;

use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn body_ids_of_another_tenant_are_not_found() {
    let app = common::app().await;
    let mine = common::tree(&app.store, "dev").await;
    let theirs = common::tree(&app.store, "someone-else").await;

    // Move a feature into another tenant's component.
    let (status, body) = app
        .send(
            "POST",
            &format!("/features/{}/move", mine.feature),
            Some(json!({ "componentId": theirs.component })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let feature = app.store.get_feature(mine.feature).await.unwrap();
    assert_eq!(feature.component_id, mine.component);

    // Move a test case into another tenant's feature.
    let (status, body) = app
        .send(
            "POST",
            &format!("/features/{}/test-cases", mine.feature),
            Some(json!({ "name": "T", "steps": [{ "action": "a", "expected": "b" }] })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let case = body["id"].as_str().unwrap().to_owned();
    let (status, body) = app
        .send(
            "POST",
            &format!("/test-cases/{case}/move"),
            Some(json!({ "featureId": theirs.feature })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let (_, body) = app.send("GET", &format!("/test-cases/{case}"), None).await;
    assert_eq!(body["featureId"], json!(mine.feature));

    // Duplicate a model into another tenant's feature.
    let (status, body) = app
        .send(
            "POST",
            &format!("/models/{}/duplicate", mine.model),
            Some(json!({ "featureId": theirs.feature })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let models = app.store.list_models(theirs.feature).await.unwrap();
    assert_eq!(models.len(), 1, "nothing was copied into their feature");
}

#[tokio::test]
async fn body_ids_of_the_own_tenant_still_work() {
    let app = common::app().await;
    let mine = common::tree(&app.store, "dev").await;
    let other = common::tree(&app.store, "dev").await;

    let (status, body) = app
        .send(
            "POST",
            &format!("/features/{}/move", mine.feature),
            Some(json!({ "componentId": other.component })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = app
        .send(
            "POST",
            &format!("/models/{}/duplicate", other.model),
            Some(json!({ "featureId": mine.feature })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}
