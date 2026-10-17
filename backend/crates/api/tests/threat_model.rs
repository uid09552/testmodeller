//! Threat model endpoints over HTTP (FR-060..FR-076). Requires Docker.

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

#[tokio::test]
async fn threat_model_crud_conflict_layout_and_tenancy() {
    let app = common::app().await;
    let p = project(&app, "dev").await;

    let (s, _) = app
        .send("GET", &format!("/projects/{p}/threat-model"), None)
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "nothing is created implicitly");

    let tm = post(
        &app,
        &format!("/projects/{p}/threat-model"),
        json!({"name": "Shop"}),
    )
    .await;
    assert_eq!(
        tm["thresholds"][2],
        json!({"level": "high", "min": 10, "max": 16})
    );
    assert_eq!(tm["layoutVersion"], 0);
    let tm_id = id(&tm);
    let (s, _) = app
        .send(
            "POST",
            &format!("/projects/{p}/threat-model"),
            Some(json!({"name": "x"})),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);

    // Scope edit with If-Match; stale version is 412; bad thresholds 422.
    let (s, v) = app
        .send_if_match(
            "PUT",
            &format!("/threat-models/{tm_id}"),
            1,
            json!({"name": "Shop", "assumptions": "TLS"}),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["assumptions"], "TLS");
    let (s, _) = app
        .send_if_match(
            "PUT",
            &format!("/threat-models/{tm_id}"),
            1,
            json!({"name": "Shop"}),
        )
        .await;
    assert_eq!(s, StatusCode::PRECONDITION_FAILED);
    let (s, v) = app
        .send_if_match(
            "PUT",
            &format!("/threat-models/{tm_id}"),
            2,
            json!({"name": "Shop", "thresholds": [
                {"level": "low", "min": 1, "max": 4}, {"level": "moderate", "min": 5, "max": 8},
                {"level": "high", "min": 10, "max": 16}, {"level": "critical", "min": 17, "max": 25}]}),
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(v["errors"][0]["field"], "thresholds");

    // Layout: own version, 0 allowed.
    let (s, v) = app
        .send_if_match(
            "PUT",
            &format!("/threat-models/{tm_id}/layout"),
            0,
            json!({"v": 1}),
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["layoutVersion"], 1);
    let (s, _) = app
        .send_if_match(
            "PUT",
            &format!("/threat-models/{tm_id}/layout"),
            0,
            json!({"v": 1}),
        )
        .await;
    assert_eq!(s, StatusCode::PRECONDITION_FAILED);
    let (s, _) = app
        .send_if_match(
            "PUT",
            &format!("/threat-models/{tm_id}/layout"),
            1,
            json!([1]),
        )
        .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Another tenant's threat model and items are not found.
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
    for path in [
        format!("/threat-models/{}", foreign.audit.id),
        format!("/threat-models/{}/components", foreign.audit.id),
        format!("/projects/{other}/threat-model"),
    ] {
        let (s, _) = app.send("GET", &path, None).await;
        assert_eq!(s, StatusCode::NOT_FOUND, "{path}");
    }

    let (s, _) = app
        .send("DELETE", &format!("/threat-models/{tm_id}"), None)
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn architecture_threats_risks_controls_and_tests() {
    let app = common::app().await;
    let p = project(&app, "dev").await;
    let tm = id(&post(
        &app,
        &format!("/projects/{p}/threat-model"),
        json!({"name": "Shop"}),
    )
    .await);
    let base = format!("/threat-models/{tm}");

    let internet = id(&post(
        &app,
        &format!("{base}/zones"),
        json!({"name": "Internet", "kind": "internet", "trustLevel": 0}),
    )
    .await);
    let internal = id(&post(
        &app,
        &format!("{base}/zones"),
        json!({"name": "Internal", "kind": "internal", "trustLevel": 3}),
    )
    .await);
    let records = id(&post(
        &app,
        &format!("{base}/assets"),
        json!({"name": "Customer records", "category": "business-data", "confidentiality": "high"}),
    )
    .await);
    let web = id(&post(
        &app,
        &format!("{base}/components"),
        json!({"name": "Web app", "type": "web-frontend", "zoneId": internet}),
    )
    .await);
    let api_v = post(&app, &format!("{base}/components"), json!({
        "name": "Customer API", "type": "api", "internetFacing": true, "zoneId": internal, "assetIds": [records]
    }))
    .await;
    assert_eq!(api_v["layer"], "integration");
    let api = id(&api_v);

    let (s, v) = app
        .send(
            "POST",
            &format!("{base}/components"),
            Some(json!({"name": "x", "type": "mainframe"})),
        )
        .await;
    assert!(s.is_client_error(), "{s}");
    assert!(
        v.to_string().contains("web-frontend"),
        "names the allowed types: {v}"
    );
    let (s, _) = app
        .send(
            "POST",
            &format!("{base}/components"),
            Some(json!({"name": "Web app", "type": "api"})),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);

    let flow = post(
        &app,
        &format!("{base}/flows"),
        json!({"sourceId": web, "targetId": api, "relationship": "calls"}),
    )
    .await;
    assert_eq!(
        flow["crossing"],
        json!({"crosses": true, "direction": "into-more-trusted", "fromTrust": 0, "toTrust": 3})
    );
    let codes: Vec<&str> = flow["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert_eq!(
        codes,
        [
            "CROSSING_AUTH_UNDOCUMENTED",
            "CROSSING_ENCRYPTION_UNDOCUMENTED"
        ]
    );
    let flow_id = id(&flow);

    let (_, asset) = app
        .send("GET", &format!("{base}/assets/{records}"), None)
        .await;
    assert_eq!(asset["componentIds"], json!([api]));

    // An item of another threat model is not found under this one.
    let p2 = project(&app, "dev").await;
    let tm2 = id(&post(
        &app,
        &format!("/projects/{p2}/threat-model"),
        json!({"name": "Other"}),
    )
    .await);
    let (s, _) = app
        .send(
            "GET",
            &format!("/threat-models/{tm2}/components/{api}"),
            None,
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // Suggest: candidates with explanations; second run creates nothing.
    let sug = post(&app, &format!("{base}/threats/suggest"), json!({})).await;
    let rules: Vec<&str> = sug["created"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["explanation"]["ruleId"].as_str().unwrap())
        .collect();
    assert_eq!(
        rules,
        ["SPOOF-BOUNDARY-AUTH", "INFO-PUBLIC-SENSITIVE", "DOS-PUBLIC"]
    );
    assert!(sug["created"]
        .as_array()
        .unwrap()
        .iter()
        .all(|t| t["status"] == "candidate"));
    let again = post(&app, &format!("{base}/threats/suggest"), json!({})).await;
    assert_eq!(again["created"], json!([]));
    assert_eq!(again["skipped"], 3);
    let spoof = id(&sug["created"][0]);
    let dos = id(&sug["created"][2]);

    // Bulk: dismiss needs a reason; confirm makes them open.
    let (s, v) = app
        .send(
            "POST",
            &format!("{base}/threats/bulk-status"),
            Some(json!({"ids": [dos], "status": "dismissed"})),
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(v["errors"][0]["field"], "dismissReason");
    post(
        &app,
        &format!("{base}/threats/bulk-status"),
        json!({"ids": [dos], "status": "dismissed", "reason": "behind CDN"}),
    )
    .await;
    let confirmed = post(
        &app,
        &format!("{base}/threats/bulk-status"),
        json!({"ids": [spoof], "status": "open"}),
    )
    .await;
    assert_eq!(confirmed[0]["status"], "open");
    assert_eq!(confirmed[0]["flags"], json!(["no-control"]));

    // Manual threat with risk; accept needs a rationale.
    let manual = post(&app, &format!("{base}/threats"), json!({
        "title": "Unauthorized access to customer records", "category": "information-disclosure",
        "targets": [{"kind": "component", "id": api}], "assetIds": [records],
        "inherent": {"likelihood": 4, "impact": 5}
    }))
    .await;
    assert_eq!(manual["displayId"], "THR-4");
    assert_eq!(
        (
            manual["inherentScore"].clone(),
            manual["inherentLevel"].clone()
        ),
        (json!(20), json!("critical"))
    );
    assert_eq!(manual["currentLevel"], Value::Null);
    let manual_id = id(&manual);
    let (s, _) = app
        .send(
            "POST",
            &format!("{base}/threats"),
            Some(json!({"title": "t", "category": "spoofing", "targets": []})),
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    let (s, v) = app
        .send_if_match("PUT", &format!("{base}/threats/{manual_id}"), manual["version"].as_i64().unwrap(), json!({
            "title": "Unauthorized access to customer records", "category": "information-disclosure",
            "targets": [{"kind": "component", "id": api}], "treatment": "accept"
        }))
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(v["errors"][0]["field"], "acceptanceRationale");

    // Control linked to the threat; ratings unchanged.
    let ctl = post(&app, &format!("{base}/controls"), json!({
        "name": "Object-level authorization", "type": "preventive", "category": "identity-access",
        "threatIds": [manual_id], "componentIds": [api]
    }))
    .await;
    assert_eq!(ctl["displayId"], "CTL-1");
    let ctl_id = id(&ctl);
    let (_, t) = app
        .send("GET", &format!("{base}/threats/{manual_id}"), None)
        .await;
    assert_eq!(t["controlIds"], json!([ctl_id]));
    assert_eq!(t["inherentScore"], 20);
    assert_eq!(t["flags"], json!(["planned-controls-only"]));

    // Generate tests: candidates and dismissed excluded; no duplicates.
    let gen = post(&app, &format!("{base}/security-tests/generate"), json!({})).await;
    let created = gen["created"].as_array().unwrap();
    assert_eq!(created.len(), 2, "{gen}");
    assert!(created
        .iter()
        .all(|t| t["status"] == "draft" && t["origin"] == "generated"));
    let auth_test = created
        .iter()
        .find(|t| t["templateId"] == "AUTH-FLOW")
        .unwrap();
    assert_eq!(
        auth_test["steps"][0]["action"],
        "Call Customer API from Web app without credentials"
    );
    let again = post(
        &app,
        &format!("{base}/security-tests/generate"),
        Value::Null,
    )
    .await;
    assert_eq!(
        (again["created"].clone(), again["skipped"].clone()),
        (json!([]), json!(2))
    );
    let access_test = created
        .iter()
        .find(|t| t["templateId"] == "ACCESS-COMPONENT")
        .unwrap();
    assert_eq!(access_test["controlIds"], json!([ctl_id]));
    let test_id = id(access_test);

    // Results only for ready tests.
    let (s, _) = app
        .send(
            "POST",
            &format!("{base}/security-tests/{test_id}/results"),
            Some(json!({"outcome": "failed"})),
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    post(
        &app,
        &format!("{base}/security-tests/bulk-status"),
        json!({"ids": [test_id], "status": "ready"}),
    )
    .await;
    post(
        &app,
        &format!("{base}/security-tests/{test_id}/results"),
        json!({"outcome": "failed", "note": "IDOR on /customers/{id}"}),
    )
    .await;
    let (_, tests) = app
        .send("GET", &format!("{base}/security-tests"), None)
        .await;
    let t = tests
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == test_id.as_str())
        .unwrap();
    assert_eq!(t["lastResult"]["outcome"], "failed");
    let (_, threat) = app
        .send("GET", &format!("{base}/threats/{manual_id}"), None)
        .await;
    assert!(threat["flags"]
        .as_array()
        .unwrap()
        .contains(&json!("failing-test")));
    let (_, ctl) = app
        .send("GET", &format!("{base}/controls/{ctl_id}"), None)
        .await;
    assert_eq!(ctl["securityTestIds"], json!([test_id]));

    // Unlinking the control makes its generated test stale.
    let (_, threat) = app
        .send("GET", &format!("{base}/threats/{manual_id}"), None)
        .await;
    let (s, _) = app
        .send_if_match("PUT", &format!("{base}/threats/{manual_id}"), threat["version"].as_i64().unwrap(), json!({
            "title": "Unauthorized access to customer records", "category": "information-disclosure",
            "targets": [{"kind": "component", "id": api}], "controlIds": []
        }))
        .await;
    assert_eq!(s, StatusCode::OK);
    let (_, t) = app
        .send("GET", &format!("{base}/security-tests/{test_id}"), None)
        .await;
    assert_eq!(t["stale"], "control unlinked: CTL-1");

    // Summary.
    let (_, s) = app.send("GET", &format!("{base}/summary"), None).await;
    assert_eq!(
        (
            s["components"].clone(),
            s["flows"].clone(),
            s["crossings"].clone()
        ),
        (json!(2), json!(1), json!(1))
    );
    assert_eq!(s["threatsByStatus"]["candidate"], 1);
    assert_eq!(s["threatsByStatus"]["dismissed"], 1);
    assert_eq!(s["threatsByStatus"]["open"], 2);
    assert_eq!(s["controlsVerifiedRatio"], 0.0);
    assert_eq!(s["inherentMatrix"].as_array().unwrap().len(), 25);

    // Deleting a component deletes its flows.
    let (s, _) = app
        .send("DELETE", &format!("{base}/components/{web}"), None)
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _) = app
        .send("GET", &format!("{base}/flows/{flow_id}"), None)
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}
