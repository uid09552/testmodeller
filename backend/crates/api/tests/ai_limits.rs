//! Limits around AI work and request size (07-ai-integration.md). Requires Docker.

mod common;

use std::time::Duration;

use axum::http::StatusCode;
use serde_json::{json, Value};
use tokio::net::TcpListener;

/// An endpoint that accepts connections and never answers, so jobs keep running.
async fn silent_endpoint() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((sock, _)) = listener.accept().await {
            held.push(sock);
        }
    });
    base
}

/// An endpoint nothing listens on, so jobs fail at once.
async fn closed_endpoint() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    drop(listener);
    base
}

async fn configure(app: &common::App, base: &str) {
    let (s, v) = app
        .send(
            "PUT",
            "/settings/ai",
            Some(json!({ "provider": "openai-compatible", "baseUrl": base, "model": "m", "apiKey": "sk-1" })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{v}");
}

async fn request(app: &common::App, feature: uuid::Uuid) -> (StatusCode, Value) {
    app.send(
        "POST",
        "/ai/proposals",
        Some(json!({ "kind": "test-cases", "featureId": feature })),
    )
    .await
}

async fn settle(app: &common::App, job: &Value) {
    let path = format!("/jobs/{}", job["id"].as_str().unwrap());
    for _ in 0..200 {
        let (_, v) = app.send("GET", &path, None).await;
        if v["status"] == "failed" || v["status"] == "succeeded" {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("job {path} did not finish");
}

#[tokio::test]
async fn a_stored_key_never_follows_a_new_endpoint() {
    let app = common::app().await;
    configure(&app, "https://llm.example.com/v1").await;

    // Same endpoint, other model: the key stays.
    let (_, v) = app
        .send(
            "PUT",
            "/settings/ai",
            Some(json!({ "provider": "openai-compatible", "baseUrl": "https://llm.example.com/v1", "model": "other" })),
        )
        .await;
    assert_eq!(v["secretConfigured"], true, "{v}");

    // New endpoint without a key: the stored key is dropped.
    let (s, v) = app
        .send(
            "PUT",
            "/settings/ai",
            Some(json!({ "provider": "openai-compatible", "baseUrl": "https://attacker.example/v1", "model": "other" })),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["secretConfigured"], false, "{v}");

    // New endpoint with its own key: kept.
    configure(&app, "https://llm2.example.com/v1").await;
    let (_, v) = app.send("GET", "/settings/ai", None).await;
    assert_eq!(v["secretConfigured"], true, "{v}");
}

#[tokio::test]
async fn an_environment_key_cannot_follow_a_new_endpoint() {
    let app = common::app_with_ai_key(Some("sk-from-env")).await;
    let (s, v) = app
        .send(
            "PUT",
            "/settings/ai",
            Some(json!({ "provider": "openai-compatible", "baseUrl": "https://attacker.example/v1", "model": "m" })),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{v}");
    let (_, v) = app.send("GET", "/settings/ai", None).await;
    assert_ne!(v["baseUrl"], "https://attacker.example/v1", "{v}");
}

#[tokio::test]
async fn running_jobs_are_capped_per_tenant() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    configure(&app, &silent_endpoint().await).await;
    for _ in 0..4 {
        let (s, v) = request(&app, t.feature).await;
        assert_eq!(s, StatusCode::ACCEPTED, "{v}");
    }
    let (s, v) = request(&app, t.feature).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS, "{v}");
}

#[tokio::test]
async fn feature_jobs_are_rate_limited_per_project() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let other = common::tree(&app.store, "dev").await;
    configure(&app, &closed_endpoint().await).await;
    for _ in 0..10 {
        let (s, v) = request(&app, t.feature).await;
        assert_eq!(s, StatusCode::ACCEPTED, "{v}");
        settle(&app, &v).await;
    }
    let (s, v) = request(&app, t.feature).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS, "{v}");
    // Another project has its own budget.
    let (s, v) = request(&app, other.feature).await;
    assert_eq!(s, StatusCode::ACCEPTED, "{v}");
}

#[tokio::test]
async fn large_bodies_are_accepted_only_by_imports() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let big = "x".repeat(5 * 1024 * 1024);
    let (s, _) = app
        .send(
            "POST",
            &format!("/features/{}/move", t.feature),
            Some(json!({ "componentId": t.component, "pad": big })),
        )
        .await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);
    let (s, _) = app
        .send_raw(
            "POST",
            &format!("/projects/{}/test-results?format=junit", t.project),
            Some(("application/xml", big.into_bytes())),
        )
        .await;
    assert_ne!(s, StatusCode::PAYLOAD_TOO_LARGE);
}
