//! Residual risk, evidence, acceptance expiry, baselines and detail fields over
//! HTTP (FR-077..FR-084). Requires Docker.

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use tm_storage::NamedFields;

use common::App;

async fn project(app: &App, tenant: &str) -> String {
    app.store
        .create_project(
            tenant,
            NamedFields {
                name: "P".into(),
                description: None,
            },
        )
        .await
        .unwrap()
        .audit
        .id
        .to_string()
}

async fn post(app: &App, path: &str, body: Value) -> Value {
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

fn in_days(n: i64) -> String {
    (chrono::Utc::now().date_naive() + chrono::Duration::days(n)).to_string()
}

struct Model {
    base: String,
    web: String,
    api: String,
    flow: String,
}

async fn model(app: &App) -> Model {
    let p = project(app, "dev").await;
    let tm = id(&post(
        app,
        &format!("/projects/{p}/threat-model"),
        json!({"name": "Shop"}),
    )
    .await);
    let base = format!("/threat-models/{tm}");
    let internet = id(&post(
        app,
        &format!("{base}/zones"),
        json!({"name": "Internet", "kind": "internet", "trustLevel": 0}),
    )
    .await);
    let internal = id(&post(
        app,
        &format!("{base}/zones"),
        json!({"name": "Internal", "kind": "internal", "trustLevel": 3}),
    )
    .await);
    let web = id(&post(
        app,
        &format!("{base}/components"),
        json!({"name": "Web", "type": "web-frontend", "zoneId": internet}),
    )
    .await);
    let api = id(&post(
        app,
        &format!("{base}/components"),
        json!({"name": "API", "type": "api", "zoneId": internal, "owner": "team"}),
    )
    .await);
    let flow = id(&post(
        app,
        &format!("{base}/flows"),
        json!({"sourceId": api, "targetId": web, "relationship": "sends-data"}),
    )
    .await);
    Model {
        base,
        web,
        api,
        flow,
    }
}

fn threat_body(target: &str, extra: Value) -> Value {
    let mut b = json!({
        "title": "Spoofed session", "category": "spoofing",
        "targets": [{"kind": "component", "id": target}],
        "inherent": {"likelihood": 4, "impact": 5}
    });
    for (k, v) in extra.as_object().unwrap() {
        b[k] = v.clone();
    }
    b
}

#[tokio::test]
async fn residual_rating_flags_and_matrix() {
    let app = common::app().await;
    let m = model(&app).await;
    let t = post(
        &app,
        &format!("{}/threats", m.base),
        threat_body(
            &m.api,
            json!({"current": {"likelihood": 3, "impact": 4}, "residual": {"likelihood": 2, "impact": 3},
                   "threatSource": "malicious tenant admin", "confidence": "medium"}),
        ),
    )
    .await;
    assert_eq!(t["residualScore"], 6);
    assert_eq!(t["residualLevel"], "moderate");
    assert_eq!(t["threatSource"], "malicious tenant admin");
    assert_eq!(t["confidence"], "medium");
    // Residual below current with no verified control.
    assert!(t["flags"]
        .as_array()
        .unwrap()
        .contains(&json!("unsupported-residual")));
    // Out of range.
    let (s, v) = app
        .send(
            "POST",
            &format!("{}/threats", m.base),
            Some(threat_body(
                &m.api,
                json!({"residual": {"likelihood": 2, "impact": 0}}),
            )),
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(v["errors"][0]["field"], "residual");

    let (_, summary) = app.send("GET", &format!("{}/summary", m.base), None).await;
    let cell = summary["residualMatrix"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["likelihood"] == 2 && c["impact"] == 3)
        .unwrap();
    assert_eq!(cell["count"], 1);
    // Unrated residual is not in any cell.
    post(
        &app,
        &format!("{}/threats", m.base),
        threat_body(&m.web, json!({})),
    )
    .await;
    let (_, summary) = app.send("GET", &format!("{}/summary", m.base), None).await;
    let total: i64 = summary["residualMatrix"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["count"].as_i64().unwrap())
        .sum();
    assert_eq!(total, 1);
}

#[tokio::test]
async fn acceptance_needs_an_expiry_and_lapses_to_open() {
    let app = common::app().await;
    let m = model(&app).await;
    let accept = |expires: Value| {
        threat_body(
            &m.api,
            json!({"treatment": "accept", "acceptanceRationale": "low exposure",
                   "acceptedBy": "J. Doe", "acceptanceExpires": expires}),
        )
    };
    for bad in [Value::Null, json!(in_days(-1)), json!(in_days(4000))] {
        let (s, v) = app
            .send("POST", &format!("{}/threats", m.base), Some(accept(bad)))
            .await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(v["errors"][0]["field"], "acceptanceExpires");
    }
    let t = post(
        &app,
        &format!("{}/threats", m.base),
        accept(json!(in_days(20))),
    )
    .await;
    assert_eq!(t["status"], "accepted");
    assert_eq!(t["effectiveStatus"], "accepted");
    assert_eq!(t["acceptedOn"], chrono::Utc::now().date_naive().to_string());
    let (_, summary) = app.send("GET", &format!("{}/summary", m.base), None).await;
    assert_eq!(summary["expiringAcceptances"].as_array().unwrap().len(), 1);

    // A lapsed acceptance reads as open with the flag (set directly in the
    // store: the API refuses a past expiry).
    let pool = sqlx::PgPool::connect(&app.url).await.unwrap();
    sqlx::query("UPDATE tm_threats SET acceptance_expires = current_date - 1 WHERE id = $1::uuid")
        .bind(id(&t))
        .execute(&pool)
        .await
        .unwrap();
    let (_, got) = app
        .send("GET", &format!("{}/threats/{}", m.base, id(&t)), None)
        .await;
    assert_eq!(got["status"], "accepted");
    assert_eq!(got["effectiveStatus"], "open");
    assert!(got["flags"]
        .as_array()
        .unwrap()
        .contains(&json!("acceptance-expired")));
    assert_eq!(got["acceptanceRationale"], "low exposure");
    let (_, summary) = app.send("GET", &format!("{}/summary", m.base), None).await;
    assert_eq!(summary["expiredAcceptances"], 1);
    assert_eq!(summary["threatsByStatus"]["open"], 1);
}

#[tokio::test]
async fn verified_needs_evidence_and_losing_it_downgrades() {
    let app = common::app().await;
    let m = model(&app).await;
    let control = |status: &str| json!({"name": "MFA", "type": "preventive", "category": "identity-access", "status": status});
    // A new control cannot start verified.
    let (s, v) = app
        .send(
            "POST",
            &format!("{}/controls", m.base),
            Some(control("verified")),
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(v["errors"][0]["field"], "status");
    let c = post(
        &app,
        &format!("{}/controls", m.base),
        control("implemented"),
    )
    .await;
    let cid = id(&c);
    let path = format!("{}/controls/{cid}", m.base);

    let (s, v) = app
        .send_if_match("PUT", &path, 1, control("verified"))
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{v}");

    // Evidence needs a link or a note.
    let (s, _) = app
        .send(
            "POST",
            &format!("{path}/evidence"),
            Some(json!({"kind": "test-report", "title": "Pen test", "evidenceDate": "2026-10-01"})),
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    let e = post(
        &app,
        &format!("{path}/evidence"),
        json!({"kind": "test-report", "title": "Pen test", "link": "https://x/report",
               "evidenceDate": "2026-10-01", "reviewer": "R"}),
    )
    .await;
    assert_eq!(e["ownerKind"], "control");

    let mut body = control("verified");
    body["reviewBy"] = json!(in_days(-1));
    let (s, v) = app.send_if_match("PUT", &path, 1, body).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["status"], "verified");
    assert!(v["verifiedAt"].is_string());
    assert_eq!(v["evidenceCount"], 1);
    assert!(v["flags"]
        .as_array()
        .unwrap()
        .contains(&json!("verification-overdue")));
    let (_, summary) = app.send("GET", &format!("{}/summary", m.base), None).await;
    assert_eq!(summary["verificationOverdue"], 1);

    let (s, _) = app
        .send("DELETE", &format!("{}/evidence/{}", m.base, id(&e)), None)
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (_, got) = app.send("GET", &path, None).await;
    assert_eq!(got["status"], "implemented");
    assert_eq!(got["evidenceCount"], 0);
}

#[tokio::test]
async fn evidence_of_a_test_result_and_of_foreign_controls() {
    let app = common::app().await;
    let m = model(&app).await;
    let t = id(&post(
        &app,
        &format!("{}/threats", m.base),
        threat_body(&m.api, json!({})),
    )
    .await);
    let test = id(&post(
        &app,
        &format!("{}/security-tests", m.base),
        json!({"title": "x", "type": "other", "status": "ready", "threatIds": [t]}),
    )
    .await);
    let r = id(&post(
        &app,
        &format!("{}/security-tests/{test}/results", m.base),
        json!({"outcome": "passed"}),
    )
    .await);
    let path = format!("{}/security-tests/{test}/results/{r}/evidence", m.base);
    let e = post(
        &app,
        &path,
        json!({"kind": "scan", "title": "Run log", "note": "see CI", "evidenceDate": "2026-10-02"}),
    )
    .await;
    assert_eq!(e["ownerKind"], "test-result");
    let (_, list) = app.send("GET", &path, None).await;
    assert_eq!(list.as_array().unwrap().len(), 1);

    // A control of another threat model is not found.
    let other = model(&app).await;
    let c = id(&post(
        &app,
        &format!("{}/controls", other.base),
        json!({"name": "x", "type": "preventive", "category": "identity-access"}),
    )
    .await);
    let (s, _) = app
        .send(
            "POST",
            &format!("{}/controls/{c}/evidence", m.base),
            Some(json!({"kind": "scan", "title": "t", "note": "n", "evidenceDate": "2026-10-02"})),
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn baselines_freeze_compare_and_stay_tenant_scoped() {
    let app = common::app().await;
    let m = model(&app).await;
    let kept = id(&post(
        &app,
        &format!("{}/threats", m.base),
        threat_body(
            &m.api,
            json!({"title": "Kept", "current": {"likelihood": 4, "impact": 4}}),
        ),
    )
    .await);
    let removed = id(&post(
        &app,
        &format!("{}/threats", m.base),
        threat_body(&m.web, json!({"title": "Removed"})),
    )
    .await);
    let ctl = post(
        &app,
        &format!("{}/controls", m.base),
        json!({"name": "WAF", "type": "preventive", "category": "application-network"}),
    )
    .await;

    let b = post(
        &app,
        &format!("{}/baselines", m.base),
        json!({"name": "Release 2.3 review", "note": "signed off"}),
    )
    .await;
    assert_eq!(b["threatCount"], 2);
    assert_eq!(b["controlCount"], 1);
    let (s, _) = app
        .send(
            "POST",
            &format!("{}/baselines", m.base),
            Some(json!({"name": "Release 2.3 review"})),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);

    // Change the live model: lower a level, delete a threat, add one, verify a control.
    let (_, kept_now) = app
        .send("GET", &format!("{}/threats/{kept}", m.base), None)
        .await;
    let mut body = threat_body(
        &m.api,
        json!({"title": "Kept", "current": {"likelihood": 2, "impact": 2}}),
    );
    body["treatment"] = json!("mitigate");
    let (s, _) = app
        .send_if_match(
            "PUT",
            &format!("{}/threats/{kept}", m.base),
            kept_now["version"].as_i64().unwrap(),
            body,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    app.send("DELETE", &format!("{}/threats/{removed}", m.base), None)
        .await;
    post(
        &app,
        &format!("{}/threats", m.base),
        threat_body(&m.web, json!({"title": "Added"})),
    )
    .await;
    let cpath = format!("{}/controls/{}", m.base, id(&ctl));
    post(
        &app,
        &format!("{cpath}/evidence"),
        json!({"kind": "review", "title": "r", "note": "n", "evidenceDate": "2026-10-02"}),
    )
    .await;
    let (s, _) = app
        .send_if_match(
            "PUT",
            &cpath,
            1,
            json!({"name": "WAF", "type": "preventive", "category": "application-network", "status": "verified"}),
        )
        .await;
    assert_eq!(s, StatusCode::OK);

    // The baseline still shows the old data.
    let bid = id(&b);
    let (s, detail) = app
        .send("GET", &format!("{}/baselines/{bid}", m.base), None)
        .await;
    assert_eq!(s, StatusCode::OK);
    let snap = &detail["snapshot"];
    assert_eq!(snap["threats"].as_array().unwrap().len(), 2);
    let kept_snap = snap["threats"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["title"] == "Kept")
        .unwrap();
    assert_eq!(kept_snap["currentLevel"], "high");
    assert_eq!(snap["controls"][0]["status"], "planned");

    let (s, cmp) = app
        .send("GET", &format!("{}/baselines/{bid}/compare", m.base), None)
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(cmp["added"][0]["title"], "Added");
    assert_eq!(cmp["removed"][0]["title"], "Removed");
    let changed = &cmp["changed"][0];
    assert_eq!(changed["title"], "Kept");
    assert!(changed["changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["field"] == "currentLevel" && c["before"] == "high" && c["after"] == "low"));
    assert_eq!(cmp["controlsChanged"][0]["after"], "verified");

    // Tenancy: another tenant cannot see the baseline.
    let other = project(&app, "other").await;
    let foreign = app
        .store
        .create_threat_model(
            other.parse().unwrap(),
            Default::default(),
            Default::default(),
        )
        .await
        .unwrap();
    let (s, _) = app
        .send(
            "GET",
            &format!("/threat-models/{}/baselines/{bid}", foreign.audit.id),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    // ... nor can the dev tenant read it through another threat model's path.
    let own = model(&app).await;
    let (s, _) = app
        .send("GET", &format!("{}/baselines/{bid}", own.base), None)
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    let (s, _) = app
        .send("DELETE", &format!("{}/baselines/{bid}", m.base), None)
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (_, list) = app
        .send("GET", &format!("{}/baselines", m.base), None)
        .await;
    assert!(list.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn detail_fields_sensitive_crossings_boundaries_and_completeness() {
    let app = common::app().await;
    let m = model(&app).await;
    let asset = post(
        &app,
        &format!("{}/assets", m.base),
        json!({"name": "Records", "category": "business-data", "classification": "restricted",
               "dataLocation": "eu-west-1", "retention": "7y", "encryptionAtRest": "no"}),
    )
    .await;
    assert_eq!(asset["classification"], "restricted");
    assert_eq!(asset["encryptionAtRest"], "no");
    // The API (trust 3) sends data to the web app (trust 0): less trusted.
    let (s, flow) = app
        .send_if_match(
            "PUT",
            &format!("{}/flows/{}", m.base, m.flow),
            1,
            json!({"sourceId": m.api, "targetId": m.web, "relationship": "sends-data",
                   "assetIds": [id(&asset)], "entryPoint": true,
                   "authorizationModel": "RBAC", "dataClassification": "confidential"}),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{flow}");
    assert_eq!(flow["entryPoint"], true);
    let codes: Vec<&str> = flow["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"CROSSING_SENSITIVE_DATA"), "{codes:?}");

    // A threat on the API shows the crossing it is exposed to.
    let t = post(
        &app,
        &format!("{}/threats", m.base),
        threat_body(&m.api, json!({})),
    )
    .await;
    assert_eq!(t["affectedBoundaries"][0]["direction"], "into-less-trusted");

    let (_, summary) = app.send("GET", &format!("{}/summary", m.base), None).await;
    let codes: Vec<&str> = summary["completeness"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].as_str().unwrap())
        .collect();
    for expected in [
        "COMPONENT_NO_OWNER",
        "FLOW_AUTH_UNKNOWN",
        "THREAT_NO_TREATMENT",
        "THREAT_NO_OWNER",
    ] {
        assert!(codes.contains(&expected), "{expected} in {codes:?}");
    }
}
