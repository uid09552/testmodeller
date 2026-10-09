//! HTTP API for TestModeller. See docs/specification/05-api.md and openapi.yaml.

pub mod ai_service;
pub mod auth;
pub mod config;
pub mod dto;
pub mod error;
pub mod extract;
pub mod handlers;
pub mod results_import;
pub mod secrets;
pub mod tenant_guard;

use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post, put};
use axum::Router;
use tm_storage::Store;

use crate::ai_service::AiService;
use crate::auth::{AuthMode, Authenticator};
use crate::handlers::{
    ai, export, models, organization as org, test_cases as tc, test_results as results,
};

/// Maximum request body size (imports can be large).
const BODY_LIMIT: usize = 16 * 1024 * 1024;

/// Shared handler state.
#[derive(Clone)]
pub struct AppState {
    /// Persistence.
    pub store: Store,
    /// AI orchestration.
    pub ai: Arc<AiService>,
}

impl AppState {
    /// Creates state from a store, an optional AI API key, and the secret
    /// store that persists it (see `secrets`).
    pub fn new(
        store: Store,
        ai_api_key: Option<String>,
        secrets: Arc<secrets::SecretStore>,
    ) -> Self {
        Self {
            ai: Arc::new(AiService::new(store.clone(), ai_api_key, secrets)),
            store,
        }
    }
}

/// Builds the authentication mode: dev mode skips validation (FR-040).
pub fn auth_mode(config: Option<auth::AuthConfig>) -> AuthMode {
    match config {
        Some(c) => AuthMode::Jwt(Arc::new(Authenticator::new(c))),
        None => AuthMode::Dev,
    }
}

/// Builds the `/api/v1` router.
///
/// `auth` decides whether tokens are validated. Every route except `/health`
/// runs through authentication and the tenant guard, so a new endpoint is
/// covered the day it is added (FR-041, FR-044, FR-045).
pub fn router(state: AppState, auth: AuthMode) -> Router {
    let protected = Router::new()
        .route(
            "/projects",
            get(org::list_projects).post(org::create_project),
        )
        .route(
            "/projects/{id}",
            get(org::get_project)
                .patch(org::update_project)
                .delete(org::delete_project),
        )
        .route("/projects/{id}/tree", get(org::project_tree))
        .route("/projects/{id}/search", get(org::search))
        .route(
            "/projects/{id}/components",
            get(org::list_components).post(org::create_component),
        )
        .route(
            "/components/{id}",
            get(org::get_component)
                .patch(org::update_component)
                .delete(org::delete_component),
        )
        .route(
            "/components/{id}/features",
            get(org::list_features).post(org::create_feature),
        )
        .route(
            "/features/{id}",
            get(org::get_feature)
                .patch(org::update_feature)
                .delete(org::delete_feature),
        )
        .route("/features/{id}/move", post(org::move_feature))
        .route(
            "/features/{id}/models",
            get(models::list_models).post(models::create_model),
        )
        .route(
            "/models/{id}",
            get(models::get_model)
                .put(models::replace_model)
                .patch(models::update_model_meta)
                .delete(models::delete_model),
        )
        .route("/models/{id}/duplicate", post(models::duplicate_model))
        .route("/models/{id}/validate", post(models::validate_model))
        .route("/models/{id}/versions", get(models::list_versions))
        .route(
            "/models/{id}/versions/{version}/restore",
            post(models::restore_version),
        )
        .route(
            "/models/{id}/states",
            get(models::list_states).post(models::add_state),
        )
        .route(
            "/states/{id}",
            get(models::get_state)
                .patch(models::update_state)
                .delete(models::delete_state),
        )
        .route(
            "/models/{id}/transitions",
            get(models::list_transitions).post(models::add_transition),
        )
        .route(
            "/transitions/{id}",
            get(models::get_transition)
                .patch(models::update_transition)
                .delete(models::delete_transition),
        )
        .route(
            "/features/{id}/test-cases",
            get(tc::list_test_cases).post(tc::create_test_case),
        )
        .route(
            "/test-cases/{id}",
            get(tc::get_test_case)
                .put(tc::replace_test_case)
                .delete(tc::delete_test_case),
        )
        .route("/test-cases/{id}/move", post(tc::move_test_case))
        .route(
            "/test-cases/{id}/assignments",
            get(tc::list_assignments).put(tc::replace_assignments),
        )
        .route("/states/{id}/test-cases", get(tc::state_test_cases))
        .route(
            "/states/{id}/test-cases/{tc}",
            put(tc::assign_state).delete(tc::unassign_state),
        )
        .route(
            "/transitions/{id}/test-cases",
            get(tc::transition_test_cases),
        )
        .route(
            "/transitions/{id}/test-cases/{tc}",
            put(tc::assign_transition).delete(tc::unassign_transition),
        )
        .route("/models/{id}/test-cases", get(tc::model_test_cases))
        .route("/models/{id}/generate", post(tc::generate))
        .route("/models/{id}/coverage", get(tc::model_coverage))
        .route("/models/{id}/stale-tests", get(tc::stale_tests))
        .route("/models/{id}/simulate", post(tc::simulate))
        .route("/features/{id}/coverage", get(tc::feature_coverage))
        .route("/components/{id}/coverage", get(tc::component_coverage))
        .route("/projects/{id}/traceability", get(tc::traceability))
        .route("/projects/{id}/test-results", post(results::import))
        .route("/test-cases/{id}/results", get(results::history))
        .route("/projects/{id}/export", get(export::export))
        .route("/projects/{id}/import", post(export::import))
        .route("/ai/proposals", post(ai::request_proposals))
        .route("/jobs/{id}", get(ai::get_job))
        .route("/projects/{id}/proposals", get(ai::list_proposals))
        .route("/proposals/{id}", get(ai::get_proposal))
        .route("/proposals/{id}/accept", post(ai::accept_proposal))
        .route("/proposals/{id}/reject", post(ai::reject_proposal))
        .route(
            "/settings/ai",
            get(ai::get_ai_settings).put(ai::put_ai_settings),
        )
        // Order matters: the guard runs after authentication, so it can read
        // the identity, and both run before any handler.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            tenant_guard::guard,
        ))
        .layer(axum::middleware::from_fn_with_state(
            auth,
            auth::authenticate,
        ))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(state.clone());

    let api = Router::new()
        // Open, so a load balancer needs no token.
        .route("/health", get(org::health))
        .with_state(state)
        .merge(protected);
    Router::new().nest("/api/v1", api)
}
