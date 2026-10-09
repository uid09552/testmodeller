//! The editor's layout stored with a model (ADR 0010). Requires Docker.

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};

/// The model as `PUT` input, with `layout` set as given (or left out).
fn input(model: &Value, layout: Option<Value>) -> Value {
    let mut body = json!({
        "name": model["name"],
        "variables": model["variables"],
        "states": model["states"],
        "transitions": model["transitions"],
    });
    if let Some(l) = layout {
        body["layout"] = l;
    }
    body
}

async fn get(app: &common::App, id: impl std::fmt::Display) -> Value {
    let (status, m) = app.send("GET", &format!("/models/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    m
}

#[tokio::test]
async fn saves_keeps_clears_and_rejects_layout() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let start = t.graph.states[0].id;
    let layout = json!({ "v": 1, "states": { start.to_string(): { "shape": "diamond" } } });

    let m = get(&app, t.model).await;
    assert!(m.get("layout").is_none(), "no layout before the first save");
    let path = format!("/models/{}", t.model);
    let (status, saved) = app
        .send_if_match(
            "PUT",
            &path,
            m["version"].as_i64().unwrap(),
            input(&m, Some(layout.clone())),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(get(&app, t.model).await["layout"], layout);

    // Absent keeps it.
    let m = get(&app, t.model).await;
    app.send_if_match(
        "PUT",
        &path,
        m["version"].as_i64().unwrap(),
        input(&m, None),
    )
    .await;
    assert_eq!(get(&app, t.model).await["layout"], layout);

    // Too large or not an object: rejected, nothing changes.
    let before = get(&app, t.model).await;
    let version = before["version"].as_i64().unwrap();
    let big = json!({ "pad": "x".repeat(256 * 1024) });
    for bad in [big, json!([1, 2]), json!("layout")] {
        let (status, body) = app
            .send_if_match("PUT", &path, version, input(&before, Some(bad)))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["errors"][0]["field"], json!("layout"));
    }
    assert_eq!(get(&app, t.model).await["version"], before["version"]);

    // Null clears it.
    app.send_if_match("PUT", &path, version, input(&before, Some(Value::Null)))
        .await;
    assert!(get(&app, t.model).await.get("layout").is_none());
}

#[tokio::test]
async fn restore_and_duplicate_carry_layout() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let start = t.graph.states[0].id;
    let first = t.graph.transitions[0].id;
    let layout = json!({
        "v": 1,
        "states": { start.to_string(): { "color": "#ff0000" } },
        "transitions": { first.to_string(): { "curve": 40 } },
    });
    let path = format!("/models/{}", t.model);
    let m = get(&app, t.model).await;
    app.send_if_match(
        "PUT",
        &path,
        m["version"].as_i64().unwrap(),
        input(&m, Some(layout.clone())),
    )
    .await;
    let with_layout = get(&app, t.model).await["version"].as_i64().unwrap();
    let m = get(&app, t.model).await;
    app.send_if_match(
        "PUT",
        &path,
        with_layout,
        input(&m, Some(json!({ "v": 1 }))),
    )
    .await;

    // Restoring the earlier version brings its layout back.
    let (status, restored) = app
        .send(
            "POST",
            &format!("/models/{}/versions/{with_layout}/restore", t.model),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{restored}");
    assert_eq!(get(&app, t.model).await["layout"], layout);

    // A duplicate's layout points at the copy's elements.
    let (status, copy) = app
        .send(
            "POST",
            &format!("/models/{}/duplicate", t.model),
            Some(json!({})),
        )
        .await;
    assert!(status.is_success(), "{copy}");
    let new_start = copy["states"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == json!("Start"))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(new_start, start.to_string());
    let copy_layout = &get(&app, copy["id"].as_str().unwrap()).await["layout"];
    assert_eq!(copy_layout["states"][&new_start]["color"], json!("#ff0000"));
    assert!(copy_layout["states"].get(start.to_string()).is_none());
    assert_eq!(copy_layout["transitions"].as_object().unwrap().len(), 1);
    assert!(copy_layout["transitions"].get(first.to_string()).is_none());
}

#[tokio::test]
async fn export_and_import_keep_layout() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let start = t.graph.states[0].id;
    let path = format!("/models/{}", t.model);
    let m = get(&app, t.model).await;
    app.send_if_match(
        "PUT",
        &path,
        m["version"].as_i64().unwrap(),
        input(
            &m,
            Some(json!({ "v": 1, "states": { start.to_string(): { "shape": "circle" } } })),
        ),
    )
    .await;

    let (status, export) = app
        .send(
            "GET",
            &format!("/projects/{}/export?format=json", t.project),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let target = common::tree(&app.store, "dev").await;
    let (status, result) = app
        .send(
            "POST",
            &format!("/projects/{}/import", target.project),
            Some(export),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{result}");

    let (_, tree) = app
        .send("GET", &format!("/projects/{}/tree", target.project), None)
        .await;
    let imported = tree["components"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|c| c["features"].as_array().unwrap().clone())
        .flat_map(|f| f["models"].as_array().unwrap().clone())
        .find(|m| m["id"] != json!(target.model.to_string()))
        .unwrap();
    let model = get(&app, imported["id"].as_str().unwrap()).await;
    let new_start = model["states"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == json!("Start"))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        model["layout"]["states"][&new_start]["shape"],
        json!("circle")
    );
}

#[tokio::test]
async fn layout_does_not_change_behaviour() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let path = format!("/models/{}", t.model);
    let generate = json!({ "criterion": "transition-pair", "seed": 3 });
    let (_, before_gen) = app
        .send(
            "POST",
            &format!("/models/{}/generate", t.model),
            Some(generate.clone()),
        )
        .await;
    let (_, before_val) = app
        .send("POST", &format!("/models/{}/validate", t.model), None)
        .await;
    let (_, before_cov) = app
        .send("GET", &format!("/models/{}/coverage", t.model), None)
        .await;

    let m = get(&app, t.model).await;
    let states: serde_json::Map<String, Value> = t
        .graph
        .states
        .iter()
        .map(|s| {
            (
                s.id.to_string(),
                json!({ "shape": "diamond", "decision": true }),
            )
        })
        .collect();
    app.send_if_match(
        "PUT",
        &path,
        m["version"].as_i64().unwrap(),
        input(&m, Some(json!({ "v": 1, "states": states }))),
    )
    .await;

    let (_, after_gen) = app
        .send(
            "POST",
            &format!("/models/{}/generate", t.model),
            Some(generate),
        )
        .await;
    let names = |g: &Value| -> Vec<Value> {
        g["testCases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["steps"].clone())
            .collect()
    };
    assert_eq!(names(&before_gen), names(&after_gen));
    assert_eq!(before_gen["coverage"], after_gen["coverage"]);
    let (_, after_val) = app
        .send("POST", &format!("/models/{}/validate", t.model), None)
        .await;
    assert_eq!(before_val, after_val);
    let (_, after_cov) = app
        .send("GET", &format!("/models/{}/coverage", t.model), None)
        .await;
    assert_eq!(before_cov, after_cov);
}
