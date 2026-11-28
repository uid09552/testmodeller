//! Playwright skeleton export (FR-024, ADR 0020). Requires Docker.

mod common;

use axum::http::StatusCode;
use serde_json::json;
use tm_api::results_import::find_tag;

#[tokio::test]
async fn skeleton_carries_the_tag_and_escapes_user_text() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let nasty = "Pay \"now\" `${process.exit(1)}` \\ \u{2028} */ // x";
    let (st, tc) = app
        .send(
            "POST",
            &format!("/features/{}/test-cases", t.feature),
            Some(json!({
                "name": nasty,
                "preconditions": "line one\nline two",
                "steps": [{ "action": "a\"); evil(); (\"", "expected": "ok\n// hi" }]
            })),
        )
        .await;
    assert_eq!(st, StatusCode::CREATED, "{tc}");
    let id: uuid::Uuid = tc["id"].as_str().unwrap().parse().unwrap();

    let (st, ts) = app
        .get_text(&format!("/projects/{}/export?format=playwright", t.project))
        .await;
    assert_eq!(st, StatusCode::OK);
    assert!(ts.contains("import { test } from '@playwright/test';"));
    // The tag survives into the title, where the result importer finds it.
    let title = ts
        .lines()
        .find(|l| l.trim_start().starts_with("test(\""))
        .unwrap();
    assert_eq!(find_tag(title), Some(id));
    // User text is only ever inside JSON string literals: no raw quote, backtick or newline.
    assert!(title.contains(r#"Pay \"now\" `${process.exit(1)}` \\ "#));
    assert!(ts.contains(r#"test.step("a\"); evil(); (\"""#));
    assert!(ts.contains(r#"const expected = "ok // hi";"#));
    assert!(!ts.contains("\n// hi"));

    // Scoped like the other formats: another tenant's project is not found.
    let other = common::tree(&app.store, "someone-else").await;
    let (st, _) = app
        .get_text(&format!(
            "/projects/{}/export?format=playwright",
            other.project
        ))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}
