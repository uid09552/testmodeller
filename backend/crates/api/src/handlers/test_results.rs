//! Test result import and history (FR-050..FR-053).

use std::collections::HashSet;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use tm_storage::NewResult;
use uuid::Uuid;

use crate::dto::{ImportReportDto, TestResultDto};
use crate::error::{ApiError, ApiResult};
use crate::extract::{ApiPath, ApiQuery};
use crate::results_import::{self, Candidates, Match, ParseError, ResultFormat};
use crate::AppState;

/// Query of `POST /projects/{projectId}/test-results`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportQuery {
    format: ResultFormat,
    #[serde(default)]
    dry_run: bool,
}

/// The file name from `Content-Disposition: ...; filename="..."`, if sent.
fn file_name(headers: &HeaderMap) -> Option<String> {
    let v = headers.get("content-disposition")?.to_str().ok()?;
    let name = v
        .split(';')
        .find_map(|p| p.trim().strip_prefix("filename="))?;
    let name: String = name.trim_matches('"').chars().take(200).collect();
    (!name.is_empty()).then_some(name)
}

/// `POST /projects/{projectId}/test-results?format=&dryRun=`
pub async fn import(
    State(state): State<AppState>,
    ApiPath(project_id): ApiPath<Uuid>,
    ApiQuery(q): ApiQuery<ImportQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<ImportReportDto>> {
    let file = results_import::parse(q.format, &body).map_err(|e| match e {
        ParseError::TooLarge => ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, e.to_string()),
        _ => ApiError::bad_request(format!("cannot import this file: {e}")),
    })?;
    let candidates = Candidates::new(state.store.project_test_case_names(project_id).await?);
    let source = match file_name(&headers) {
        Some(name) => format!("{}: {name}", q.format.as_str()),
        None => q.format.as_str().to_owned(),
    };

    let now = Utc::now();
    let mut report = ImportReportDto {
        dry_run: q.dry_run,
        total: file.results.len(),
        ..Default::default()
    };
    let mut rows: Vec<NewResult> = Vec::new();
    let mut seen = HashSet::new();
    for r in &file.results {
        match candidates.resolve(r) {
            Match::Found(id) => {
                report.matched += 1;
                // The same test twice in one run (e.g. a retried test): the first one counts.
                if !seen.insert(id) {
                    report.duplicates += 1;
                    continue;
                }
                rows.push(NewResult {
                    test_case_id: id,
                    status: r.status,
                    duration_ms: r.duration_ms,
                    message: r.message.clone(),
                    executed_at: r.executed_at.unwrap_or(now),
                });
            }
            Match::Unmatched => report.unmatched.push(r.name.clone()),
            Match::Ambiguous => report.ambiguous.push(r.name.clone()),
        }
    }

    let ids: Vec<Uuid> = rows.iter().map(|r| r.test_case_id).collect();
    let already = state.store.recorded_in_run(&ids, &file.run_id).await?;
    report.duplicates += already.len();
    rows.retain(|r| !already.contains(&r.test_case_id));
    if !q.dry_run {
        report.recorded = state
            .store
            .record_results(project_id, &file.run_id, &source, &rows)
            .await?;
    }
    Ok(Json(report))
}

/// Query of `GET /test-cases/{testCaseId}/results`.
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    limit: Option<i64>,
}

/// `GET /test-cases/{testCaseId}/results?limit=`
pub async fn history(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(q): ApiQuery<HistoryQuery>,
) -> ApiResult<Json<Vec<TestResultDto>>> {
    let limit = q.limit.unwrap_or(50);
    if !(1..=200).contains(&limit) {
        return Err(ApiError::bad_request("limit must be between 1 and 200"));
    }
    let results = state.store.test_results(id, limit).await?;
    Ok(Json(results.iter().map(Into::into).collect()))
}
