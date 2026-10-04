//! AI proposals, jobs and settings (FR-030 to FR-034).

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use tm_domain::{AiSettings, ProposalStatus};
use tm_storage::ProposalFilter;
use uuid::Uuid;

use crate::ai_service::{acceptance_for, ProposalJobRequest};
use crate::dto::*;
use crate::error::{ApiError, ApiResult};
use crate::extract::{optional_json, ApiJson, ApiPath, ApiQuery, Paging};
use crate::AppState;

const MAX_PROMPT_CHARS: usize = 8000;

/// `POST /ai/proposals` — starts an asynchronous job (202).
pub async fn request_proposals(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ProposalRequestDto>,
) -> ApiResult<(StatusCode, Json<JobDto>)> {
    if req.preview_context {
        // The contract defines no response field for the previewed context yet.
        return Err(ApiError::bad_request(
            "previewContext is not supported until the API contract defines its response",
        ));
    }
    if let Some(p) = &req.prompt {
        check_len("prompt", p, 0, MAX_PROMPT_CHARS)?;
    }
    if let Some(c) = req.count {
        if !(1..=20).contains(&c) {
            return Err(ApiError::bad_request("count must be between 1 and 20"));
        }
    }
    let job = state
        .ai
        .start_job(ProposalJobRequest {
            kind: req.kind,
            feature_id: req.feature_id,
            model_id: req.model_id,
            prompt: req.prompt,
            count: req.count,
        })
        .await?;
    Ok((StatusCode::ACCEPTED, Json((&job).into())))
}

/// `GET /jobs/{jobId}`
pub async fn get_job(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<JobDto>> {
    Ok(Json((&state.store.get_job(id).await?).into()))
}

/// Query of `GET /projects/{projectId}/proposals`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalQuery {
    status: Option<ProposalStatus>,
    feature_id: Option<Uuid>,
    model_id: Option<Uuid>,
    limit: Option<i64>,
    cursor: Option<String>,
}

/// `GET /projects/{projectId}/proposals`
pub async fn list_proposals(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(q): ApiQuery<ProposalQuery>,
) -> ApiResult<Json<PageDto<ProposalDto>>> {
    let paging = Paging::new(q.limit, q.cursor).to_request()?;
    let filter = ProposalFilter {
        status: q.status,
        feature_id: q.feature_id,
        model_id: q.model_id,
    };
    let page = state.store.list_proposals(id, filter, paging).await?;
    Ok(Json(PageDto::from_page(page)))
}

/// `GET /proposals/{proposalId}`
pub async fn get_proposal(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<ProposalDto>> {
    Ok(Json((&state.store.get_proposal(id).await?).into()))
}

/// `POST /proposals/{proposalId}/accept` — creates the entity; optionally with an edited payload.
pub async fn accept_proposal(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    body: Bytes,
) -> ApiResult<Json<ProposalDto>> {
    let body: AcceptProposal = optional_json(&body)?.unwrap_or_default();
    let proposal = state.store.get_proposal(id).await?;
    if proposal.status != ProposalStatus::Pending {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            format!("proposal is already {}", proposal.status),
        ));
    }
    let payload = body.payload.unwrap_or_else(|| proposal.payload.clone());
    let acceptance = acceptance_for(&proposal, &payload)?;
    let accepted = state
        .store
        .accept_proposal(id, &payload, acceptance)
        .await?;
    Ok(Json((&accepted).into()))
}

/// `POST /proposals/{proposalId}/reject`
pub async fn reject_proposal(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    body: Bytes,
) -> ApiResult<Json<ProposalDto>> {
    let body: RejectProposal = optional_json(&body)?.unwrap_or_default();
    let rejected = state
        .store
        .reject_proposal(id, body.reason.as_deref())
        .await?;
    Ok(Json((&rejected).into()))
}

async fn settings_dto(state: &AppState, s: AiSettings) -> AiSettingsDto {
    AiSettingsDto {
        provider: s.provider,
        base_url: s.base_url,
        model: s.model,
        secret_configured: state.ai.secret_configured().await,
        max_tokens_per_request: s.max_tokens_per_request,
    }
}

/// `GET /settings/ai`
pub async fn get_ai_settings(State(state): State<AppState>) -> ApiResult<Json<AiSettingsDto>> {
    let s = state.store.get_ai_settings().await?;
    Ok(Json(settings_dto(&state, s).await))
}

/// `PUT /settings/ai` — the API key is kept in memory only and never returned.
pub async fn put_ai_settings(
    State(state): State<AppState>,
    ApiJson(input): ApiJson<AiSettingsInput>,
) -> ApiResult<Json<AiSettingsDto>> {
    if let Some(url) = input.base_url.as_deref().filter(|u| !u.is_empty()) {
        let ok = url.starts_with("https://") || url.starts_with("http://");
        if !ok {
            return Err(ApiError::bad_request("baseUrl must be an http(s) URL"));
        }
    }
    if input.max_tokens_per_request.is_some_and(|t| t < 1) {
        return Err(ApiError::bad_request("maxTokensPerRequest must be >= 1"));
    }
    let settings = AiSettings {
        provider: input.provider.unwrap_or(tm_domain::AiProvider::None),
        base_url: input.base_url.filter(|u| !u.is_empty()),
        model: input.model.filter(|m| !m.is_empty()),
        max_tokens_per_request: input.max_tokens_per_request,
    };
    let saved = state.store.put_ai_settings(&settings).await?;
    if let Some(key) = input.api_key {
        state.ai.set_api_key(key).await;
    }
    Ok(Json(settings_dto(&state, saved).await))
}
