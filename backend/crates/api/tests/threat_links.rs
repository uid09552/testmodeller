//! Feature test cases that cover threats, and security tests converted into
//! them (ADR 0016, FR-093, FR-094). Requires Docker.

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};

async fn post(app: &common::App, path: &str, body: Value) -> Value {
    let (s, v) = app.send("POST", path, Some(body)).await;
    assert!(
        s == StatusCode::CREATED || s == StatusCode::OK,
        "{path}: {s} {v}"
    );
    v
}

fn id(v: &Value) -> String {
    v["id"].as_str().unwrap().to_owned()
}

/// A project with a feature and a threat model holding one open threat.
struct Setup {
    t: common::Tree,
    base: String,
    threat: String,
}

async fn setup(app: &common::App, tenant: &str) -> Setup {
    let t = common::tree(&app.store, tenant).await;
    let tm = id(&post(
        app,
        &format!("/projects/{}/threat-model", t.project),
        json!({"name": "TM"}),
    )
    .await);
    let base = format!("/threat-models/{tm}");
    let api = id(&post(
        app,
        &format!("{base}/components"),
        json!({"name": "API", "type": "api"}),
    )
    .await);
    let threat = id(&post(
        app,
        &format!("{base}/threats"),
        json!({"title": "Account takeover", "category": "spoofing",
               "targets": [{"kind": "component", "id": api}]}),
    )
    .await);
    Setup { t, base, threat }
}

fn junit(name: &str, failed: bool, at: &str) -> String {
    let case = if failed {
        format!(r#"<testcase name="{name}"><failure message="locked out"/></testcase>"#)
    } else {
        format!(r#"<testcase name="{name}"/>"#)
    };
    format!(r#"<testsuites><testsuite name="s" timestamp="{at}">{case}</testsuite></testsuites>"#)
}

#[tokio::test]
async fn a_test_case_covers_threats_and_its_failures_reach_the_threat() {
    let app = common::app().await;
    let s = setup(&app, "dev").await;

    let tc = post(
        &app,
        &format!("/features/{}/test-cases", s.t.feature),
        json!({"name": "Lockout after three failures", "steps": [{"action": "a", "expected": "b"}],
               "threatIds": [s.threat, s.threat]}),
    )
    .await;
    assert_eq!(tc["threatIds"], json!([s.threat]), "duplicates are dropped");
    let tc_id = id(&tc);

    // Read back on the threat side, and counted in the summary.
    let (_, threat) = app
        .send("GET", &format!("{}/threats/{}", s.base, s.threat), None)
        .await;
    assert_eq!(threat["testCaseIds"], json!([tc_id]));
    assert!(!threat["flags"]
        .as_array()
        .unwrap()
        .contains(&json!("failing-test")));
    let (_, summary) = app.send("GET", &format!("{}/summary", s.base), None).await;
    assert_eq!(summary["threatsCoveredByTestCases"], 1);

    // A failing CI run raises the threat's failing-test flag; a later pass clears it.
    let import = |file: String| {
        let app = &app;
        let project = s.t.project;
        async move {
            app.send_raw(
                "POST",
                &format!("/projects/{project}/test-results?format=junit"),
                Some(("application/xml", file.into_bytes())),
            )
            .await
        }
    };
    let (st, v) = import(junit(
        "Lockout after three failures",
        true,
        "2026-11-01T10:00:00",
    ))
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let (_, threat) = app
        .send("GET", &format!("{}/threats/{}", s.base, s.threat), None)
        .await;
    assert!(
        threat["flags"]
            .as_array()
            .unwrap()
            .contains(&json!("failing-test")),
        "{threat}"
    );
    import(junit(
        "Lockout after three failures",
        false,
        "2026-11-02T10:00:00",
    ))
    .await;
    let (_, threat) = app
        .send("GET", &format!("{}/threats/{}", s.base, s.threat), None)
        .await;
    assert!(
        !threat["flags"]
            .as_array()
            .unwrap()
            .contains(&json!("failing-test")),
        "{threat}"
    );

    // The Gherkin export tags the scenario with the threat's display id.
    let (_, gherkin) = app
        .get_text(&format!("/projects/{}/export?format=gherkin", s.t.project))
        .await;
    assert!(
        gherkin.contains(&format!("  @tm-{tc_id} @THR-1\n")),
        "{gherkin}"
    );

    // Omitting threatIds keeps the links; an empty list clears them.
    let (_, current) = app.send("GET", &format!("/test-cases/{tc_id}"), None).await;
    let version = current["version"].as_i64().unwrap();
    let (st, v) = app
        .send_if_match(
            "PUT",
            &format!("/test-cases/{tc_id}"),
            version,
            json!({"name": "Lockout", "steps": []}),
        )
        .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["threatIds"], json!([s.threat]));
    let (_, v) = app
        .send_if_match(
            "PUT",
            &format!("/test-cases/{tc_id}"),
            version + 1,
            json!({"name": "Lockout", "steps": [], "threatIds": []}),
        )
        .await;
    assert_eq!(v["threatIds"], json!([]));
    let (_, threat) = app
        .send("GET", &format!("{}/threats/{}", s.base, s.threat), None)
        .await;
    assert_eq!(threat["testCaseIds"], json!([]));
}

#[tokio::test]
async fn only_threats_of_the_same_project_can_be_covered() {
    let app = common::app().await;
    let mine = setup(&app, "dev").await;
    let other_project = setup(&app, "dev").await;
    let (st, v) = app
        .send(
            "POST",
            &format!("/features/{}/test-cases", mine.t.feature),
            Some(json!({"name": "x", "steps": [], "threatIds": [other_project.threat]})),
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{v}");

    // Moving a covering test case to another project drops the link.
    let tc = id(&post(
        &app,
        &format!("/features/{}/test-cases", mine.t.feature),
        json!({"name": "x", "steps": [], "threatIds": [mine.threat]}),
    )
    .await);
    let moved = post(
        &app,
        &format!("/test-cases/{tc}/move"),
        json!({"featureId": other_project.t.feature}),
    )
    .await;
    assert_eq!(moved["threatIds"], json!([]));
}

#[tokio::test]
async fn a_security_test_converts_into_a_covering_test_case_once() {
    let app = common::app().await;
    let s = setup(&app, "dev").await;
    let sec = id(&post(
        &app,
        &format!("{}/security-tests", s.base),
        json!({"title": "Brute force is locked out", "objective": "No password guessing",
               "preconditions": ["A user exists", "Lockout is configured"],
               "steps": [{"action": "Fail login three times", "expected": "Account is locked"}],
               "type": "authentication", "status": "ready", "threatIds": [s.threat]}),
    )
    .await);

    // A feature of another project is refused, and nothing changes.
    let elsewhere = setup(&app, "dev").await;
    let (st, v) = app
        .send(
            "POST",
            &format!("{}/security-tests/{sec}/convert", s.base),
            Some(json!({"featureId": elsewhere.t.feature})),
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{v}");
    // A model of another feature is refused.
    let (st, _) = app
        .send(
            "POST",
            &format!("{}/security-tests/{sec}/convert", s.base),
            Some(json!({"featureId": s.t.feature, "modelId": elsewhere.t.model})),
        )
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    // Another tenant's feature is not found.
    let foreign = common::tree(&app.store, "someone-else").await;
    let (st, _) = app
        .send(
            "POST",
            &format!("{}/security-tests/{sec}/convert", s.base),
            Some(json!({"featureId": foreign.feature})),
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    let tc = post(
        &app,
        &format!("{}/security-tests/{sec}/convert", s.base),
        json!({"featureId": s.t.feature, "modelId": s.t.model}),
    )
    .await;
    assert_eq!(tc["name"], "Brute force is locked out");
    assert_eq!(tc["description"], "No password guessing");
    assert_eq!(tc["preconditions"], "A user exists\nLockout is configured");
    assert_eq!(tc["steps"][0]["action"], "Fail login three times");
    assert_eq!(tc["tags"], json!(["security"]));
    assert_eq!(tc["origin"], "manual");
    assert_eq!(tc["threatIds"], json!([s.threat]));
    // Assigned to the model's initial state, so the editor lists it.
    assert_eq!(tc["assignments"].as_array().unwrap().len(), 1);
    assert_eq!(
        tc["assignments"][0]["stateId"],
        json!(s.t.graph.states[0].id)
    );

    let (_, test) = app
        .send("GET", &format!("{}/security-tests/{sec}", s.base), None)
        .await;
    assert_eq!(test["status"], "obsolete");
    assert_eq!(test["convertedToTestCaseId"], tc["id"]);

    let (st, _) = app
        .send(
            "POST",
            &format!("{}/security-tests/{sec}/convert", s.base),
            Some(json!({"featureId": s.t.feature})),
        )
        .await;
    assert_eq!(st, StatusCode::CONFLICT);
}

/// AGENTS.md rule 10: every body id is tenant-checked, even where storage
/// would also refuse it. Another tenant's ids read as not found.
#[tokio::test]
async fn ids_of_another_tenant_are_not_found() {
    let app = common::app().await;
    let mine = setup(&app, "dev").await;
    // Built through the API as "dev", then handed to another tenant: the API
    // itself would (rightly) not let "dev" build in someone else's project.
    let theirs = setup(&app, "dev").await;
    let pool = sqlx::PgPool::connect(&app.url).await.unwrap();
    sqlx::query("UPDATE projects SET tenant_id = 'someone-else' WHERE id = $1")
        .bind(theirs.t.project)
        .execute(&pool)
        .await
        .unwrap();

    // threatIds on create.
    let (st, v) = app
        .send(
            "POST",
            &format!("/features/{}/test-cases", mine.t.feature),
            Some(json!({"name": "x", "steps": [], "threatIds": [theirs.threat]})),
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "{v}");

    // threatIds on replace; the test case keeps its links.
    let tc = post(
        &app,
        &format!("/features/{}/test-cases", mine.t.feature),
        json!({"name": "x", "steps": [], "threatIds": [mine.threat]}),
    )
    .await;
    let (st, v) = app
        .send_if_match(
            "PUT",
            &format!("/test-cases/{}", id(&tc)),
            tc["version"].as_i64().unwrap(),
            json!({"name": "x", "steps": [], "threatIds": [theirs.threat]}),
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "{v}");
    let (_, v) = app
        .send("GET", &format!("/test-cases/{}", id(&tc)), None)
        .await;
    assert_eq!(v["threatIds"], json!([mine.threat]));

    // modelId on convert.
    let sec = id(&post(
        &app,
        &format!("{}/security-tests", mine.base),
        json!({"title": "x", "type": "other", "status": "ready", "threatIds": [mine.threat]}),
    )
    .await);
    let (st, v) = app
        .send(
            "POST",
            &format!("{}/security-tests/{sec}/convert", mine.base),
            Some(json!({"featureId": mine.t.feature, "modelId": theirs.t.model})),
        )
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "{v}");
    let (_, test) = app
        .send("GET", &format!("{}/security-tests/{sec}", mine.base), None)
        .await;
    assert_eq!(test["status"], "ready", "nothing was converted");
}
