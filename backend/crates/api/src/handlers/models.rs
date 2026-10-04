//! Models, versions, validation, states and transitions.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::dto::*;
use crate::error::ApiResult;
use crate::extract::{optional_json, ApiJson, ApiPath, IfMatch};
use crate::AppState;

/// `GET /features/{featureId}/models`
pub async fn list_models(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<Vec<ModelSummaryDto>>> {
    let models = state.store.list_models(id).await?;
    Ok(Json(models.iter().map(Into::into).collect()))
}

/// `POST /features/{featureId}/models`
pub async fn create_model(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<ModelInput>,
) -> ApiResult<(StatusCode, Json<ModelDto>)> {
    let (name, meta, graph) = input.into_parts()?;
    let m = state.store.create_model(id, &name, meta, &graph).await?;
    Ok((StatusCode::CREATED, Json((&m).into())))
}

/// `GET /models/{modelId}`
pub async fn get_model(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<ModelDto>> {
    Ok(Json((&state.store.get_model(id).await?).into()))
}

/// `PUT /models/{modelId}`
pub async fn replace_model(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    IfMatch(version): IfMatch,
    ApiJson(input): ApiJson<ModelInput>,
) -> ApiResult<Json<ModelDto>> {
    let (name, meta, graph) = input.into_parts()?;
    let m = state
        .store
        .replace_model(id, version, &name, meta, &graph)
        .await?;
    Ok(Json((&m).into()))
}

/// `PATCH /models/{modelId}`
pub async fn update_model_meta(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    IfMatch(version): IfMatch,
    ApiJson(input): ApiJson<ModelMetaInput>,
) -> ApiResult<Json<ModelSummaryDto>> {
    let m = state
        .store
        .update_model_meta(id, version, input.into_meta()?)
        .await?;
    Ok(Json((&m).into()))
}

/// `DELETE /models/{modelId}`
pub async fn delete_model(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<StatusCode> {
    state.store.delete_model(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /models/{modelId}/duplicate`
pub async fn duplicate_model(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<ModelDto>)> {
    let body: DuplicateModel = optional_json(&body)?.unwrap_or_default();
    if let Some(n) = &body.name {
        check_len("name", n, 1, 200)?;
    }
    let m = state
        .store
        .duplicate_model(id, body.name.as_deref(), body.feature_id)
        .await?;
    Ok((StatusCode::CREATED, Json((&m).into())))
}

/// `POST /models/{modelId}/validate` — validates the stored model, or the draft graph in the body.
pub async fn validate_model(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    body: Bytes,
) -> ApiResult<Json<Vec<ValidationIssueDto>>> {
    let graph = match optional_json::<ModelInput>(&body)? {
        Some(draft) => {
            state.store.get_model_graph(id).await?;
            ModelInput::graph(draft.variables, draft.states, draft.transitions)?
        }
        None => state.store.get_model_graph(id).await?,
    };
    let issues = tm_domain::validate(&graph);
    Ok(Json(issues.iter().map(Into::into).collect()))
}

/// `GET /models/{modelId}/versions`
pub async fn list_versions(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<Vec<ModelVersionDto>>> {
    let versions = state.store.model_versions(id).await?;
    Ok(Json(versions.iter().map(Into::into).collect()))
}

/// `POST /models/{modelId}/versions/{version}/restore`
pub async fn restore_version(
    State(state): State<AppState>,
    ApiPath((id, version)): ApiPath<(Uuid, u32)>,
) -> ApiResult<Json<ModelDto>> {
    let version = i32::try_from(version)
        .ok()
        .filter(|v| *v >= 1)
        .ok_or_else(|| crate::error::ApiError::bad_request("version must be >= 1"))?;
    let m = state.store.restore_model_version(id, version).await?;
    Ok(Json((&m).into()))
}

/// `GET /models/{modelId}/states`
pub async fn list_states(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<Vec<StateDto>>> {
    let states = state.store.list_states(id).await?;
    Ok(Json(states.iter().map(Into::into).collect()))
}

/// `POST /models/{modelId}/states`
pub async fn add_state(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<StateInput>,
) -> ApiResult<(StatusCode, Json<StateDto>)> {
    let s = state.store.add_state(id, input.into_state()?).await?;
    Ok((StatusCode::CREATED, Json((&s).into())))
}

/// `GET /states/{stateId}`
pub async fn get_state(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<StateDto>> {
    Ok(Json((&state.store.get_state(id).await?).into()))
}

/// `PATCH /states/{stateId}`
pub async fn update_state(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<StateInput>,
) -> ApiResult<Json<StateDto>> {
    let s = state.store.update_state(id, input.into_state()?).await?;
    Ok(Json((&s).into()))
}

/// `DELETE /states/{stateId}`
pub async fn delete_state(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<StatusCode> {
    state.store.delete_state(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /models/{modelId}/transitions`
pub async fn list_transitions(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<Vec<TransitionDto>>> {
    let ts = state.store.list_transitions(id).await?;
    Ok(Json(ts.iter().map(Into::into).collect()))
}

/// `POST /models/{modelId}/transitions`
pub async fn add_transition(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<TransitionInput>,
) -> ApiResult<(StatusCode, Json<TransitionDto>)> {
    let t = state
        .store
        .add_transition(id, input.into_transition()?)
        .await?;
    Ok((StatusCode::CREATED, Json((&t).into())))
}

/// `GET /transitions/{transitionId}`
pub async fn get_transition(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<TransitionDto>> {
    Ok(Json((&state.store.get_transition(id).await?).into()))
}

/// `PATCH /transitions/{transitionId}`
pub async fn update_transition(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<TransitionInput>,
) -> ApiResult<Json<TransitionDto>> {
    let t = state
        .store
        .update_transition(id, input.into_transition()?)
        .await?;
    Ok(Json((&t).into()))
}

/// `DELETE /transitions/{transitionId}`
pub async fn delete_transition(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<StatusCode> {
    state.store.delete_transition(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
