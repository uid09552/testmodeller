//! `GET /models/{modelId}/stale-tests` end to end. Requires Docker.

mod common;

use axum::http::StatusCode;
use serde_json::json;

/// Counts rows, to show a read changed nothing.
async fn counts(app: &common::App, model: uuid::Uuid) -> (i64, i64) {
    let (_, tcs) = app
        .send("GET", &format!("/models/{model}/test-cases"), None)
        .await;
    let cases = tcs.as_array().unwrap();
    let assignments: usize = cases
        .iter()
        .map(|c| c["assignments"].as_array().unwrap().len())
        .sum();
    (cases.len() as i64, assignments as i64)
}

#[tokio::test]
async fn reports_stale_generated_tests_and_changes_nothing() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;

    let (status, generated) = app
        .send(
            "POST",
            &format!("/models/{}/generate", t.model),
            Some(json!({ "criterion": "transition", "seed": 1, "save": true })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(generated["testCases"].as_array().unwrap().len(), 1);
    let generated_id = generated["testCases"][0]["id"].as_str().unwrap().to_owned();

    // A manual test case on the same transition is never checked.
    let login = t.graph.transitions[1].id;
    let (status, _) = app
        .send(
            "POST",
            &format!("/features/{}/test-cases", t.feature),
            Some(json!({
                "name": "manual", "steps": [{ "action": "a", "expected": "b" }],
                "assignments": [{ "modelId": t.model, "transitionId": login, "stepOrder": 1 }],
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);

    let path = format!("/models/{}/stale-tests", t.model);
    let (status, body) = app.send("GET", &path, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));

    // Deleting the login transition orphans step 2 of the generated test.
    let (status, _) = app
        .send("DELETE", &format!("/transitions/{login}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let before = counts(&app, t.model).await;
    let (status, body) = app.send("GET", &path, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!([{
            "testCaseId": generated_id,
            "reasons": [{
                "code": "STEP_UNASSIGNED", "stepOrder": 2,
                "message": "step 2 lost its transition; it was deleted from the model",
            }],
        }])
    );
    assert_eq!(counts(&app, t.model).await, before);
}

#[tokio::test]
async fn another_tenants_model_is_not_found() {
    let app = common::app().await;
    let theirs = common::tree(&app.store, "someone-else").await;
    let (status, _) = app
        .send(
            "GET",
            &format!("/models/{}/stale-tests", theirs.model),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
