//! Threat models (FR-060..FR-076). See docs/specification/09-threat-modeling.md.
//!
//! Every write is followed by a read of the whole threat model, so the answer
//! carries the computed fields (crossings, warnings, levels, flags, staleness)
//! as of that write.

use std::collections::HashSet;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use tm_domain::threat::{
    assurance, lifecycle, rules, templates, ControlStatus, EvidenceOwnerKind, SecurityTestStatus,
    ThreatStatus,
};
use tm_storage::ThreatModelAggregate;
use tm_storage::{EvidenceOwner, NewBaseline};
use uuid::Uuid;

use crate::auth::Identity;
use crate::dto::{check_layout, check_len};
use crate::error::{ApiError, ApiResult};
use crate::extract::{optional_json, ApiJson, ApiPath, IfMatch};
use crate::threat_dto::*;
use crate::AppState;

async fn load(state: &AppState, tm: Uuid) -> ApiResult<ThreatModelAggregate> {
    Ok(state.store.load_threat_model(tm).await?)
}

fn view(agg: &ThreatModelAggregate) -> View<'_> {
    View::new(agg, Utc::now().date_naive())
}

fn not_found(entity: &str) -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, format!("{entity} not found"))
}

// ---------- threat model ----------

/// `GET /projects/{projectId}/threat-model`
pub async fn get_of_project(
    State(state): State<AppState>,
    ApiPath(project_id): ApiPath<Uuid>,
) -> ApiResult<Json<ThreatModelDto>> {
    state.store.get_project(project_id).await?;
    Ok(Json(
        (&state.store.threat_model_of_project(project_id).await?).into(),
    ))
}

/// `POST /projects/{projectId}/threat-model`
pub async fn create(
    State(state): State<AppState>,
    ApiPath(project_id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<ThreatModelInput>,
) -> ApiResult<(StatusCode, Json<ThreatModelDto>)> {
    let (fields, thresholds) = input.into_parts(Default::default(), false)?;
    let m = state
        .store
        .create_threat_model(project_id, fields, thresholds)
        .await?;
    Ok((StatusCode::CREATED, Json((&m).into())))
}

/// `GET /threat-models/{id}`
pub async fn get(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<ThreatModelDto>> {
    Ok(Json((&state.store.get_threat_model(id).await?).into()))
}

/// `PUT /threat-models/{id}` — scope and thresholds; absent thresholds are kept.
pub async fn replace(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    IfMatch(version): IfMatch,
    ApiJson(input): ApiJson<ThreatModelInput>,
) -> ApiResult<Json<ThreatModelDto>> {
    let current = state.store.get_threat_model(id).await?;
    let (fields, thresholds) = input.into_parts(current.thresholds, current.fields.ai_enabled)?;
    let m = state
        .store
        .update_threat_model(id, version, fields, thresholds)
        .await?;
    Ok(Json((&m).into()))
}

/// `DELETE /threat-models/{id}`
pub async fn delete(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<StatusCode> {
    state.store.delete_threat_model(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `PUT /threat-models/{id}/layout` with `If-Match: <layoutVersion>` (0 allowed).
pub async fn save_layout(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<LayoutSaved>> {
    let expected = headers
        .get(axum::http::header::IF_MATCH)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().trim_start_matches("W/").trim_matches('"'))
        .and_then(|v| v.parse::<i32>().ok())
        .filter(|v| *v >= 0)
        .ok_or_else(|| {
            ApiError::bad_request("If-Match header with the layoutVersion is required")
        })?;
    let layout: Option<serde_json::Value> =
        optional_json::<serde_json::Value>(&body)?.filter(|v| !v.is_null());
    if let Some(l) = &layout {
        check_layout(l)?;
    }
    let layout_version = state
        .store
        .save_threat_model_layout(id, expected, layout)
        .await?;
    Ok(Json(LayoutSaved { layout_version }))
}

/// `GET /threat-models/{id}/summary`
pub async fn summary(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<SummaryDto>> {
    let agg = load(&state, id).await?;
    Ok(Json(view(&agg).summary()))
}

// ---------- plain collections ----------

/// Generates list/create/get/replace/delete handlers for a collection whose
/// input needs no checks beyond its own and the storage's.
macro_rules! collection {
    ($m:ident, $input:ty, $dto:ty, [$($path:ident).+], $to_dto:ident, $entity:literal,
     $create:ident, $update:ident, $delete:ident) => {
        pub mod $m {
            use super::*;

            async fn one(state: &AppState, tm: Uuid, id: Uuid) -> ApiResult<$dto> {
                let agg = load(state, tm).await?;
                let v = view(&agg);
                let items = agg.$($path).+.as_slice();
                let item = items
                    .iter()
                    .find(|x| x.audit.id == id)
                    .ok_or_else(|| not_found($entity))?;
                Ok(v.$to_dto(item))
            }

            /// List.
            pub async fn list(
                State(state): State<AppState>,
                ApiPath(tm): ApiPath<Uuid>,
            ) -> ApiResult<Json<Vec<$dto>>> {
                let agg = load(&state, tm).await?;
                let v = view(&agg);
                let items = agg.$($path).+.as_slice();
                Ok(Json(items.iter().map(|x| v.$to_dto(x)).collect()))
            }

            /// Create.
            pub async fn create(
                State(state): State<AppState>,
                ApiPath(tm): ApiPath<Uuid>,
                ApiJson(input): ApiJson<$input>,
            ) -> ApiResult<(StatusCode, Json<$dto>)> {
                let fields = input.into_fields()?;
                state.store.get_threat_model(tm).await?;
                let id = state.store.$create(tm, &fields).await?;
                Ok((StatusCode::CREATED, Json(one(&state, tm, id).await?)))
            }

            /// Get.
            pub async fn get(
                State(state): State<AppState>,
                ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
            ) -> ApiResult<Json<$dto>> {
                Ok(Json(one(&state, tm, id).await?))
            }

            /// Replace.
            pub async fn replace(
                State(state): State<AppState>,
                ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
                IfMatch(version): IfMatch,
                ApiJson(input): ApiJson<$input>,
            ) -> ApiResult<Json<$dto>> {
                let fields = input.into_fields()?;
                state.store.$update(tm, id, version, &fields).await?;
                Ok(Json(one(&state, tm, id).await?))
            }

            /// Delete.
            pub async fn delete(
                State(state): State<AppState>,
                ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
            ) -> ApiResult<StatusCode> {
                state.store.$delete(tm, id).await?;
                Ok(StatusCode::NO_CONTENT)
            }
        }
    };
}

collection!(
    zones,
    ZoneInput,
    ZoneDto,
    [arch.zones],
    zone,
    "zone",
    create_tm_zone,
    update_tm_zone,
    delete_tm_zone
);
collection!(
    components,
    ArchComponentInput,
    ArchComponentDto,
    [arch.components],
    component,
    "component",
    create_tm_component,
    update_tm_component,
    delete_tm_component
);
collection!(
    assets,
    AssetInput,
    AssetDto,
    [arch.assets],
    asset,
    "asset",
    create_tm_asset,
    update_tm_asset,
    delete_tm_asset
);
collection!(
    flows,
    FlowInput,
    FlowDto,
    [arch.flows],
    flow,
    "flow",
    create_tm_flow,
    update_tm_flow,
    delete_tm_flow
);
collection!(
    tests,
    SecurityTestInput,
    SecurityTestDto,
    [tests],
    test,
    "security test",
    create_security_test,
    update_security_test,
    delete_security_test
);

// ---------- threats ----------

async fn threat_dtos(state: &AppState, tm: Uuid, ids: &[Uuid]) -> ApiResult<Vec<ThreatDto>> {
    let agg = load(state, tm).await?;
    let v = view(&agg);
    Ok(ids
        .iter()
        .filter_map(|id| v.threat(*id))
        .map(|t| v.threat_dto(t))
        .collect())
}

/// `GET /threat-models/{id}/threats`
pub async fn list_threats(
    State(state): State<AppState>,
    ApiPath(tm): ApiPath<Uuid>,
) -> ApiResult<Json<Vec<ThreatDto>>> {
    let agg = load(&state, tm).await?;
    let v = view(&agg);
    Ok(Json(agg.threats.iter().map(|t| v.threat_dto(t)).collect()))
}

/// `POST /threat-models/{id}/threats` — a manual threat.
pub async fn create_threat(
    State(state): State<AppState>,
    ApiPath(tm): ApiPath<Uuid>,
    ApiJson(input): ApiJson<ThreatInput>,
) -> ApiResult<(StatusCode, Json<ThreatDto>)> {
    let mut fields = input.into_fields()?;
    let today = Utc::now().date_naive();
    lifecycle::check_threat_on(&mut fields, None, today).map_err(invalid)?;
    state.store.get_threat_model(tm).await?;
    let accepted_on = assurance::acceptance_date(None, &fields, today);
    let id = state
        .store
        .create_tm_threat(tm, &fields, accepted_on)
        .await?;
    let dto = threat_dtos(&state, tm, &[id])
        .await?
        .pop()
        .ok_or_else(|| not_found("threat"))?;
    Ok((StatusCode::CREATED, Json(dto)))
}

/// `GET /threat-models/{id}/threats/{itemId}`
pub async fn get_threat(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
) -> ApiResult<Json<ThreatDto>> {
    let dto = threat_dtos(&state, tm, &[id])
        .await?
        .pop()
        .ok_or_else(|| not_found("threat"))?;
    Ok(Json(dto))
}

/// `PUT /threat-models/{id}/threats/{itemId}`
pub async fn replace_threat(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
    IfMatch(version): IfMatch,
    ApiJson(input): ApiJson<ThreatInput>,
) -> ApiResult<Json<ThreatDto>> {
    let mut fields = input.into_fields()?;
    let agg = load(&state, tm).await?;
    let previous = agg
        .threats
        .iter()
        .find(|t| t.audit.id == id)
        .ok_or_else(|| not_found("threat"))?;
    let today = Utc::now().date_naive();
    lifecycle::check_threat_on(&mut fields, Some(&previous.fields), today).map_err(invalid)?;
    let accepted_on = assurance::acceptance_date(Some(previous), &fields, today);
    state
        .store
        .update_tm_threat(tm, id, version, &fields, accepted_on)
        .await?;
    let dto = threat_dtos(&state, tm, &[id])
        .await?
        .pop()
        .ok_or_else(|| not_found("threat"))?;
    Ok(Json(dto))
}

/// `DELETE /threat-models/{id}/threats/{itemId}`
pub async fn delete_threat(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    state.store.delete_tm_threat(tm, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /threat-models/{id}/threats/suggest`
pub async fn suggest_threats(
    State(state): State<AppState>,
    ApiPath(tm): ApiPath<Uuid>,
) -> ApiResult<Json<SuggestThreatsResult>> {
    let agg = load(&state, tm).await?;
    let suggestion = rules::suggest(&agg.arch, &agg.rule_keys);
    let ids = state
        .store
        .create_tm_candidates(tm, &suggestion.candidates)
        .await?;
    let skipped = suggestion.skipped + (suggestion.candidates.len() - ids.len());
    Ok(Json(SuggestThreatsResult {
        ruleset_version: rules::RULESET_VERSION,
        created: threat_dtos(&state, tm, &ids).await?,
        skipped,
    }))
}

/// `POST /threat-models/{id}/threats/bulk-status` — confirm or dismiss.
pub async fn bulk_threat_status(
    State(state): State<AppState>,
    ApiPath(tm): ApiPath<Uuid>,
    ApiJson(input): ApiJson<ThreatBulkStatus>,
) -> ApiResult<Json<Vec<ThreatDto>>> {
    if input.ids.is_empty() {
        return Err(invalid(tm_domain::threat::ThreatModelError::new(
            "ids",
            "must not be empty",
        )));
    }
    if !matches!(input.status, ThreatStatus::Open | ThreatStatus::Dismissed) {
        return Err(invalid(tm_domain::threat::ThreatModelError::new(
            "status",
            "bulk changes only confirm (open) or dismiss",
        )));
    }
    let agg = load(&state, tm).await?;
    let mut changes = Vec::with_capacity(input.ids.len());
    for id in &input.ids {
        let previous = agg
            .threats
            .iter()
            .find(|t| t.audit.id == *id)
            .ok_or_else(|| not_found("threat"))?;
        let mut next = previous.fields.clone();
        next.status = input.status;
        if input.status == ThreatStatus::Dismissed {
            next.dismiss_reason = input.reason.clone().filter(|r| !r.trim().is_empty());
        }
        lifecycle::check_threat(&mut next, Some(&previous.fields)).map_err(invalid)?;
        changes.push((*id, next.status, next.dismiss_reason));
    }
    state.store.set_tm_threat_statuses(tm, &changes).await?;
    Ok(Json(threat_dtos(&state, tm, &input.ids).await?))
}

// ---------- security test actions ----------

async fn test_dtos(state: &AppState, tm: Uuid, ids: &[Uuid]) -> ApiResult<Vec<SecurityTestDto>> {
    let agg = load(state, tm).await?;
    let v = view(&agg);
    Ok(agg
        .tests
        .iter()
        .filter(|t| ids.contains(&t.audit.id))
        .map(|t| v.test(t))
        .collect())
}

/// `POST /threat-models/{id}/security-tests/generate`
pub async fn generate_tests(
    State(state): State<AppState>,
    ApiPath(tm): ApiPath<Uuid>,
    body: Bytes,
) -> ApiResult<Json<GenerateResult>> {
    let req: GenerateRequest = optional_json::<Option<GenerateRequest>>(&body)?
        .flatten()
        .unwrap_or_default();
    let agg = load(&state, tm).await?;
    let threats: Vec<_> = match &req.threat_ids {
        Some(ids) => ids
            .iter()
            .map(|id| {
                agg.threats
                    .iter()
                    .find(|t| t.audit.id == *id)
                    .ok_or_else(|| not_found("threat"))
            })
            .collect::<ApiResult<_>>()?,
        None => agg.threats.iter().collect(),
    };
    let existing: HashSet<String> = agg
        .tests
        .iter()
        .filter(|t| t.fields.status != SecurityTestStatus::Obsolete)
        .filter_map(|t| t.source.as_ref().map(|s| s.key()))
        .collect();
    let v = view(&agg);
    let generated = templates::generate(&threats, &agg.arch, v.controls(), &existing);
    let ids = state
        .store
        .create_generated_tests(tm, &generated.tests)
        .await?;
    let skipped = generated.skipped + (generated.tests.len() - ids.len());
    Ok(Json(GenerateResult {
        created: test_dtos(&state, tm, &ids).await?,
        skipped,
    }))
}

/// `POST /threat-models/{id}/security-tests/bulk-status`
pub async fn bulk_test_status(
    State(state): State<AppState>,
    ApiPath(tm): ApiPath<Uuid>,
    ApiJson(input): ApiJson<TestBulkStatus>,
) -> ApiResult<Json<Vec<SecurityTestDto>>> {
    if input.ids.is_empty() {
        return Err(invalid(tm_domain::threat::ThreatModelError::new(
            "ids",
            "must not be empty",
        )));
    }
    state
        .store
        .set_security_test_statuses(tm, &input.ids, input.status)
        .await?;
    Ok(Json(test_dtos(&state, tm, &input.ids).await?))
}

/// `POST /threat-models/{id}/security-tests/{itemId}/results`
pub async fn record_result(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
    ApiJson(input): ApiJson<ResultInput>,
) -> ApiResult<(StatusCode, Json<ResultDto>)> {
    let agg = load(&state, tm).await?;
    let test = agg
        .tests
        .iter()
        .find(|t| t.audit.id == id)
        .ok_or_else(|| not_found("security test"))?;
    if test.fields.status != SecurityTestStatus::Ready {
        return Err(invalid(tm_domain::threat::ThreatModelError::new(
            "status",
            "only a ready security test takes results",
        )));
    }
    if input
        .note
        .as_deref()
        .is_some_and(|n| n.chars().count() > 2000)
    {
        return Err(invalid(tm_domain::threat::ThreatModelError::new(
            "note",
            "at most 2000 characters",
        )));
    }
    let note = input.note.filter(|n| !n.trim().is_empty());
    let r = state
        .store
        .record_security_test_result(
            tm,
            id,
            input.outcome,
            note,
            input.executed_at.unwrap_or_else(Utc::now),
        )
        .await?;
    Ok((StatusCode::CREATED, Json((&r).into())))
}

/// `GET /threat-models/{id}/security-tests/{itemId}/results`
pub async fn results(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
) -> ApiResult<Json<Vec<ResultDto>>> {
    let rs = state.store.security_test_results(tm, id).await?;
    Ok(Json(rs.iter().map(Into::into).collect()))
}

// ---------- controls ----------

/// Controls need evidence to become verified (FR-078).
pub mod controls {
    use super::*;

    /// List.
    pub async fn list(
        State(state): State<AppState>,
        ApiPath(tm): ApiPath<Uuid>,
    ) -> ApiResult<Json<Vec<ControlDto>>> {
        let agg = load(&state, tm).await?;
        let v = view(&agg);
        Ok(Json(agg.controls.iter().map(|x| v.control(x)).collect()))
    }

    async fn one(state: &AppState, tm: Uuid, id: Uuid) -> ApiResult<ControlDto> {
        let agg = load(state, tm).await?;
        let v = view(&agg);
        agg.controls
            .iter()
            .find(|c| c.audit.id == id)
            .map(|c| v.control(c))
            .ok_or_else(|| not_found("control"))
    }

    /// Create. A new control has no evidence yet, so it cannot be verified.
    pub async fn create(
        State(state): State<AppState>,
        ApiPath(tm): ApiPath<Uuid>,
        ApiJson(input): ApiJson<ControlInput>,
    ) -> ApiResult<(StatusCode, Json<ControlDto>)> {
        let fields = input.into_fields()?;
        if fields.status == ControlStatus::Verified {
            assurance::check_can_verify(0).map_err(invalid)?;
        }
        state.store.get_threat_model(tm).await?;
        let id = state.store.create_tm_control(tm, &fields).await?;
        Ok((StatusCode::CREATED, Json(one(&state, tm, id).await?)))
    }

    /// Get.
    pub async fn get(
        State(state): State<AppState>,
        ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
    ) -> ApiResult<Json<ControlDto>> {
        Ok(Json(one(&state, tm, id).await?))
    }

    /// Replace. Becoming verified needs at least one evidence record; a control
    /// that was verified before evidence existed can still be edited.
    pub async fn replace(
        State(state): State<AppState>,
        ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
        IfMatch(version): IfMatch,
        ApiJson(input): ApiJson<ControlInput>,
    ) -> ApiResult<Json<ControlDto>> {
        let fields = input.into_fields()?;
        if fields.status == ControlStatus::Verified {
            let agg = load(&state, tm).await?;
            let previous = agg
                .controls
                .iter()
                .find(|c| c.audit.id == id)
                .ok_or_else(|| not_found("control"))?;
            if previous.fields.status != ControlStatus::Verified {
                let n = state.store.control_evidence_count(tm, id).await?;
                assurance::check_can_verify(n).map_err(invalid)?;
            }
        }
        state
            .store
            .update_tm_control(tm, id, version, &fields)
            .await?;
        Ok(Json(one(&state, tm, id).await?))
    }

    /// Delete.
    pub async fn delete(
        State(state): State<AppState>,
        ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
    ) -> ApiResult<StatusCode> {
        state.store.delete_tm_control(tm, id).await?;
        Ok(StatusCode::NO_CONTENT)
    }
}

// ---------- evidence ----------

fn evidence_dtos(list: &[assurance::Evidence]) -> Vec<EvidenceDto> {
    list.iter().map(Into::into).collect()
}

/// `GET /threat-models/{id}/controls/{itemId}/evidence`
pub async fn control_evidence(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
) -> ApiResult<Json<Vec<EvidenceDto>>> {
    let list = state
        .store
        .list_evidence(tm, EvidenceOwner::Control(id))
        .await?;
    Ok(Json(evidence_dtos(&list)))
}

/// `POST /threat-models/{id}/controls/{itemId}/evidence`
pub async fn add_control_evidence(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
    ApiJson(input): ApiJson<EvidenceInput>,
) -> ApiResult<(StatusCode, Json<EvidenceDto>)> {
    let f = input.into_fields()?;
    let e = state
        .store
        .add_evidence(tm, EvidenceOwner::Control(id), &f)
        .await?;
    Ok((StatusCode::CREATED, Json((&e).into())))
}

/// `GET …/security-tests/{itemId}/results/{resultId}/evidence`
pub async fn result_evidence(
    State(state): State<AppState>,
    ApiPath((tm, test_id, result_id)): ApiPath<(Uuid, Uuid, Uuid)>,
) -> ApiResult<Json<Vec<EvidenceDto>>> {
    let list = state
        .store
        .list_evidence(tm, EvidenceOwner::TestResult { test_id, result_id })
        .await?;
    Ok(Json(evidence_dtos(&list)))
}

/// `POST …/security-tests/{itemId}/results/{resultId}/evidence`
pub async fn add_result_evidence(
    State(state): State<AppState>,
    ApiPath((tm, test_id, result_id)): ApiPath<(Uuid, Uuid, Uuid)>,
    ApiJson(input): ApiJson<EvidenceInput>,
) -> ApiResult<(StatusCode, Json<EvidenceDto>)> {
    let f = input.into_fields()?;
    let e = state
        .store
        .add_evidence(tm, EvidenceOwner::TestResult { test_id, result_id }, &f)
        .await?;
    debug_assert_eq!(e.owner_kind, EvidenceOwnerKind::TestResult);
    Ok((StatusCode::CREATED, Json((&e).into())))
}

/// `DELETE /threat-models/{id}/evidence/{itemId}`
pub async fn delete_evidence(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    state.store.delete_evidence(tm, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- baselines ----------

/// `GET /threat-models/{id}/baselines`
pub async fn list_baselines(
    State(state): State<AppState>,
    ApiPath(tm): ApiPath<Uuid>,
) -> ApiResult<Json<Vec<BaselineDto>>> {
    state.store.get_threat_model(tm).await?;
    let list = state.store.list_baselines(tm).await?;
    Ok(Json(list.iter().map(Into::into).collect()))
}

/// `POST /threat-models/{id}/baselines`
pub async fn create_baseline(
    State(state): State<AppState>,
    identity: Identity,
    ApiPath(tm): ApiPath<Uuid>,
    ApiJson(input): ApiJson<BaselineInput>,
) -> ApiResult<(StatusCode, Json<BaselineDto>)> {
    check_len("name", &input.name, 1, 200)?;
    if let Some(n) = &input.note {
        check_len("note", n, 0, 2000)?;
    }
    let agg = load(&state, tm).await?;
    let snapshot = view(&agg).snapshot();
    let meta = state
        .store
        .create_baseline(
            tm,
            NewBaseline {
                name: input.name.trim().to_owned(),
                note: input.note.filter(|n| !n.trim().is_empty()),
                created_by: identity.subject,
                schema_version: SNAPSHOT_VERSION,
                threat_count: i32::try_from(snapshot.threats.len()).unwrap_or(i32::MAX),
                control_count: i32::try_from(snapshot.controls.len()).unwrap_or(i32::MAX),
                snapshot: serde_json::to_value(&snapshot)
                    .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json((&meta).into())))
}

async fn snapshot_of(
    state: &AppState,
    tm: Uuid,
    id: Uuid,
) -> ApiResult<(tm_storage::BaselineMeta, SnapshotDto)> {
    let (meta, value) = state.store.get_baseline(tm, id).await?;
    let mut snap: SnapshotDto = serde_json::from_value(value).map_err(|e| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("the stored baseline cannot be read: {e}"),
        )
    })?;
    snap.refresh(Utc::now().date_naive());
    Ok((meta, snap))
}

/// `GET /threat-models/{id}/baselines/{itemId}`
pub async fn get_baseline(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
) -> ApiResult<Json<BaselineDetailDto>> {
    let (meta, snapshot) = snapshot_of(&state, tm, id).await?;
    Ok(Json(BaselineDetailDto {
        meta: (&meta).into(),
        snapshot,
    }))
}

/// `DELETE /threat-models/{id}/baselines/{itemId}`
pub async fn delete_baseline(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    state.store.delete_baseline(tm, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /threat-models/{id}/baselines/{itemId}/compare`
pub async fn compare_baseline(
    State(state): State<AppState>,
    ApiPath((tm, id)): ApiPath<(Uuid, Uuid)>,
) -> ApiResult<Json<ComparisonDto>> {
    let (_, snapshot) = snapshot_of(&state, tm, id).await?;
    let agg = load(&state, tm).await?;
    let (live_threats, live_controls) = view(&agg).digests();
    let (base_threats, base_controls) = snapshot.digests();
    Ok(Json(
        assurance::diff(&base_threats, &live_threats, &base_controls, &live_controls).into(),
    ))
}
