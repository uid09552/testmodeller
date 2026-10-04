//! Wire types matching docs/specification/openapi.yaml, with explicit mapping to domain types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tm_domain as d;
use uuid::Uuid;

use crate::error::{ApiError, FieldError};

// ---------- validation helpers ----------

/// Checks string length in characters.
pub fn check_len(field: &str, value: &str, min: usize, max: usize) -> Result<(), ApiError> {
    let n = value.chars().count();
    if n < min || n > max {
        return Err(
            ApiError::bad_request(format!("invalid {field}")).with_errors(vec![FieldError {
                field: field.into(),
                message: format!("length must be between {min} and {max}"),
            }]),
        );
    }
    Ok(())
}

// ---------- common ----------

/// Audit fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuditDto {
    pub id: Uuid,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&d::Audit> for AuditDto {
    fn from(a: &d::Audit) -> Self {
        Self {
            id: a.id,
            version: a.version,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

/// Paginated list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDto<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

impl<T> PageDto<T> {
    /// Maps a storage page.
    pub fn from_page<U>(page: tm_storage::Page<U>) -> Self
    where
        T: for<'a> From<&'a U>,
    {
        Self {
            items: page.items.iter().map(T::from).collect(),
            next_cursor: page.next_cursor,
        }
    }
}

/// Liveness response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    pub status: String,
    pub version: String,
}

// ---------- projects / components / features ----------

/// Project or component input.
#[derive(Debug, Clone, Deserialize)]
pub struct NamedInput {
    pub name: String,
    pub description: Option<String>,
}

impl NamedInput {
    /// Validates and converts.
    pub fn into_fields(self) -> Result<tm_storage::NamedFields, ApiError> {
        check_len("name", &self.name, 1, 200)?;
        Ok(tm_storage::NamedFields {
            name: self.name,
            description: self.description,
        })
    }
}

/// Project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl From<&d::Project> for ProjectDto {
    fn from(p: &d::Project) -> Self {
        Self {
            audit: (&p.audit).into(),
            name: p.name.clone(),
            description: p.description.clone(),
        }
    }
}

/// Component.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub project_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl From<&d::Component> for ComponentDto {
    fn from(c: &d::Component) -> Self {
        Self {
            audit: (&c.audit).into(),
            project_id: c.project_id,
            name: c.name.clone(),
            description: c.description.clone(),
        }
    }
}

/// Feature input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureInput {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl FeatureInput {
    /// Validates and converts.
    pub fn into_fields(self) -> Result<tm_storage::FeatureFields, ApiError> {
        check_len("name", &self.name, 1, 200)?;
        Ok(tm_storage::FeatureFields {
            name: self.name,
            description: self.description,
            scenario_description: self.scenario_description,
            tags: self.tags,
        })
    }
}

/// Feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub component_id: Uuid,
    #[serde(flatten)]
    pub input: FeatureInput,
}

impl From<&d::Feature> for FeatureDto {
    fn from(f: &d::Feature) -> Self {
        Self {
            audit: (&f.audit).into(),
            component_id: f.component_id,
            input: FeatureInput {
                name: f.name.clone(),
                description: f.description.clone(),
                scenario_description: f.scenario_description.clone(),
                tags: Some(f.tags.clone()),
            },
        }
    }
}

/// Body of `POST /features/{id}/move`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveFeature {
    pub component_id: Uuid,
}

/// Body of `POST /test-cases/{id}/move`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveTestCase {
    pub feature_id: Uuid,
}

// ---------- models ----------

/// Model metadata input.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelMetaInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<d::ModelStatus>,
}

impl ModelMetaInput {
    /// Validates and converts.
    pub fn into_meta(self) -> Result<tm_storage::ModelMeta, ApiError> {
        if let Some(n) = &self.name {
            check_len("name", n, 1, 200)?;
        }
        Ok(tm_storage::ModelMeta {
            name: self.name,
            description: self.description,
            status: self.status,
        })
    }
}

/// Canvas position.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PositionDto {
    pub x: f64,
    pub y: f64,
}

/// State input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub kind: d::StateKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<PositionDto>,
}

impl StateInput {
    /// Validates and converts, assigning a new id if absent.
    pub fn into_state(self) -> Result<d::State, ApiError> {
        check_len("name", &self.name, 1, 500)?;
        Ok(d::State {
            id: self.id.unwrap_or_else(Uuid::new_v4),
            name: self.name,
            description: self.description,
            kind: self.kind,
            position: self.position.map(|p| d::Position { x: p.x, y: p.y }),
        })
    }
}

impl From<&d::State> for StateInput {
    fn from(s: &d::State) -> Self {
        Self {
            id: Some(s.id),
            name: s.name.clone(),
            description: s.description.clone(),
            kind: s.kind,
            position: s.position.map(|p| PositionDto { x: p.x, y: p.y }),
        }
    }
}

/// State.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateDto {
    #[serde(flatten)]
    pub input: StateInput,
    pub model_id: Uuid,
    pub test_case_count: i64,
}

impl From<&tm_storage::StateRecord> for StateDto {
    fn from(r: &tm_storage::StateRecord) -> Self {
        Self {
            input: (&r.state).into(),
            model_id: r.model_id,
            test_case_count: r.test_case_count,
        }
    }
}

/// Transition input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
    pub from: Uuid,
    pub to: Uuid,
    pub event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
}

impl TransitionInput {
    /// Validates and converts, assigning a new id if absent.
    pub fn into_transition(self) -> Result<d::Transition, ApiError> {
        check_len("event", &self.event, 1, 500)?;
        Ok(d::Transition {
            id: self.id.unwrap_or_else(Uuid::new_v4),
            from: self.from,
            to: self.to,
            event: self.event,
            guard: self.guard,
            action: self.action,
            expected: self.expected,
        })
    }
}

impl From<&d::Transition> for TransitionInput {
    fn from(t: &d::Transition) -> Self {
        Self {
            id: Some(t.id),
            from: t.from,
            to: t.to,
            event: t.event.clone(),
            guard: t.guard.clone(),
            action: t.action.clone(),
            expected: t.expected.clone(),
        }
    }
}

/// Transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionDto {
    #[serde(flatten)]
    pub input: TransitionInput,
    pub model_id: Uuid,
    pub test_case_count: i64,
}

impl From<&tm_storage::TransitionRecord> for TransitionDto {
    fn from(r: &tm_storage::TransitionRecord) -> Self {
        Self {
            input: (&r.transition).into(),
            model_id: r.model_id,
            test_case_count: r.test_case_count,
        }
    }
}

/// Model variable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableDto {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: d::VariableType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial: Option<serde_json::Value>,
}

impl From<&d::Variable> for VariableDto {
    fn from(v: &d::Variable) -> Self {
        Self {
            name: v.name.clone(),
            var_type: v.var_type,
            initial: v.initial.clone(),
        }
    }
}

impl From<VariableDto> for d::Variable {
    fn from(v: VariableDto) -> Self {
        Self {
            name: v.name,
            var_type: v.var_type,
            initial: v.initial,
        }
    }
}

/// Full model input (create / atomic save).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelInput {
    #[serde(flatten)]
    pub meta: ModelMetaInput,
    #[serde(default)]
    pub variables: Vec<VariableDto>,
    #[serde(default)]
    pub states: Vec<StateInput>,
    #[serde(default)]
    pub transitions: Vec<TransitionInput>,
}

impl ModelInput {
    /// Validates and converts to `(name, meta, graph)`. Rejects structurally broken graphs (422).
    pub fn into_parts(self) -> Result<(String, tm_storage::ModelMeta, d::ModelGraph), ApiError> {
        let graph = Self::graph(self.variables, self.states, self.transitions)?;
        let structural = d::validation::structural_issues(&graph);
        if !structural.is_empty() {
            return Err(ApiError::unprocessable("model graph is inconsistent")
                .with_errors(structural.iter().map(issue_error).collect()));
        }
        let meta = self.meta.into_meta()?;
        let name = meta
            .name
            .clone()
            .ok_or_else(|| ApiError::bad_request("name is required"))?;
        Ok((name, meta, graph))
    }

    /// Converts the graph part without structural checks (used by validate).
    pub fn graph(
        variables: Vec<VariableDto>,
        states: Vec<StateInput>,
        transitions: Vec<TransitionInput>,
    ) -> Result<d::ModelGraph, ApiError> {
        Ok(d::ModelGraph {
            variables: variables.into_iter().map(Into::into).collect(),
            states: states
                .into_iter()
                .map(StateInput::into_state)
                .collect::<Result<_, _>>()?,
            transitions: transitions
                .into_iter()
                .map(TransitionInput::into_transition)
                .collect::<Result<_, _>>()?,
        })
    }

    /// Wire form of a model graph with metadata.
    pub fn from_graph(name: &str, description: Option<&str>, graph: &d::ModelGraph) -> Self {
        Self {
            meta: ModelMetaInput {
                name: Some(name.to_owned()),
                description: description.map(str::to_owned),
                status: None,
            },
            variables: graph.variables.iter().map(Into::into).collect(),
            states: graph.states.iter().map(Into::into).collect(),
            transitions: graph.transitions.iter().map(Into::into).collect(),
        }
    }
}

fn issue_error(i: &d::ValidationIssue) -> FieldError {
    let field = i
        .transition_id
        .map(|t| format!("transitions[{t}]"))
        .or_else(|| i.state_id.map(|s| format!("states[{s}]")))
        .unwrap_or_else(|| i.code.to_owned());
    FieldError {
        field,
        message: format!("{}: {}", i.code, i.message),
    }
}

/// Model summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSummaryDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub feature_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: d::ModelStatus,
    pub state_count: i64,
    pub transition_count: i64,
    pub test_case_count: i64,
}

impl From<&d::ModelSummary> for ModelSummaryDto {
    fn from(m: &d::ModelSummary) -> Self {
        Self {
            audit: (&m.audit).into(),
            feature_id: m.feature_id,
            name: m.name.clone(),
            description: m.description.clone(),
            status: m.status,
            state_count: m.state_count,
            transition_count: m.transition_count,
            test_case_count: m.test_case_count,
        }
    }
}

/// Full model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDto {
    #[serde(flatten)]
    pub summary: ModelSummaryDto,
    pub variables: Vec<VariableDto>,
    pub states: Vec<StateDto>,
    pub transitions: Vec<TransitionDto>,
}

impl From<&d::Model> for ModelDto {
    fn from(m: &d::Model) -> Self {
        let id = m.summary.audit.id;
        Self {
            summary: (&m.summary).into(),
            variables: m.graph.variables.iter().map(Into::into).collect(),
            states: m
                .graph
                .states
                .iter()
                .map(|s| StateDto {
                    input: s.into(),
                    model_id: id,
                    test_case_count: m.state_test_case_counts.get(&s.id).copied().unwrap_or(0),
                })
                .collect(),
            transitions: m
                .graph
                .transitions
                .iter()
                .map(|t| TransitionDto {
                    input: t.into(),
                    model_id: id,
                    test_case_count: m
                        .transition_test_case_counts
                        .get(&t.id)
                        .copied()
                        .unwrap_or(0),
                })
                .collect(),
        }
    }
}

/// Body of `POST /models/{id}/duplicate`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateModel {
    pub name: Option<String>,
    pub feature_id: Option<Uuid>,
}

/// Version history entry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelVersionDto {
    pub version: i32,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl From<&d::ModelVersion> for ModelVersionDto {
    fn from(v: &d::ModelVersion) -> Self {
        Self {
            version: v.version,
            created_at: v.created_at,
            summary: v.summary.clone(),
        }
    }
}

/// Validation issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssueDto {
    pub severity: String,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_id: Option<Uuid>,
}

impl From<&d::ValidationIssue> for ValidationIssueDto {
    fn from(i: &d::ValidationIssue) -> Self {
        Self {
            severity: i.severity.as_str().into(),
            code: i.code.into(),
            message: i.message.clone(),
            state_id: i.state_id,
            transition_id: i.transition_id,
        }
    }
}

// ---------- test cases & assignments ----------

/// Test step input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStepInput {
    pub action: String,
    pub expected: String,
}

/// Test step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStepDto {
    pub order: i32,
    pub action: String,
    pub expected: String,
}

/// Assignment input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentInput {
    pub model_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_order: Option<i32>,
}

impl AssignmentInput {
    /// Validates exactly-one-target and converts (422 otherwise).
    pub fn into_new(self) -> Result<tm_storage::NewAssignment, ApiError> {
        let target = match (self.state_id, self.transition_id) {
            (Some(s), None) => d::AssignmentTarget::State(s),
            (None, Some(t)) => d::AssignmentTarget::Transition(t),
            _ => {
                return Err(ApiError::unprocessable(
                    "an assignment needs exactly one of stateId or transitionId",
                ))
            }
        };
        Ok(tm_storage::NewAssignment {
            model_id: self.model_id,
            target,
            step_order: self.step_order,
        })
    }

    /// Converts a list.
    pub fn into_list(list: Vec<Self>) -> Result<Vec<tm_storage::NewAssignment>, ApiError> {
        list.into_iter().map(Self::into_new).collect()
    }
}

/// Assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentDto {
    #[serde(flatten)]
    pub input: AssignmentInput,
    pub test_case_id: Uuid,
}

impl From<&d::Assignment> for AssignmentDto {
    fn from(a: &d::Assignment) -> Self {
        let (state_id, transition_id) = match a.target {
            d::AssignmentTarget::State(s) => (Some(s), None),
            d::AssignmentTarget::Transition(t) => (None, Some(t)),
        };
        Self {
            input: AssignmentInput {
                model_id: a.model_id,
                state_id,
                transition_id,
                step_order: a.step_order,
            },
            test_case_id: a.test_case_id,
        }
    }
}

/// Test case input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseInput {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preconditions: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<d::Priority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<d::TestCaseStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub steps: Vec<TestStepInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignments: Option<Vec<AssignmentInput>>,
}

impl TestCaseInput {
    /// Validates and converts to data plus optional assignments.
    pub fn into_parts(
        self,
    ) -> Result<(d::TestCaseData, Option<Vec<tm_storage::NewAssignment>>), ApiError> {
        check_len("name", &self.name, 1, 300)?;
        let assignments = self
            .assignments
            .map(AssignmentInput::into_list)
            .transpose()?;
        Ok((
            d::TestCaseData {
                name: self.name,
                description: self.description,
                preconditions: self.preconditions,
                priority: self.priority,
                status: self.status.unwrap_or(d::TestCaseStatus::Draft),
                tags: self.tags.unwrap_or_default(),
                steps: self
                    .steps
                    .into_iter()
                    .enumerate()
                    .map(|(i, s)| d::TestStep {
                        order: i as i32 + 1,
                        action: s.action,
                        expected: s.expected,
                    })
                    .collect(),
            },
            assignments,
        ))
    }

    /// Wire form of test case data.
    pub fn from_data(data: &d::TestCaseData) -> Self {
        Self {
            name: data.name.clone(),
            description: data.description.clone(),
            preconditions: data.preconditions.clone(),
            priority: data.priority,
            status: Some(data.status),
            tags: Some(data.tags.clone()),
            steps: data
                .steps
                .iter()
                .map(|s| TestStepInput {
                    action: s.action.clone(),
                    expected: s.expected.clone(),
                })
                .collect(),
            assignments: None,
        }
    }
}

/// Test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCaseDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub feature_id: Uuid,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preconditions: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<d::Priority>,
    pub status: d::TestCaseStatus,
    #[serde(default)]
    pub tags: Vec<String>,
    pub origin: d::Origin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_from_model_id: Option<Uuid>,
    pub steps: Vec<TestStepDto>,
    pub assignments: Vec<AssignmentDto>,
}

impl From<&d::TestCase> for TestCaseDto {
    fn from(t: &d::TestCase) -> Self {
        Self {
            audit: (&t.audit).into(),
            feature_id: t.feature_id,
            name: t.data.name.clone(),
            description: t.data.description.clone(),
            preconditions: t.data.preconditions.clone(),
            priority: t.data.priority,
            status: t.data.status,
            tags: t.data.tags.clone(),
            origin: t.origin,
            generated_from_model_id: t.generated_from_model_id,
            steps: t
                .data
                .steps
                .iter()
                .map(|s| TestStepDto {
                    order: s.order,
                    action: s.action.clone(),
                    expected: s.expected.clone(),
                })
                .collect(),
            assignments: t.assignments.iter().map(Into::into).collect(),
        }
    }
}

impl TestCaseDto {
    /// Converts back to domain data (used by import).
    pub fn to_data(&self) -> d::TestCaseData {
        d::TestCaseData {
            name: self.name.clone(),
            description: self.description.clone(),
            preconditions: self.preconditions.clone(),
            priority: self.priority,
            status: self.status,
            tags: self.tags.clone(),
            steps: self
                .steps
                .iter()
                .enumerate()
                .map(|(i, s)| d::TestStep {
                    order: i as i32 + 1,
                    action: s.action.clone(),
                    expected: s.expected.clone(),
                })
                .collect(),
        }
    }
}

// ---------- generation & coverage ----------

/// Body of `POST /models/{id}/generate`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationRequest {
    pub criterion: d::CoverageCriterion,
    pub seed: Option<i64>,
    pub max_path_length: Option<i64>,
    #[serde(default)]
    pub save: bool,
}

/// Covered/total pair.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageCount {
    pub covered: usize,
    pub total: usize,
}

/// Coverage report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CoverageDto {
    pub states: CoverageCount,
    pub transitions: CoverageCount,
    pub uncovered_state_ids: Vec<Uuid>,
    pub uncovered_transition_ids: Vec<Uuid>,
}

impl From<&tm_generation::CoverageReport> for CoverageDto {
    fn from(c: &tm_generation::CoverageReport) -> Self {
        Self {
            states: CoverageCount {
                covered: c.states_covered,
                total: c.states_total,
            },
            transitions: CoverageCount {
                covered: c.transitions_covered,
                total: c.transitions_total,
            },
            uncovered_state_ids: c.uncovered_state_ids.clone(),
            uncovered_transition_ids: c.uncovered_transition_ids.clone(),
        }
    }
}

/// Skipped path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedPathDto {
    pub transition_ids: Vec<Uuid>,
    pub reason: String,
}

/// Generation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationResultDto {
    pub seed: i64,
    pub test_cases: Vec<TestCaseDto>,
    pub coverage: CoverageDto,
    pub skipped_paths: Vec<SkippedPathDto>,
}

// ---------- explorer ----------

/// Tree model node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeModelDto {
    pub id: Uuid,
    pub name: String,
    pub status: d::ModelStatus,
}

/// Tree feature node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeFeatureDto {
    pub id: Uuid,
    pub name: String,
    pub models: Vec<TreeModelDto>,
}

/// Tree component node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeComponentDto {
    pub id: Uuid,
    pub name: String,
    pub features: Vec<TreeFeatureDto>,
}

/// Explorer tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTreeDto {
    pub components: Vec<TreeComponentDto>,
}

impl From<&[tm_storage::TreeComponent]> for ProjectTreeDto {
    fn from(tree: &[tm_storage::TreeComponent]) -> Self {
        Self {
            components: tree
                .iter()
                .map(|c| TreeComponentDto {
                    id: c.id,
                    name: c.name.clone(),
                    features: c
                        .features
                        .iter()
                        .map(|f| TreeFeatureDto {
                            id: f.id,
                            name: f.name.clone(),
                            models: f
                                .models
                                .iter()
                                .map(|m| TreeModelDto {
                                    id: m.id,
                                    name: m.name.clone(),
                                    status: m.status,
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

/// Search hit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHitDto {
    #[serde(rename = "type")]
    pub hit_type: String,
    pub id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

impl From<&tm_storage::SearchHit> for SearchHitDto {
    fn from(h: &tm_storage::SearchHit) -> Self {
        Self {
            hit_type: match h.hit_type {
                tm_storage::SearchType::Component => "component",
                tm_storage::SearchType::Feature => "feature",
                tm_storage::SearchType::Model => "model",
                tm_storage::SearchType::TestCase => "testCase",
            }
            .into(),
            id: h.id,
            name: h.name.clone(),
            snippet: h.snippet.clone(),
        }
    }
}

// ---------- AI ----------

/// Body of `POST /ai/proposals`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalRequestDto {
    pub kind: d::ProposalKind,
    pub feature_id: Uuid,
    pub model_id: Option<Uuid>,
    pub prompt: Option<String>,
    pub count: Option<u32>,
    #[serde(default)]
    pub preview_context: bool,
}

/// Proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalDto {
    pub id: Uuid,
    pub project_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<Uuid>,
    pub kind: d::ProposalKind,
    pub status: d::ProposalStatus,
    pub payload: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resulting_entity_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl From<&d::Proposal> for ProposalDto {
    fn from(p: &d::Proposal) -> Self {
        Self {
            id: p.id,
            project_id: p.project_id,
            feature_id: p.feature_id,
            model_id: p.model_id,
            kind: p.kind,
            status: p.status,
            payload: p.payload.clone(),
            rationale: p.rationale.clone(),
            source: p.source.clone(),
            resulting_entity_id: p.resulting_entity_id,
            created_at: p.created_at,
        }
    }
}

/// Body of `POST /proposals/{id}/accept`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AcceptProposal {
    pub payload: Option<serde_json::Value>,
}

/// Body of `POST /proposals/{id}/reject`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RejectProposal {
    pub reason: Option<String>,
}

/// Payload of a `states-and-transitions` proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementsPayload {
    #[serde(default)]
    pub states: Vec<StateInput>,
    #[serde(default)]
    pub transitions: Vec<TransitionInput>,
}

/// Payload of a `feature-description` proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureDescriptionPayload {
    pub scenario_description: String,
}

/// Job.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDto {
    pub id: Uuid,
    pub status: d::JobStatus,
    pub proposal_ids: Vec<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::error::Problem>,
}

impl From<&d::Job> for JobDto {
    fn from(j: &d::Job) -> Self {
        Self {
            id: j.id,
            status: j.status,
            proposal_ids: j.proposal_ids.clone(),
            error: j.error.as_ref().map(|msg| {
                ApiError::new(axum::http::StatusCode::BAD_GATEWAY, msg.clone()).problem()
            }),
        }
    }
}

/// AI settings (secrets are never returned).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsDto {
    pub provider: d::AiProvider,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub secret_configured: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_request: Option<i32>,
}

/// AI settings input.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsInput {
    pub provider: Option<d::AiProvider>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub max_tokens_per_request: Option<i32>,
    pub api_key: Option<String>,
}
