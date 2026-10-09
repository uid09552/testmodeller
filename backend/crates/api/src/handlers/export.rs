//! Export (JSON, CSV, Gherkin) and JSON import (FR-007, FR-024).

use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tm_storage::{
    ExportBundle, ExportScope, ImportComponent, ImportFeature, ImportModel, ImportTestCase,
};
use uuid::Uuid;

use crate::dto::*;
use crate::error::{ApiError, ApiResult};
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::AppState;

const FORMAT: &str = "testmodeller-export";
const FORMAT_VERSION: u32 = 1;

/// Export format.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// Nested JSON, re-importable.
    Json,
    /// One row per test step.
    Csv,
    /// Gherkin feature files.
    Gherkin,
}

/// Query of `GET /projects/{projectId}/export`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportQuery {
    format: ExportFormat,
    feature_id: Option<Uuid>,
    component_id: Option<Uuid>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportDoc {
    format: &'static str,
    format_version: u32,
    exported_at: DateTime<Utc>,
    project: ProjectDto,
    components: Vec<ExportComponentDoc>,
}

#[derive(Serialize)]
struct ExportComponentDoc {
    #[serde(flatten)]
    component: ComponentDto,
    features: Vec<ExportFeatureDoc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportFeatureDoc {
    #[serde(flatten)]
    feature: FeatureDto,
    models: Vec<ModelDto>,
    test_cases: Vec<TestCaseDto>,
}

/// `GET /projects/{projectId}/export`
pub async fn export(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(q): ApiQuery<ExportQuery>,
) -> ApiResult<Response> {
    let scope = match (q.feature_id, q.component_id) {
        (Some(f), _) => ExportScope::Feature(f),
        (None, Some(c)) => ExportScope::Component(c),
        (None, None) => ExportScope::Project,
    };
    let bundle = state.store.export_bundle(id, scope).await?;
    let stem = format!("testmodeller-{id}");
    let (content_type, ext, body) = match q.format {
        ExportFormat::Json => {
            let doc = to_doc(&bundle);
            let body = serde_json::to_vec_pretty(&doc)
                .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            ("application/json", "json", body)
        }
        ExportFormat::Csv => (
            "text/csv; charset=utf-8",
            "csv",
            render_csv(&bundle).into_bytes(),
        ),
        ExportFormat::Gherkin => (
            "text/plain; charset=utf-8",
            "feature",
            render_gherkin(&bundle).into_bytes(),
        ),
    };
    Ok((
        [
            (header::CONTENT_TYPE, content_type.to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{stem}.{ext}\""),
            ),
        ],
        body,
    )
        .into_response())
}

fn to_doc(b: &ExportBundle) -> ExportDoc {
    ExportDoc {
        format: FORMAT,
        format_version: FORMAT_VERSION,
        exported_at: Utc::now(),
        project: (&b.project).into(),
        components: b
            .components
            .iter()
            .map(|c| ExportComponentDoc {
                component: (&c.component).into(),
                features: c
                    .features
                    .iter()
                    .map(|f| ExportFeatureDoc {
                        feature: (&f.feature).into(),
                        models: f.models.iter().map(Into::into).collect(),
                        test_cases: f.test_cases.iter().map(Into::into).collect(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

fn render_csv(b: &ExportBundle) -> String {
    let mut out = String::from(
        "component,feature,testCaseId,testCase,status,priority,origin,tags,preconditions,step,action,expected,tmId\r\n",
    );
    for c in &b.components {
        for f in &c.features {
            for tc in &f.test_cases {
                let d = &tc.data;
                let prefix = [
                    c.component.name.clone(),
                    f.feature.name.clone(),
                    tc.audit.id.to_string(),
                    d.name.clone(),
                    d.status.to_string(),
                    d.priority.map(|p| p.to_string()).unwrap_or_default(),
                    tc.origin.to_string(),
                    d.tags.join(";"),
                    d.preconditions.clone().unwrap_or_default(),
                ];
                let rows: Vec<[String; 3]> = if d.steps.is_empty() {
                    vec![[String::new(), String::new(), String::new()]]
                } else {
                    d.steps
                        .iter()
                        .map(|s| [s.order.to_string(), s.action.clone(), s.expected.clone()])
                        .collect()
                };
                // The tag the result importer matches on (FR-051), last so that
                // existing column positions stay put.
                let tag = [tm_tag(tc.audit.id)];
                for row in rows {
                    let fields: Vec<String> = prefix
                        .iter()
                        .chain(row.iter())
                        .chain(tag.iter())
                        .map(|s| csv_field(s))
                        .collect();
                    out.push_str(&fields.join(","));
                    out.push_str("\r\n");
                }
            }
        }
    }
    out
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn gherkin_tag(t: &str) -> String {
    format!("@{}", t.split_whitespace().collect::<Vec<_>>().join("_"))
}

/// Stable tag of a test case: what the result importer matches on (FR-051).
pub fn tm_tag(id: Uuid) -> String {
    format!("@tm-{id}")
}

fn render_gherkin(b: &ExportBundle) -> String {
    let mut out = String::new();
    for c in &b.components {
        for f in &c.features {
            out.push_str(&format!("# Component: {}\n", one_line(&c.component.name)));
            if !f.feature.tags.is_empty() {
                let tags: Vec<String> = f.feature.tags.iter().map(|t| gherkin_tag(t)).collect();
                out.push_str(&format!("{}\n", tags.join(" ")));
            }
            out.push_str(&format!("Feature: {}\n", one_line(&f.feature.name)));
            for line in f
                .feature
                .scenario_description
                .as_deref()
                .or(f.feature.description.as_deref())
                .unwrap_or("")
                .lines()
            {
                out.push_str(&format!("  {}\n", line.trim_end()));
            }
            for tc in &f.test_cases {
                let d = &tc.data;
                out.push('\n');
                let tags: Vec<String> = std::iter::once(tm_tag(tc.audit.id))
                    .chain(d.tags.iter().map(|t| gherkin_tag(t)))
                    .collect();
                out.push_str(&format!("  {}\n", tags.join(" ")));
                out.push_str(&format!("  Scenario: {}\n", one_line(&d.name)));
                if let Some(p) = d.preconditions.as_deref().filter(|p| !p.trim().is_empty()) {
                    out.push_str(&format!("    Given {}\n", one_line(p)));
                }
                for s in &d.steps {
                    out.push_str(&format!("    When {}\n", one_line(&s.action)));
                    out.push_str(&format!("    Then {}\n", one_line(&s.expected)));
                }
            }
            out.push('\n');
        }
    }
    out
}

#[derive(Deserialize)]
struct ImportDoc {
    format: Option<String>,
    #[serde(default)]
    components: Vec<ImportComponentDoc>,
}

#[derive(Deserialize)]
struct ImportComponentDoc {
    name: String,
    description: Option<String>,
    #[serde(default)]
    features: Vec<ImportFeatureDoc>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportFeatureDoc {
    #[serde(flatten)]
    input: FeatureInput,
    #[serde(default)]
    models: Vec<ImportModelDoc>,
    #[serde(default)]
    test_cases: Vec<ImportTestCaseDoc>,
}

#[derive(Deserialize)]
struct ImportModelDoc {
    id: Uuid,
    name: String,
    description: Option<String>,
    status: Option<tm_domain::ModelStatus>,
    #[serde(default)]
    variables: Vec<VariableDto>,
    #[serde(default)]
    states: Vec<StateInput>,
    #[serde(default)]
    transitions: Vec<TransitionInput>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportTestCaseDoc {
    #[serde(flatten)]
    input: TestCaseInput,
    origin: Option<tm_domain::Origin>,
    generated_from_model_id: Option<Uuid>,
}

/// Response of `POST /projects/{projectId}/import`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    components: i64,
    features: i64,
    models: i64,
    test_cases: i64,
}

fn convert(doc: ImportDoc) -> ApiResult<Vec<ImportComponent>> {
    doc.components
        .into_iter()
        .map(|c| {
            check_len("component name", &c.name, 1, 200)?;
            Ok(ImportComponent {
                name: c.name,
                description: c.description,
                features: c
                    .features
                    .into_iter()
                    .map(|f| {
                        Ok(ImportFeature {
                            fields: f.input.into_fields()?,
                            models: f
                                .models
                                .into_iter()
                                .map(|m| {
                                    check_len("model name", &m.name, 1, 200)?;
                                    Ok(ImportModel {
                                        source_id: m.id,
                                        name: m.name,
                                        description: m.description,
                                        status: m.status.unwrap_or(tm_domain::ModelStatus::Draft),
                                        graph: ModelInput::graph(
                                            m.variables,
                                            m.states,
                                            m.transitions,
                                        )?,
                                    })
                                })
                                .collect::<ApiResult<_>>()?,
                            test_cases: f
                                .test_cases
                                .into_iter()
                                .map(|t| {
                                    let (data, assignments) = t.input.into_parts()?;
                                    Ok(ImportTestCase {
                                        data,
                                        origin: t.origin.unwrap_or(tm_domain::Origin::Manual),
                                        generated_from_model_id: t.generated_from_model_id,
                                        assignments: assignments.unwrap_or_default(),
                                    })
                                })
                                .collect::<ApiResult<_>>()?,
                        })
                    })
                    .collect::<ApiResult<_>>()?,
            })
        })
        .collect()
}

/// `POST /projects/{projectId}/import` — imports a JSON export with fresh ids.
pub async fn import(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(body): ApiJson<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<ImportResult>)> {
    let doc: ImportDoc = serde_json::from_value(body)
        .map_err(|e| ApiError::unprocessable(format!("not a TestModeller export: {e}")))?;
    if doc.format.as_deref().is_some_and(|f| f != FORMAT) {
        return Err(ApiError::unprocessable("unsupported export format"));
    }
    let components =
        convert(doc).map_err(|e| ApiError::unprocessable(e.detail).with_errors(e.errors))?;
    let counts = state.store.import_bundle(id, &components).await?;
    Ok((
        StatusCode::CREATED,
        Json(ImportResult {
            components: counts.components,
            features: counts.features,
            models: counts.models,
            test_cases: counts.test_cases,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_quotes_special_characters() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn gherkin_tags_have_no_spaces() {
        assert_eq!(gherkin_tag("smoke test"), "@smoke_test");
    }
}
