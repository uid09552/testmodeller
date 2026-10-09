//! Test result import end to end (FR-050..FR-053). Requires Docker.

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use uuid::Uuid;

/// Creates a manual test case assigned to the given transition of the tree's model.
async fn case(app: &common::App, t: &common::Tree, name: &str, transition: usize) -> String {
    let (status, body) = app
        .send(
            "POST",
            &format!("/features/{}/test-cases", t.feature),
            Some(json!({
                "name": name, "steps": [{ "action": "a", "expected": "b" }],
                "assignments": [{ "modelId": t.model, "transitionId": t.graph.transitions[transition].id }],
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_str().unwrap().to_owned()
}

async fn import(
    app: &common::App,
    project: Uuid,
    format: &str,
    dry: bool,
    file: &str,
) -> (StatusCode, Value) {
    app.send_raw(
        "POST",
        &format!("/projects/{project}/test-results?format={format}&dryRun={dry}"),
        Some(("application/xml", file.as_bytes().to_vec())),
    )
    .await
}

fn junit(timestamp: &str, cases: &[(&str, Option<&str>)]) -> String {
    let body: String = cases
        .iter()
        .map(|(name, failure)| match failure {
            None => format!(r#"<testcase name="{name}" time="0.5"/>"#),
            Some(msg) => {
                format!(r#"<testcase name="{name}"><failure message="{msg}"/></testcase>"#)
            }
        })
        .collect();
    format!(
        r#"<testsuites><testsuite name="s" timestamp="{timestamp}">{body}</testsuite></testsuites>"#
    )
}

#[tokio::test]
async fn imports_matches_records_and_reports() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let login = case(&app, &t, "Valid login", 0).await;
    let close = case(&app, &t, "Close", 2).await;
    case(&app, &t, "Twice", 1).await;
    case(&app, &t, "Twice", 1).await;
    // Another tenant's test case: its tag must not match here.
    let other = common::tree(&app.store, "someone-else").await;
    let foreign: Uuid = app
        .store
        .create_test_case(
            other.feature,
            &tm_domain::TestCaseData {
                name: "Valid login".into(),
                description: None,
                preconditions: None,
                priority: None,
                status: tm_domain::TestCaseStatus::Draft,
                tags: vec![],
                steps: vec![],
                implementation_url: None,
                backlog_url: None,
            },
            tm_domain::Origin::Manual,
            &[],
        )
        .await
        .unwrap()
        .audit
        .id;

    let file = junit(
        "2026-10-01T09:00:00",
        &[
            (&format!("renamed in the runner @tm-{close}"), None),
            ("Valid login", Some("expected home")),
            ("Twice", None),
            ("Nobody", None),
            (&format!("Foreign @tm-{foreign}"), None),
        ],
    );

    // Dry run: matches, records nothing.
    let (status, report) = import(&app, t.project, "junit", true, &file).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["dryRun"], json!(true));
    assert_eq!(report["recorded"], json!(0));
    assert_eq!(report["matched"], json!(2));
    let (_, tc) = app.send("GET", &format!("/test-cases/{login}"), None).await;
    assert!(tc.get("lastResult").is_none());

    let (status, report) = import(&app, t.project, "junit", false, &file).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        report,
        json!({
            "dryRun": false, "total": 5, "matched": 2, "recorded": 2, "duplicates": 0,
            "unmatched": ["Nobody", format!("Foreign @tm-{foreign}")],
            "ambiguous": ["Twice"],
        })
    );

    // Matched by name, failed.
    let (_, tc) = app.send("GET", &format!("/test-cases/{login}"), None).await;
    assert_eq!(tc["lastResult"]["status"], json!("failed"));
    assert_eq!(tc["lastResult"]["message"], json!("expected home"));
    assert_eq!(
        tc["lastResult"]["executedAt"],
        json!("2026-10-01T09:00:00Z")
    );
    // Matched by tag regardless of name.
    let (_, tc) = app.send("GET", &format!("/test-cases/{close}"), None).await;
    assert_eq!(tc["lastResult"]["status"], json!("passed"));
    assert_eq!(tc["lastResult"]["durationMs"], json!(500));

    // Nothing recorded for the other tenant.
    let foreign_results = app.store.test_results(foreign, 10).await.unwrap();
    assert!(foreign_results.is_empty());

    // Same file again: nothing new.
    let (_, report) = import(&app, t.project, "junit", false, &file).await;
    assert_eq!(report["recorded"], json!(0));
    assert_eq!(report["duplicates"], json!(2));

    // A newer run becomes the latest; the older stays in the history.
    let newer = junit("2026-10-02T09:00:00", &[("Valid login", None)]);
    let (_, report) = import(&app, t.project, "junit", false, &newer).await;
    assert_eq!(report["recorded"], json!(1));
    let (_, tc) = app.send("GET", &format!("/test-cases/{login}"), None).await;
    assert_eq!(tc["lastResult"]["status"], json!("passed"));
    let (status, history) = app
        .send("GET", &format!("/test-cases/{login}/results"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let statuses: Vec<&str> = history
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, ["passed", "failed"]);

    // Passing coverage: transition 0 (login, now passing) and 2 (close, passing) are
    // passing; transition 1 is covered by "Twice" tests without results.
    let (_, cov) = app
        .send("GET", &format!("/models/{}/coverage", t.model), None)
        .await;
    assert_eq!(
        cov["transitions"],
        json!({ "covered": 3, "total": 3, "passing": 2 })
    );

    // The traceability matrix carries the latest result too.
    let (_, trace) = app
        .send(
            "GET",
            &format!("/projects/{}/traceability", t.project),
            None,
        )
        .await;
    let row = trace["untraced"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == json!(login))
        .unwrap()
        .clone();
    assert_eq!(row["lastResult"]["status"], json!("passed"));
}

#[tokio::test]
async fn cucumber_import_by_tag() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let id = case(&app, &t, "Login", 0).await;
    let file = json!([{ "name": "F", "elements": [{
        "type": "scenario", "name": "Something else",
        "tags": [{ "name": format!("@tm-{id}") }],
        "steps": [
            { "result": { "status": "passed", "duration": 1_000_000 } },
            { "result": { "status": "failed", "error_message": "boom" } },
        ],
    }]}])
    .to_string();
    let (status, report) = import(&app, t.project, "cucumber", false, &file).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["recorded"], json!(1));
    let (_, tc) = app.send("GET", &format!("/test-cases/{id}"), None).await;
    assert_eq!(tc["lastResult"]["status"], json!("failed"));
    assert_eq!(tc["lastResult"]["source"], json!("cucumber"));
}

#[tokio::test]
async fn rejects_bad_files_and_records_nothing() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let id = case(&app, &t, "Login", 0).await;

    let (status, body) = import(&app, t.project, "junit", false, "<testsuite><testcase").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["detail"].as_str().unwrap().contains("cannot import"));

    let lol = r#"<?xml version="1.0"?><!DOCTYPE l [<!ENTITY a "aaaa">]><testsuite><testcase name="&a;"/></testsuite>"#;
    let (status, body) = import(&app, t.project, "junit", false, lol).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["detail"].as_str().unwrap().contains("DOCTYPE"));

    let (status, _) = import(&app, t.project, "trx", false, "<x/>").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let big = format!("<testsuite>{}</testsuite>", " ".repeat(11 * 1024 * 1024));
    let (status, _) = import(&app, t.project, "junit", false, &big).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);

    let (_, history) = app
        .send("GET", &format!("/test-cases/{id}/results"), None)
        .await;
    assert_eq!(history, json!([]));
}

#[tokio::test]
async fn another_tenants_project_and_history_are_not_found() {
    let app = common::app().await;
    let theirs = common::tree(&app.store, "someone-else").await;
    let (status, _) = import(
        &app,
        theirs.project,
        "junit",
        false,
        &junit("2026-01-01T00:00:00", &[]),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn exports_carry_the_tag() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let id = case(&app, &t, "Login", 0).await;
    let (status, gherkin) = app
        .get_text(&format!("/projects/{}/export?format=gherkin", t.project))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        gherkin.contains(&format!("  @tm-{id}\n  Scenario: Login")),
        "{gherkin}"
    );
    let (_, csv) = app
        .get_text(&format!("/projects/{}/export?format=csv", t.project))
        .await;
    let mut lines = csv.lines();
    assert!(lines.next().unwrap().ends_with(",expected,tmId"));
    assert!(lines.next().unwrap().ends_with(&format!(",@tm-{id}")));
}
