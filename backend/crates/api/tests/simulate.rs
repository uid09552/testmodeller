//! `POST /models/{modelId}/simulate` end to end. Requires Docker.

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};

/// The tree's graph as the editor would send it, with a guarded retry loop added.
fn graph(t: &common::Tree) -> Value {
    let [start, out, inn, done] = [0, 1, 2, 3].map(|i| t.graph.states[i].id);
    json!({
        "name": "M",
        "variables": [{ "name": "attempts", "type": "integer", "initial": 0 }],
        "states": [
            { "id": start, "name": "Start", "kind": "initial" },
            { "id": out, "name": "Out", "kind": "normal" },
            { "id": inn, "name": "In", "kind": "normal" },
            { "id": done, "name": "Done", "kind": "final" },
        ],
        "transitions": [
            { "id": t.graph.transitions[0].id, "from": start, "to": out, "event": "open" },
            { "id": t.graph.transitions[1].id, "from": out, "to": inn, "event": "login",
              "guard": "attempts > 0" },
            { "id": t.graph.transitions[2].id, "from": inn, "to": done, "event": "close" },
            { "id": "11111111-1111-4111-8111-111111111111", "from": out, "to": out, "event": "retry",
              "action": "attempts = attempts + 1" },
        ],
    })
}

#[tokio::test]
async fn steps_through_the_graph_it_is_sent_and_stores_nothing() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let path = format!("/models/{}/simulate", t.model);
    let g = graph(&t);
    let before = app
        .send("GET", &format!("/models/{}", t.model), None)
        .await
        .1;

    let (status, s) = app.send("POST", &path, Some(json!({ "graph": g }))).await;
    assert_eq!(status, StatusCode::OK, "{s}");
    assert_eq!(s["stateId"], json!(t.graph.states[0].id));
    assert_eq!(s["env"], json!({ "attempts": 0 }));

    let (_, s) = app
        .send("POST", &path, Some(json!({
            "graph": g, "stateId": s["stateId"], "env": s["env"], "take": t.graph.transitions[0].id,
        })))
        .await;
    assert_eq!(s["stateId"], json!(t.graph.states[1].id));
    let login = s["transitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["transitionId"] == json!(t.graph.transitions[1].id))
        .unwrap()
        .clone();
    assert_eq!(login["enabled"], json!(false));
    assert_eq!(login["reason"], json!("guard `attempts > 0` is false"));

    // Taking a blocked transition is refused.
    let (status, body) = app
        .send("POST", &path, Some(json!({
            "graph": g, "stateId": s["stateId"], "env": s["env"], "take": t.graph.transitions[1].id,
        })))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");

    let (_, s) = app
        .send(
            "POST",
            &path,
            Some(json!({
                "graph": g, "stateId": s["stateId"], "env": s["env"],
                "take": "11111111-1111-4111-8111-111111111111",
            })),
        )
        .await;
    assert_eq!(s["env"], json!({ "attempts": 1 }));

    // The stored model is untouched (the graph above has a transition it does not).
    let after = app
        .send("GET", &format!("/models/{}", t.model), None)
        .await
        .1;
    assert_eq!(before, after);
}

#[tokio::test]
async fn rejects_invalid_graphs_and_env_and_other_tenants() {
    let app = common::app().await;
    let t = common::tree(&app.store, "dev").await;
    let path = format!("/models/{}/simulate", t.model);

    let mut no_initial = graph(&t);
    no_initial["states"][0]["kind"] = json!("normal");
    let (status, body) = app
        .send("POST", &path, Some(json!({ "graph": no_initial })))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(!body["errors"].as_array().unwrap().is_empty(), "{body}");

    let (status, _) = app
        .send(
            "POST",
            &path,
            Some(json!({
                "graph": graph(&t), "stateId": t.graph.states[1].id, "env": { "attempts": "two" },
            })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let theirs = common::tree(&app.store, "someone-else").await;
    let (status, _) = app
        .send(
            "POST",
            &format!("/models/{}/simulate", theirs.model),
            Some(json!({ "graph": graph(&t) })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
