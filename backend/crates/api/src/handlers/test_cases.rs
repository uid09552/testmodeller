//! Test cases, assignments, generation and coverage.

use std::collections::HashSet;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use tm_domain::{Assignment, AssignmentTarget, Audit, Origin, TestCase, TestCaseStatus};
use tm_generation::{CoverageReport, GenerationError, GenerationOptions};
use tm_storage::{NewAssignment, TestCaseFilter, TraceFilter, TraceGap};
use uuid::Uuid;

use crate::dto::*;
use crate::error::{ApiError, ApiResult, FieldError};
use crate::extract::{ApiJson, ApiPath, ApiQuery, IfMatch, Paging};
use crate::AppState;

/// Query of `GET /features/{featureId}/test-cases`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCaseQuery {
    limit: Option<i64>,
    cursor: Option<String>,
    model_id: Option<Uuid>,
    status: Option<TestCaseStatus>,
    origin: Option<Origin>,
    #[serde(default)]
    unassigned: bool,
}

/// `GET /features/{featureId}/test-cases`
pub async fn list_test_cases(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(q): ApiQuery<TestCaseQuery>,
) -> ApiResult<Json<PageDto<TestCaseDto>>> {
    let paging = Paging::new(q.limit, q.cursor).to_request()?;
    let filter = TestCaseFilter {
        model_id: q.model_id,
        status: q.status,
        origin: q.origin,
        unassigned: q.unassigned,
    };
    let page = state.store.list_test_cases(id, filter, paging).await?;
    Ok(Json(PageDto::from_page(page)))
}

/// `POST /features/{featureId}/test-cases`
pub async fn create_test_case(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<TestCaseInput>,
) -> ApiResult<(StatusCode, Json<TestCaseDto>)> {
    let (data, assignments) = input.into_parts()?;
    let tc = state
        .store
        .create_test_case(id, &data, Origin::Manual, &assignments.unwrap_or_default())
        .await?;
    Ok((StatusCode::CREATED, Json((&tc).into())))
}

/// `GET /test-cases/{testCaseId}`
pub async fn get_test_case(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<TestCaseDto>> {
    Ok(Json((&state.store.get_test_case(id).await?).into()))
}

/// `PUT /test-cases/{testCaseId}` — replaces fields and steps; assignments only if given.
pub async fn replace_test_case(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    IfMatch(version): IfMatch,
    ApiJson(input): ApiJson<TestCaseInput>,
) -> ApiResult<Json<TestCaseDto>> {
    let (data, assignments) = input.into_parts()?;
    let tc = state
        .store
        .replace_test_case(id, version, &data, assignments.as_deref())
        .await?;
    Ok(Json((&tc).into()))
}

/// `DELETE /test-cases/{testCaseId}`
pub async fn delete_test_case(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<StatusCode> {
    state.store.delete_test_case(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /test-cases/{testCaseId}/move`
pub async fn move_test_case(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(body): ApiJson<MoveTestCase>,
) -> ApiResult<Json<TestCaseDto>> {
    let tc = state.store.move_test_case(id, body.feature_id).await?;
    Ok(Json((&tc).into()))
}

/// `GET /test-cases/{testCaseId}/assignments`
pub async fn list_assignments(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<Vec<AssignmentDto>>> {
    let list = state.store.list_assignments(id).await?;
    Ok(Json(list.iter().map(Into::into).collect()))
}

/// `PUT /test-cases/{testCaseId}/assignments`
pub async fn replace_assignments(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<Vec<AssignmentInput>>,
) -> ApiResult<Json<Vec<AssignmentDto>>> {
    let list = state
        .store
        .replace_assignments(id, &AssignmentInput::into_list(input)?)
        .await?;
    Ok(Json(list.iter().map(Into::into).collect()))
}

fn to_dtos(cases: &[TestCase]) -> Json<Vec<TestCaseDto>> {
    Json(cases.iter().map(Into::into).collect())
}

/// `GET /states/{stateId}/test-cases`
pub async fn state_test_cases(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<Vec<TestCaseDto>>> {
    let cases = state
        .store
        .test_cases_for_target(AssignmentTarget::State(id))
        .await?;
    Ok(to_dtos(&cases))
}

/// `GET /transitions/{transitionId}/test-cases`
pub async fn transition_test_cases(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<Vec<TestCaseDto>>> {
    let cases = state
        .store
        .test_cases_for_target(AssignmentTarget::Transition(id))
        .await?;
    Ok(to_dtos(&cases))
}

/// `GET /models/{modelId}/test-cases`
pub async fn model_test_cases(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<Vec<TestCaseDto>>> {
    Ok(to_dtos(&state.store.test_cases_for_model(id).await?))
}

/// `PUT /states/{stateId}/test-cases/{testCaseId}`
pub async fn assign_state(
    State(state): State<AppState>,
    ApiPath((target, tc)): ApiPath<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    state
        .store
        .assign(tc, AssignmentTarget::State(target))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /states/{stateId}/test-cases/{testCaseId}`
pub async fn unassign_state(
    State(state): State<AppState>,
    ApiPath((target, tc)): ApiPath<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    state
        .store
        .unassign(tc, AssignmentTarget::State(target))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `PUT /transitions/{transitionId}/test-cases/{testCaseId}`
pub async fn assign_transition(
    State(state): State<AppState>,
    ApiPath((target, tc)): ApiPath<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    state
        .store
        .assign(tc, AssignmentTarget::Transition(target))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /transitions/{transitionId}/test-cases/{testCaseId}`
pub async fn unassign_transition(
    State(state): State<AppState>,
    ApiPath((target, tc)): ApiPath<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    state
        .store
        .unassign(tc, AssignmentTarget::Transition(target))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /models/{modelId}/generate` (FR-020, FR-021)
pub async fn generate(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(req): ApiJson<GenerationRequest>,
) -> ApiResult<Json<GenerationResultDto>> {
    let model = state.store.get_model(id).await?;
    let seed = req
        .seed
        .unwrap_or_else(|| (Uuid::new_v4().as_u64_pair().0 >> 1) as i64);
    let max_path_length = match req.max_path_length {
        Some(n) if n < 1 => return Err(ApiError::bad_request("maxPathLength must be >= 1")),
        Some(n) => Some(usize::try_from(n).unwrap_or(usize::MAX)),
        None => None,
    };
    let options = GenerationOptions {
        criterion: req.criterion,
        seed: seed as u64,
        max_path_length,
    };
    let graph = model.graph.clone();
    let output = tokio::task::spawn_blocking(move || tm_generation::generate(&graph, options))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "generation aborted"))?
        .map_err(|e| match e {
            GenerationError::InvalidModel(issues) => {
                ApiError::unprocessable("model has validation errors; fix them before generating")
                    .with_errors(
                        issues
                            .iter()
                            .filter(|i| i.severity == tm_domain::Severity::Error)
                            .map(|i| FieldError {
                                field: i.code.into(),
                                message: i.message.clone(),
                            })
                            .collect(),
                    )
            }
            GenerationError::MissingMaxPathLength => ApiError::unprocessable(e.to_string()),
        })?;

    let cases: Vec<(tm_domain::TestCaseData, Vec<NewAssignment>)> = output
        .paths
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let (data, targets) = tm_generation::path_to_test_case(
                &model.graph,
                &model.summary.name,
                req.criterion,
                i,
                path,
            );
            let assignments = targets
                .into_iter()
                .map(|(target, step_order)| NewAssignment {
                    model_id: id,
                    target,
                    step_order,
                })
                .collect();
            (data, assignments)
        })
        .collect();

    let test_cases: Vec<TestCaseDto> = if req.save {
        state
            .store
            .save_generated(id, &cases)
            .await?
            .iter()
            .map(Into::into)
            .collect()
    } else {
        let now = Utc::now();
        cases
            .into_iter()
            .map(|(data, assignments)| {
                let tc_id = Uuid::new_v4();
                let tc = TestCase {
                    audit: Audit {
                        id: tc_id,
                        version: 1,
                        created_at: now,
                        updated_at: now,
                    },
                    feature_id: model.summary.feature_id,
                    data,
                    origin: Origin::Generated,
                    generated_from_model_id: Some(id),
                    assignments: assignments
                        .into_iter()
                        .map(|a| Assignment {
                            test_case_id: tc_id,
                            model_id: a.model_id,
                            target: a.target,
                            step_order: a.step_order,
                        })
                        .collect(),
                    last_result: None,
                };
                (&tc).into()
            })
            .collect()
    };

    Ok(Json(GenerationResultDto {
        seed,
        test_cases,
        coverage: (&output.coverage).into(),
        skipped_paths: output
            .skipped
            .into_iter()
            .map(|s| SkippedPathDto {
                transition_ids: s.transition_ids,
                reason: s.reason,
            })
            .collect(),
    }))
}

async fn coverage_of(
    state: &AppState,
    graphs: Vec<(Uuid, tm_domain::ModelGraph)>,
) -> ApiResult<CoverageDto> {
    let ids: Vec<Uuid> = graphs.iter().map(|(id, _)| *id).collect();
    let (states, transitions): (HashSet<Uuid>, HashSet<Uuid>) =
        state.store.covered_elements(&ids).await?;
    let (passing_states, passing_transitions) = state.store.passing_elements(&ids).await?;
    let mut total = CoverageReport::default();
    let (mut ps, mut pt) = (0, 0);
    for (_, graph) in &graphs {
        total.merge(CoverageReport::compute(graph, &states, &transitions));
        ps += graph
            .states
            .iter()
            .filter(|s| passing_states.contains(&s.id))
            .count();
        pt += graph
            .transitions
            .iter()
            .filter(|t| passing_transitions.contains(&t.id))
            .count();
    }
    let mut dto: CoverageDto = (&total).into();
    dto.states.passing = Some(ps);
    dto.transitions.passing = Some(pt);
    Ok(dto)
}

/// `GET /models/{modelId}/stale-tests`: generated test cases whose path no longer fits
/// the model. Read-only. See docs/specification/04-test-generation.md#stale-tests.
pub async fn stale_tests(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<Vec<StaleTestDto>>> {
    let graph = state.store.get_model_graph(id).await?;
    let cases = state.store.generated_test_cases(id).await?;
    let stale = cases
        .iter()
        .filter_map(|tc| {
            let assigned: Vec<_> = tc
                .assignments
                .iter()
                .filter(|a| a.model_id == id)
                .map(|a| (a.target, a.step_order))
                .collect();
            let reasons = tm_generation::check_path(&graph, tc.data.steps.len(), &assigned);
            (!reasons.is_empty()).then(|| StaleTestDto {
                test_case_id: tc.audit.id,
                reasons: reasons.iter().map(Into::into).collect(),
            })
        })
        .collect();
    Ok(Json(stale))
}

/// `GET /models/{modelId}/coverage` (FR-022)
pub async fn model_coverage(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<CoverageDto>> {
    let graph = state.store.get_model_graph(id).await?;
    Ok(Json(coverage_of(&state, vec![(id, graph)]).await?))
}

/// `GET /features/{featureId}/coverage`
pub async fn feature_coverage(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<CoverageDto>> {
    let graphs = state.store.feature_graphs(id).await?;
    Ok(Json(coverage_of(&state, graphs).await?))
}

/// `GET /components/{componentId}/coverage`
pub async fn component_coverage(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> ApiResult<Json<CoverageDto>> {
    let graphs = state.store.component_graphs(id).await?;
    Ok(Json(coverage_of(&state, graphs).await?))
}

/// Query of `GET /projects/{projectId}/traceability`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceabilityQuery {
    component_id: Option<Uuid>,
    feature_id: Option<Uuid>,
    gap: Option<GapParam>,
}

/// Wire form of the `gap` query parameter.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum GapParam {
    Untraced,
    Unimplemented,
    NoElements,
}

/// `GET /projects/{projectId}/traceability` (FR-026, FR-027)
pub async fn traceability(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(q): ApiQuery<TraceabilityQuery>,
) -> ApiResult<Json<TraceabilityDto>> {
    let filter = TraceFilter {
        component_id: q.component_id,
        feature_id: q.feature_id,
        gap: q.gap.map(|g| match g {
            GapParam::Untraced => TraceGap::Untraced,
            GapParam::Unimplemented => TraceGap::Unimplemented,
            GapParam::NoElements => TraceGap::NoElements,
        }),
    };
    let trace = state.store.traceability(id, filter).await?;
    Ok(Json((&trace).into()))
}
