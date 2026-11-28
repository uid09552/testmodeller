//! Upload-scoped API keys for CI (ADR 0018, FR-095). Requires Docker.

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tm_api::auth::AuthConfig;
use tm_api::secrets::{KeySource, SecretStore};
use tm_api::{auth_mode, router, AppState};
use tower::ServiceExt;

/// The same store behind a router that validates JWTs against a JWKS nobody can reach. A
/// `tm_` bearer is refused as "not a JWT" before any key is fetched, which is what we test.
fn jwt_router(app: &common::App) -> Router {
    let secrets = Arc::new(SecretStore::from_bytes(&[7u8; 32], KeySource::Configured).unwrap());
    let config = AuthConfig {
        jwks_url: "http://127.0.0.1:9/jwks".into(),
        tenant_claim: "tenant".into(),
        role_claim: "role".into(),
        issuer: None,
        audience: None,
        cache: Duration::from_secs(60),
        min_refresh: Duration::from_secs(60),
    };
    router(
        AppState::new(app.store.clone(), None, secrets),
        auth_mode(Some(config)),
    )
}

const JUNIT: &str = r#"<testsuites><testsuite name="s" timestamp="2026-11-01T10:00:00"><testcase name="Lockout"><failure message="x"/></testcase></testsuite></testsuites>"#;

async fn upload(router: &Router, path: &str, key: Option<&str>) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1{path}"))
        .header("content-type", "application/xml");
    if let Some(k) = key {
        req = req.header("authorization", format!("Bearer {k}"));
    }
    let res = router
        .clone()
        .oneshot(req.body(Body::from(JUNIT)).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn get_with(router: &Router, path: &str, key: &str) -> StatusCode {
    router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1{path}"))
                .header("authorization", format!("Bearer {key}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn an_upload_key_posts_results_for_its_project_and_nothing_else() {
    let app = common::app().await;
    let mine = common::tree(&app.store, "dev").await;
    let other = common::tree(&app.store, "dev").await;
    let jwt = jwt_router(&app);

    // Create an upload key for one project.
    let (st, created) = app
        .send(
            "POST",
            "/me/api-keys",
            Some(json!({ "name": "CI", "projectId": mine.project })),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "{created}");
    assert_eq!(created["projectId"], json!(mine.project));
    let key = created["key"].as_str().unwrap().to_owned();
    let (_, listed) = app.send("GET", "/me/api-keys", None).await;
    assert_eq!(listed[0]["projectId"], json!(mine.project));

    // Its own project: accepted, and the results are recorded (JWT mode: no user session).
    let path = format!("/projects/{}/test-results?format=junit", mine.project);
    let (st, report) = upload(&jwt, &path, Some(&key)).await;
    assert_eq!(st, StatusCode::OK, "{report}");
    assert_eq!(report["total"], 1);

    // Another project of the same tenant: refused.
    let (st, _) = upload(
        &jwt,
        &format!("/projects/{}/test-results?format=junit", other.project),
        Some(&key),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // No other route, not even a read of its own project, and not MCP.
    for p in [
        format!("/projects/{}", mine.project),
        format!("/projects/{}/export?format=json", mine.project),
        "/projects".to_owned(),
        "/me/api-keys".to_owned(),
    ] {
        assert_eq!(
            get_with(&jwt, &p, &key).await,
            StatusCode::UNAUTHORIZED,
            "{p}"
        );
    }
    let (st, _) = upload(&jwt, "/mcp", Some(&key)).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    // The wrong method on the upload route is not an upload either.
    assert_eq!(
        get_with(&jwt, &path, &key).await,
        StatusCode::UNAUTHORIZED,
        "only POST is an upload"
    );

    // Without a key, or with a revoked one, the JWT path applies and refuses.
    let (st, _) = upload(&jwt, &path, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (_, listed) = app.send("GET", "/me/api-keys", None).await;
    let id = listed[0]["id"].as_str().unwrap().to_owned();
    let (st, _) = app
        .send("DELETE", &format!("/me/api-keys/{id}"), None)
        .await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (st, _) = upload(&jwt, &path, Some(&key)).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "a revoked key stops working");
}

#[tokio::test]
async fn creating_an_upload_key_checks_role_and_tenant() {
    let app = common::app().await;
    // A project of another tenant: reads as not found, and no key is stored.
    let theirs = common::tree(&app.store, "someone-else").await;
    let (st, v) = app
        .send(
            "POST",
            "/me/api-keys",
            Some(json!({ "name": "CI", "projectId": theirs.project })),
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "{v}");
    let (_, listed) = app.send("GET", "/me/api-keys", None).await;
    assert_eq!(listed, json!([]));
    // A project that does not exist.
    let (st, _) = app
        .send(
            "POST",
            "/me/api-keys",
            Some(json!({ "name": "CI", "projectId": uuid::Uuid::new_v4() })),
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_upload_key_cannot_use_mcp_and_dies_with_its_project() {
    let app = common::app().await;
    let mine = common::tree(&app.store, "dev").await;
    let (_, created) = app
        .send(
            "POST",
            "/me/api-keys",
            Some(json!({ "name": "CI", "projectId": mine.project })),
        )
        .await;
    let key = created["key"].as_str().unwrap().to_owned();

    let res = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/mcp")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {key}"))
                .body(Body::from(
                    json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Deleting the project removes its upload keys.
    let (st, _) = app
        .send(
            "DELETE",
            &format!("/projects/{}?cascade=true", mine.project),
            None,
        )
        .await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (_, listed) = app.send("GET", "/me/api-keys", None).await;
    assert_eq!(listed, json!([]));
}
