//! AI proposals of a threat model (FR-085..FR-089). See
//! docs/specification/07-ai-integration.md#threat-model-assistant.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tm_ai::tm::payload::{
    ArchitecturePayload, ControlPayload, RatingBasis, RatingPayload, TestPayload, ThreatPayload,
};
use tm_ai::tm::{TmKind, TmPayload};
use tm_domain::threat::{
    self as t, ArchComponentFields, AssetFields, FlowFields, SecurityTestFields, ThreatFields,
    ZoneFields,
};
use tm_domain::ProposalStatus;
use tm_storage::{ControlApply, NewArchitecture, NewComponent, NewFlow, TmProposalRow};
use uuid::Uuid;

use crate::ai_service::TmJobRequest;
use crate::auth::Identity;
use crate::dto::{check_len, JobDto};
use crate::error::{ApiError, ApiResult};
use crate::extract::{optional_json, ApiJson, ApiPath, ApiQuery};
use crate::threat_dto::invalid;
use crate::AppState;

const MAX_PROMPT_CHARS: usize = 8000;
const MAX_TARGETS: usize = 50;

/// `TmProposalRequest`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmProposalRequestDto {
    pub kind: String,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub target_ids: Vec<Uuid>,
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(default)]
    pub preview_context: bool,
}

/// `TmContextPreview`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TmContextPreviewDto {
    pub context: String,
    pub truncated: bool,
    pub provider: String,
    pub endpoint_host: Option<String>,
}

/// `TmProposal`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TmProposalDto {
    pub id: Uuid,
    pub threat_model_id: Uuid,
    pub kind: String,
    pub status: String,
    pub payload: serde_json::Value,
    pub rationale: Option<String>,
    pub source: Option<String>,
    pub prompt_version: Option<String>,
    pub target_ids: Vec<Uuid>,
    pub resulting_entity_ids: Vec<Uuid>,
    pub reject_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<&TmProposalRow> for TmProposalDto {
    fn from(r: &TmProposalRow) -> Self {
        Self {
            id: r.id,
            threat_model_id: r.threat_model_id,
            kind: r.kind.clone(),
            status: r.status.clone(),
            payload: r.payload.clone(),
            rationale: r.rationale.clone(),
            source: r.source.clone(),
            prompt_version: r.prompt_version.clone(),
            target_ids: r.target_ids.clone(),
            resulting_entity_ids: r.resulting_entity_ids.clone(),
            reject_reason: r.reject_reason.clone(),
            created_at: r.created_at,
        }
    }
}

fn kind_of(s: &str) -> ApiResult<TmKind> {
    TmKind::parse(s).ok_or_else(|| {
        ApiError::unprocessable(format!(
            "kind: unknown proposal kind \"{s}\"; use one of tm-architecture, tm-threats, tm-ratings, tm-controls, tm-security-tests, tm-review"
        ))
    })
}

/// `POST /threat-models/{id}/ai/proposals` — a job (202), or the context that would be sent (200).
pub async fn request(
    State(state): State<AppState>,
    identity: Identity,
    ApiPath(tm): ApiPath<Uuid>,
    ApiJson(req): ApiJson<TmProposalRequestDto>,
) -> ApiResult<Response> {
    let kind = kind_of(&req.kind)?;
    if let Some(p) = &req.prompt {
        check_len("prompt", p, 0, MAX_PROMPT_CHARS)?;
    }
    if req.count.is_some_and(|c| !(1..=20).contains(&c)) {
        return Err(ApiError::bad_request("count must be between 1 and 20"));
    }
    if req.target_ids.len() > MAX_TARGETS {
        return Err(ApiError::bad_request("targetIds: at most 50"));
    }
    if req.preview_context {
        let p = state
            .ai
            .preview_tm_context(tm, kind, &req.target_ids)
            .await?;
        return Ok(Json(TmContextPreviewDto {
            context: p.context.text,
            truncated: p.context.truncated,
            provider: p.provider,
            endpoint_host: p.endpoint_host,
        })
        .into_response());
    }
    let job = state
        .ai
        .start_tm_job(
            &identity.tenant,
            tm,
            TmJobRequest {
                kind,
                prompt: req.prompt,
                target_ids: req.target_ids,
                count: req.count,
            },
        )
        .await?;
    Ok((StatusCode::ACCEPTED, Json(JobDto::from(&job))).into_response())
}

/// Query of the list.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    status: Option<ProposalStatus>,
    kind: Option<String>,
}

/// `GET /threat-models/{id}/ai/proposals`
pub async fn list(
    State(state): State<AppState>,
    ApiPath(tm): ApiPath<Uuid>,
    ApiQuery(q): ApiQuery<ListQuery>,
) -> ApiResult<Json<Vec<TmProposalDto>>> {
    let kind = q.kind.as_deref().map(kind_of).transpose()?;
    let rows = state
        .store
        .list_tm_proposals(
            tm,
            q.status.map(ProposalStatus::as_str),
            kind.map(TmKind::as_str),
        )
        .await?;
    Ok(Json(rows.iter().map(Into::into).collect()))
}

/// `GET /threat-models/{id}/ai/proposals/{itemId}`
pub async fn get(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
) -> ApiResult<Json<TmProposalDto>> {
    Ok(Json((&state.store.get_tm_proposal(tm, id).await?).into()))
}

/// Body of `POST …/accept`.
#[derive(Debug, Default, Deserialize)]
pub struct AcceptBody {
    #[serde(default)]
    payload: Option<serde_json::Value>,
}

/// Body of `POST …/reject`.
#[derive(Debug, Default, Deserialize)]
pub struct RejectBody {
    #[serde(default)]
    reason: Option<String>,
}

fn blank(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.trim().is_empty())
}

fn threat_fields(p: ThreatPayload) -> ThreatFields {
    ThreatFields {
        title: p.title,
        category: p.category,
        scenario: blank(p.scenario),
        attacker_objective: blank(p.attacker_objective),
        preconditions: blank(p.preconditions),
        consequences: blank(p.consequences),
        targets: p.targets,
        asset_ids: p.asset_ids,
        control_ids: vec![],
        // Accepting the proposal is the analyst's confirmation (ADR 0013).
        status: t::ThreatStatus::Open,
        dismiss_reason: None,
        inherent: None,
        current: None,
        residual: None,
        threat_source: blank(p.threat_source),
        detection: None,
        existing_mitigations: None,
        confidence: None,
        treatment: t::RiskTreatment::Undecided,
        risk_owner: None,
        due_date: None,
        acceptance_rationale: None,
        accepted_by: None,
        acceptance_expires: None,
    }
}

fn architecture(p: ArchitecturePayload) -> NewArchitecture {
    NewArchitecture {
        zones: p
            .zones
            .into_iter()
            .map(|z| ZoneFields {
                name: z.name.trim().to_owned(),
                description: blank(z.description),
                kind: z.kind,
                trust_level: z.trust_level,
            })
            .collect(),
        assets: p
            .assets
            .into_iter()
            .map(|a| AssetFields {
                name: a.name.trim().to_owned(),
                description: blank(a.description),
                category: a.category,
                owner: blank(a.owner),
                confidentiality: a.confidentiality,
                integrity: a.integrity,
                availability: a.availability,
                personal_data: a.personal_data,
                classification: a.classification,
                data_location: None,
                retention: None,
                encryption_at_rest: t::TriState::Unknown,
            })
            .collect(),
        components: p
            .components
            .into_iter()
            .map(|c| NewComponent {
                zone: blank(c.zone),
                assets: c.assets,
                fields: ArchComponentFields {
                    name: c.name.trim().to_owned(),
                    description: blank(c.description),
                    component_type: c.component_type,
                    owner: blank(c.owner),
                    technology: blank(c.technology),
                    criticality: c.criticality,
                    internet_facing: c.internet_facing,
                    zone_id: None,
                    linked_component_id: None,
                    linked_feature_id: None,
                    asset_ids: vec![],
                },
            })
            .collect(),
        flows: p
            .flows
            .into_iter()
            .map(|f| NewFlow {
                source: f.source,
                target: f.target,
                assets: f.assets,
                fields: FlowFields {
                    source_id: Uuid::nil(),
                    target_id: Uuid::nil(),
                    relationship: f.relationship,
                    label: blank(f.label),
                    protocol: blank(f.protocol),
                    bidirectional: false,
                    authentication: blank(f.authentication).unwrap_or_else(|| "unknown".into()),
                    encrypted: f.encrypted.unwrap_or(t::TriState::Unknown),
                    asset_ids: vec![],
                    data_classification: f.data_classification,
                    authorization_model: None,
                    entry_point: f.entry_point,
                },
            })
            .collect(),
    }
}

fn control(p: ControlPayload) -> ApiResult<ControlApply> {
    if let Some(control_id) = p.link_control_id {
        return Ok(ControlApply::Link {
            control_id,
            threat_ids: p.threat_ids,
        });
    }
    let (Some(name), Some(control_type), Some(category)) = (p.name, p.control_type, p.category)
    else {
        return Err(ApiError::unprocessable(
            "name, type and category are required for a new control",
        ));
    };
    Ok(ControlApply::New(t::ControlFields {
        name,
        description: blank(p.description),
        control_type,
        category,
        // Never verified or implemented by AI.
        status: t::ControlStatus::Planned,
        owner: None,
        acceptance_criteria: blank(p.acceptance_criteria),
        review_by: None,
        component_ids: p.component_ids,
        threat_ids: p.threat_ids,
    }))
}

fn test_fields(p: TestPayload) -> SecurityTestFields {
    SecurityTestFields {
        title: p.title,
        objective: blank(p.objective),
        preconditions: p
            .preconditions
            .into_iter()
            .filter(|x| !x.trim().is_empty())
            .collect(),
        steps: p.steps,
        test_type: p.test_type,
        // Drafts are reviewed before they become ready.
        status: t::SecurityTestStatus::Draft,
        threat_ids: p.threat_ids,
        control_ids: p.control_ids,
    }
}

fn input_of(agg: &tm_storage::ThreatModelAggregate) -> tm_ai::tm::TmContextInput<'_> {
    tm_ai::tm::TmContextInput {
        model: &agg.model.fields,
        thresholds: &agg.model.thresholds,
        arch: &agg.arch,
        threats: &agg.threats,
        controls: &agg.controls,
        tests: &agg.tests,
    }
}

/// `POST /threat-models/{id}/ai/proposals/{itemId}/accept`
///
/// The (possibly edited) payload is parsed with the same strictness as AI
/// output and validated against the threat model as it is now; a failure
/// leaves the proposal pending.
pub async fn accept(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
    body: Bytes,
) -> ApiResult<Json<TmProposalDto>> {
    let body: AcceptBody = optional_json(&body)?.unwrap_or_default();
    let proposal = state.store.get_tm_proposal(tm, id).await?;
    if proposal.status != "pending" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            format!("the proposal is already {}", proposal.status),
        ));
    }
    let kind = kind_of(&proposal.kind)?;
    if !kind.is_actionable() {
        return Err(ApiError::unprocessable(
            "a review is advice only; there is nothing to accept",
        ));
    }
    let json = body.payload.unwrap_or_else(|| proposal.payload.clone());
    let payload = TmPayload::from_json(kind, json.clone()).map_err(|e| {
        ApiError::unprocessable(format!("payload does not match the proposal kind: {e}"))
    })?;
    let agg = state.store.load_threat_model(tm).await?;
    tm_ai::tm::validate(&payload, &input_of(&agg))
        .map_err(|e| ApiError::unprocessable(e.to_string()))?;
    let model = proposal.source.clone().unwrap_or_else(|| "unknown".into());
    let json = payload.to_json();
    let row = match payload {
        TmPayload::Architecture(p) => {
            state
                .store
                .apply_tm_architecture(tm, id, &json, architecture(p), &model)
                .await?
        }
        TmPayload::Threat(p) => {
            let mut f = threat_fields(p);
            t::lifecycle::check_threat(&mut f, None).map_err(invalid)?;
            state
                .store
                .apply_tm_threat(tm, id, &json, &f, &model)
                .await?
        }
        TmPayload::Rating(RatingPayload {
            threat_id,
            rating,
            likelihood,
            impact,
        }) => {
            let r = t::Rating::new(likelihood, impact, "rating").map_err(invalid)?;
            let basis = match rating {
                RatingBasis::Inherent => "inherent",
                RatingBasis::Current => "current",
                RatingBasis::Residual => "residual",
            };
            state
                .store
                .apply_tm_rating(tm, id, &json, threat_id, basis, r)
                .await?
        }
        TmPayload::Control(p) => {
            state
                .store
                .apply_tm_control(tm, id, &json, control(p)?, &model)
                .await?
        }
        TmPayload::Test(p) => {
            state
                .store
                .apply_tm_test(tm, id, &json, &test_fields(p), &model)
                .await?
        }
        TmPayload::Review(_) => unreachable!("reviews are not actionable"),
    };
    Ok(Json((&row).into()))
}

/// `POST /threat-models/{id}/ai/proposals/{itemId}/reject`
pub async fn reject(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
    body: Bytes,
) -> ApiResult<Json<TmProposalDto>> {
    let body: RejectBody = optional_json(&body)?.unwrap_or_default();
    if let Some(r) = &body.reason {
        check_len("reason", r, 0, 2000)?;
    }
    let row = state
        .store
        .reject_tm_proposal(tm, id, blank(body.reason))
        .await?;
    Ok(Json((&row).into()))
}
