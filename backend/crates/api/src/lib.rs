//! HTTP API for TestModeller. See docs/specification/05-api.md and openapi.yaml.

pub mod ai_service;
pub mod config;
pub mod dto;
pub mod error;
pub mod extract;
pub mod handlers;

use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post, put};
use axum::Router;
use tm_storage::Store;

use crate::ai_service::AiService;
use crate::handlers::{ai, export, models, organization as org, test_cases as tc};

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
    /// Creates state from a store and an optional AI API key.
    pub fn new(store: Store, ai_api_key: Option<String>) -> Self {
        Self {
            ai: Arc::new(AiService::new(store.clone(), ai_api_key)),
            store,
        }
    }
}

/// Builds the `/api/v1` router.
pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(org::health))
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
        .route("/features/{id}/coverage", get(tc::feature_coverage))
        .route("/components/{id}/coverage", get(tc::component_coverage))
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
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(state);
    Router::new().nest("/api/v1", api)
}
