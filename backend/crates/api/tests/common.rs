//! Shared setup for HTTP-level tests: a real PostgreSQL (Docker) behind the router in dev mode,
//! where the caller's tenant is `dev`.
#![allow(dead_code)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::Value;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tm_api::secrets::{KeySource, SecretStore};
use tm_api::{auth_mode, router, AppState};
use tm_domain::{ModelGraph, State, StateKind, Transition};
use tm_storage::{FeatureFields, ModelMeta, NamedFields, Store};
use tower::ServiceExt;
use uuid::Uuid;

pub struct App {
    _db: ContainerAsync<Postgres>,
    pub store: Store,
    pub router: Router,
}

pub async fn app() -> App {
    let db = Postgres::default()
        .with_tag("17-alpine")
        .start()
        .await
        .expect("start PostgreSQL; is Docker running?");
    let host = db.get_host().await.unwrap();
    let port = db.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let store = Store::connect(&url, 4).await.unwrap();
    let secrets = Arc::new(SecretStore::from_bytes(&[7u8; 32], KeySource::Configured).unwrap());
    let state = AppState::new(store.clone(), None, secrets);
    App {
        _db: db,
        store,
        router: router(state, auth_mode(None)),
    }
}

impl App {
    pub async fn send(&self, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        self.send_raw(
            method,
            path,
            body.map(|b| ("application/json", b.to_string().into_bytes())),
        )
        .await
    }

    /// GET returning the body as text (exports).
    pub async fn get_text(&self, path: &str) -> (StatusCode, String) {
        let req = Request::builder()
            .uri(format!("/api/v1{path}"))
            .body(Body::empty())
            .unwrap();
        let res = self.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    pub async fn send_raw(
        &self,
        method: &str,
        path: &str,
        body: Option<(&str, Vec<u8>)>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(format!("/api/v1{path}"));
        let body = match body {
            Some((content_type, bytes)) => {
                req = req.header("content-type", content_type);
                Body::from(bytes)
            }
            None => Body::empty(),
        };
        let res = self
            .router
            .clone()
            .oneshot(req.body(body).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, json)
    }
}

/// Ids of a project with one component, feature and model.
pub struct Tree {
    pub project: Uuid,
    pub component: Uuid,
    pub feature: Uuid,
    pub model: Uuid,
    pub graph: ModelGraph,
}

fn state(name: &str, kind: StateKind) -> State {
    State {
        id: Uuid::new_v4(),
        name: name.into(),
        description: None,
        kind,
        position: None,
    }
}

fn tr(from: &State, to: &State, event: &str) -> Transition {
    Transition {
        id: Uuid::new_v4(),
        from: from.id,
        to: to.id,
        event: event.into(),
        guard: None,
        action: None,
        expected: None,
    }
}

/// Start -> Out -> In -> Done.
pub async fn tree(store: &Store, tenant: &str) -> Tree {
    let named = |n: &str| NamedFields {
        name: n.into(),
        description: None,
    };
    let project = store
        .create_project(tenant, named("P"))
        .await
        .unwrap()
        .audit
        .id;
    let component = store
        .create_component(project, named("C"))
        .await
        .unwrap()
        .audit
        .id;
    let feature = store
        .create_feature(
            component,
            FeatureFields {
                name: "F".into(),
                description: None,
                scenario_description: None,
                tags: None,
            },
        )
        .await
        .unwrap()
        .audit
        .id;
    let (start, out, inn, done) = (
        state("Start", StateKind::Initial),
        state("Out", StateKind::Normal),
        state("In", StateKind::Normal),
        state("Done", StateKind::Final),
    );
    let graph = ModelGraph {
        variables: vec![],
        transitions: vec![
            tr(&start, &out, "open"),
            tr(&out, &inn, "login"),
            tr(&inn, &done, "close"),
        ],
        states: vec![start, out, inn, done],
    };
    let model = store
        .create_model(feature, "M", ModelMeta::default(), &graph)
        .await
        .unwrap()
        .summary
        .audit
        .id;
    Tree {
        project,
        component,
        feature,
        model,
        graph,
    }
}
