//! Wire types of the threat model resources (tag `ThreatModel` in openapi.yaml)
//! and their mapping to the domain. Computed fields are derived from the
//! aggregate on every read (docs/specification/09-threat-modeling.md).

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use tm_domain::threat::{
    self as t, arch, risk, staleness, ArchComponentType, AssetCategory, CiaLevel, ControlCategory,
    ControlStatus, ControlType, Criticality, FlowRelationship, RiskLevel, RiskTreatment,
    RuleExplanation, SecurityTestOrigin, SecurityTestOutcome, SecurityTestStatus, SecurityTestStep,
    SecurityTestType, StrideCategory, TargetKind, ThreatFlag, ThreatModelError, ThreatOrigin,
    ThreatStatus, TriState, ZoneKind,
};
use tm_storage::ThreatModelAggregate;
use uuid::Uuid;

use crate::dto::{check_len, AuditDto};
use crate::error::{ApiError, FieldError};

/// Domain validation error as a 422 naming its field.
pub fn invalid(e: ThreatModelError) -> ApiError {
    ApiError::unprocessable(e.to_string()).with_errors(vec![FieldError {
        field: e.field.into(),
        message: e.message,
    }])
}

fn blank_to_none(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.trim().is_empty())
}

// ---------- threat model ----------

/// Score range of a level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThresholdDto {
    pub level: RiskLevel,
    pub min: u8,
    pub max: u8,
}

/// `ThreatModelInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreatModelInput {
    pub name: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub objectives: Option<String>,
    #[serde(default)]
    pub assumptions: Option<String>,
    #[serde(default)]
    pub exclusions: Option<String>,
    #[serde(default)]
    pub thresholds: Option<Vec<ThresholdDto>>,
}

impl ThreatModelInput {
    /// Validates; absent thresholds mean `fallback`.
    pub fn into_parts(
        self,
        fallback: t::Thresholds,
    ) -> Result<(t::ThreatModelFields, t::Thresholds), ApiError> {
        check_len("name", &self.name, 1, 200)?;
        let thresholds = match self.thresholds {
            None => fallback,
            Some(items) => t::Thresholds::new(
                items
                    .into_iter()
                    .map(|x| t::Threshold {
                        level: x.level,
                        min: x.min,
                        max: x.max,
                    })
                    .collect(),
            )
            .map_err(invalid)?,
        };
        Ok((
            t::ThreatModelFields {
                name: self.name,
                scope: self.scope,
                objectives: self.objectives,
                assumptions: self.assumptions,
                exclusions: self.exclusions,
            },
            thresholds,
        ))
    }
}

/// `ThreatModel`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreatModelDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub project_id: Uuid,
    pub name: String,
    pub scope: Option<String>,
    pub objectives: Option<String>,
    pub assumptions: Option<String>,
    pub exclusions: Option<String>,
    pub thresholds: Vec<ThresholdDto>,
    pub layout: Option<serde_json::Value>,
    pub layout_version: i32,
}

impl From<&t::ThreatModel> for ThreatModelDto {
    fn from(m: &t::ThreatModel) -> Self {
        Self {
            audit: (&m.audit).into(),
            project_id: m.project_id,
            name: m.fields.name.clone(),
            scope: m.fields.scope.clone(),
            objectives: m.fields.objectives.clone(),
            assumptions: m.fields.assumptions.clone(),
            exclusions: m.fields.exclusions.clone(),
            thresholds: m
                .thresholds
                .items()
                .iter()
                .map(|x| ThresholdDto {
                    level: x.level,
                    min: x.min,
                    max: x.max,
                })
                .collect(),
            layout: m.layout.clone(),
            layout_version: m.layout_version,
        }
    }
}

/// Answer of `PUT /threat-models/{id}/layout`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutSaved {
    pub layout_version: i32,
}

// ---------- shared ----------

/// `Warning`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WarningDto {
    pub code: String,
    pub message: String,
}

fn warnings(w: Vec<arch::Warning>) -> Vec<WarningDto> {
    w.into_iter()
        .map(|w| WarningDto {
            code: w.code.into(),
            message: w.message.into(),
        })
        .collect()
}

// ---------- zones ----------

/// `TmZoneInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub kind: ZoneKind,
    pub trust_level: u8,
}

impl ZoneInput {
    /// Validates.
    pub fn into_fields(self) -> Result<t::ZoneFields, ApiError> {
        check_len("name", &self.name, 1, 200)?;
        if self.trust_level > 4 {
            return Err(invalid(ThreatModelError::new(
                "trustLevel",
                "must be between 0 and 4",
            )));
        }
        Ok(t::ZoneFields {
            name: self.name,
            description: self.description,
            kind: self.kind,
            trust_level: self.trust_level,
        })
    }
}

/// `TmZone`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub name: String,
    pub description: Option<String>,
    pub kind: ZoneKind,
    pub trust_level: u8,
}

// ---------- components ----------

/// `ArchComponentInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchComponentInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub component_type: ArchComponentType,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub technology: Option<String>,
    #[serde(default)]
    pub criticality: Option<Criticality>,
    #[serde(default)]
    pub internet_facing: bool,
    #[serde(default)]
    pub zone_id: Option<Uuid>,
    #[serde(default)]
    pub linked_component_id: Option<Uuid>,
    #[serde(default)]
    pub linked_feature_id: Option<Uuid>,
    #[serde(default)]
    pub asset_ids: Vec<Uuid>,
}

impl ArchComponentInput {
    /// Validates.
    pub fn into_fields(self) -> Result<t::ArchComponentFields, ApiError> {
        check_len("name", &self.name, 1, 200)?;
        Ok(t::ArchComponentFields {
            name: self.name,
            description: self.description,
            component_type: self.component_type,
            owner: blank_to_none(self.owner),
            technology: blank_to_none(self.technology),
            criticality: self.criticality,
            internet_facing: self.internet_facing,
            zone_id: self.zone_id,
            linked_component_id: self.linked_component_id,
            linked_feature_id: self.linked_feature_id,
            asset_ids: self.asset_ids,
        })
    }
}

/// `ArchComponent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchComponentDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub component_type: ArchComponentType,
    pub owner: Option<String>,
    pub technology: Option<String>,
    pub criticality: Option<Criticality>,
    pub internet_facing: bool,
    pub zone_id: Option<Uuid>,
    pub linked_component_id: Option<Uuid>,
    pub linked_feature_id: Option<Uuid>,
    pub asset_ids: Vec<Uuid>,
    pub layer: t::ArchLayer,
    pub warnings: Vec<WarningDto>,
}

// ---------- assets ----------

/// `TmAssetInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub category: AssetCategory,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub confidentiality: Option<CiaLevel>,
    #[serde(default)]
    pub integrity: Option<CiaLevel>,
    #[serde(default)]
    pub availability: Option<CiaLevel>,
    #[serde(default)]
    pub personal_data: bool,
}

impl AssetInput {
    /// Validates.
    pub fn into_fields(self) -> Result<t::AssetFields, ApiError> {
        check_len("name", &self.name, 1, 200)?;
        Ok(t::AssetFields {
            name: self.name,
            description: self.description,
            category: self.category,
            owner: blank_to_none(self.owner),
            confidentiality: self.confidentiality,
            integrity: self.integrity,
            availability: self.availability,
            personal_data: self.personal_data,
        })
    }
}

/// `TmAsset`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub name: String,
    pub description: Option<String>,
    pub category: AssetCategory,
    pub owner: Option<String>,
    pub confidentiality: Option<CiaLevel>,
    pub integrity: Option<CiaLevel>,
    pub availability: Option<CiaLevel>,
    pub personal_data: bool,
    pub component_ids: Vec<Uuid>,
    pub flow_ids: Vec<Uuid>,
    pub warnings: Vec<WarningDto>,
}

// ---------- flows ----------

fn default_unknown_auth() -> String {
    "unknown".into()
}

fn default_unknown() -> TriState {
    TriState::Unknown
}

/// `TmFlowInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowInput {
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub relationship: FlowRelationship,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub bidirectional: bool,
    #[serde(default = "default_unknown_auth")]
    pub authentication: String,
    #[serde(default = "default_unknown")]
    pub encrypted: TriState,
    #[serde(default)]
    pub asset_ids: Vec<Uuid>,
}

impl FlowInput {
    /// Validates.
    pub fn into_fields(self) -> Result<t::FlowFields, ApiError> {
        check_len("authentication", &self.authentication, 0, 200)?;
        let authentication = if self.authentication.trim().is_empty() {
            "unknown".to_owned()
        } else {
            self.authentication.trim().to_owned()
        };
        Ok(t::FlowFields {
            source_id: self.source_id,
            target_id: self.target_id,
            relationship: self.relationship,
            label: blank_to_none(self.label),
            protocol: blank_to_none(self.protocol),
            bidirectional: self.bidirectional,
            authentication,
            encrypted: self.encrypted,
            asset_ids: self.asset_ids,
        })
    }
}

/// `Crossing`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CrossingDto {
    pub crosses: bool,
    pub direction: Option<String>,
    pub from_trust: Option<u8>,
    pub to_trust: Option<u8>,
}

/// `TmFlow`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub relationship: FlowRelationship,
    pub label: Option<String>,
    pub protocol: Option<String>,
    pub bidirectional: bool,
    pub authentication: String,
    pub encrypted: TriState,
    pub asset_ids: Vec<Uuid>,
    pub crossing: CrossingDto,
    pub warnings: Vec<WarningDto>,
}

// ---------- threats ----------

/// `Rating`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RatingDto {
    pub likelihood: u8,
    pub impact: u8,
}

/// `ThreatTarget`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TargetDto {
    pub kind: TargetKind,
    pub id: Uuid,
}

fn default_open() -> ThreatStatus {
    ThreatStatus::Open
}

fn default_undecided() -> RiskTreatment {
    RiskTreatment::Undecided
}

/// `ThreatInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreatInput {
    pub title: String,
    pub category: StrideCategory,
    #[serde(default)]
    pub scenario: Option<String>,
    #[serde(default)]
    pub attacker_objective: Option<String>,
    #[serde(default)]
    pub preconditions: Option<String>,
    #[serde(default)]
    pub consequences: Option<String>,
    #[serde(default)]
    pub targets: Vec<TargetDto>,
    #[serde(default)]
    pub asset_ids: Vec<Uuid>,
    #[serde(default)]
    pub control_ids: Vec<Uuid>,
    #[serde(default = "default_open")]
    pub status: ThreatStatus,
    #[serde(default)]
    pub dismiss_reason: Option<String>,
    #[serde(default)]
    pub inherent: Option<RatingDto>,
    #[serde(default)]
    pub current: Option<RatingDto>,
    #[serde(default = "default_undecided")]
    pub treatment: RiskTreatment,
    #[serde(default)]
    pub risk_owner: Option<String>,
    #[serde(default)]
    pub due_date: Option<NaiveDate>,
    #[serde(default)]
    pub acceptance_rationale: Option<String>,
    #[serde(default)]
    pub accepted_by: Option<String>,
}

impl ThreatInput {
    /// Validates the shape; the lifecycle is checked by the handler against
    /// the stored threat.
    pub fn into_fields(self) -> Result<t::ThreatFields, ApiError> {
        check_len("title", &self.title, 1, 300)?;
        let rating = |r: Option<RatingDto>, field| {
            r.map(|r| t::Rating::new(r.likelihood, r.impact, field))
                .transpose()
                .map_err(invalid)
        };
        Ok(t::ThreatFields {
            title: self.title,
            category: self.category,
            scenario: self.scenario,
            attacker_objective: self.attacker_objective,
            preconditions: self.preconditions,
            consequences: self.consequences,
            targets: self
                .targets
                .into_iter()
                .map(|x| t::ThreatTarget {
                    kind: x.kind,
                    id: x.id,
                })
                .collect(),
            asset_ids: self.asset_ids,
            control_ids: self.control_ids,
            status: self.status,
            dismiss_reason: blank_to_none(self.dismiss_reason),
            inherent: rating(self.inherent, "inherent")?,
            current: rating(self.current, "current")?,
            treatment: self.treatment,
            risk_owner: blank_to_none(self.risk_owner),
            due_date: self.due_date,
            acceptance_rationale: blank_to_none(self.acceptance_rationale),
            accepted_by: blank_to_none(self.accepted_by),
        })
    }
}

/// `Threat`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreatDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub display_id: String,
    pub title: String,
    pub category: StrideCategory,
    pub scenario: Option<String>,
    pub attacker_objective: Option<String>,
    pub preconditions: Option<String>,
    pub consequences: Option<String>,
    pub targets: Vec<TargetDto>,
    pub asset_ids: Vec<Uuid>,
    pub control_ids: Vec<Uuid>,
    pub status: ThreatStatus,
    pub dismiss_reason: Option<String>,
    pub inherent: Option<RatingDto>,
    pub current: Option<RatingDto>,
    pub treatment: RiskTreatment,
    pub risk_owner: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub acceptance_rationale: Option<String>,
    pub accepted_by: Option<String>,
    pub origin: ThreatOrigin,
    pub explanation: Option<RuleExplanation>,
    pub inherent_score: Option<u8>,
    pub current_score: Option<u8>,
    pub inherent_level: Option<RiskLevel>,
    pub current_level: Option<RiskLevel>,
    pub flags: Vec<ThreatFlag>,
}

/// `ThreatBulkStatus`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatBulkStatus {
    pub ids: Vec<Uuid>,
    pub status: ThreatStatus,
    #[serde(default)]
    pub reason: Option<String>,
}

/// `SuggestThreatsResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestThreatsResult {
    pub ruleset_version: i32,
    pub created: Vec<ThreatDto>,
    pub skipped: usize,
}

// ---------- controls ----------

fn default_planned() -> ControlStatus {
    ControlStatus::Planned
}

/// `SecurityControlInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub control_type: ControlType,
    pub category: ControlCategory,
    #[serde(default = "default_planned")]
    pub status: ControlStatus,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub acceptance_criteria: Option<String>,
    #[serde(default)]
    pub component_ids: Vec<Uuid>,
    #[serde(default)]
    pub threat_ids: Vec<Uuid>,
}

impl ControlInput {
    /// Validates.
    pub fn into_fields(self) -> Result<t::ControlFields, ApiError> {
        check_len("name", &self.name, 1, 300)?;
        Ok(t::ControlFields {
            name: self.name,
            description: self.description,
            control_type: self.control_type,
            category: self.category,
            status: self.status,
            owner: blank_to_none(self.owner),
            acceptance_criteria: blank_to_none(self.acceptance_criteria),
            component_ids: self.component_ids,
            threat_ids: self.threat_ids,
        })
    }
}

/// `SecurityControl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub display_id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub control_type: ControlType,
    pub category: ControlCategory,
    pub status: ControlStatus,
    pub owner: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub component_ids: Vec<Uuid>,
    pub threat_ids: Vec<Uuid>,
    pub verified_at: Option<DateTime<Utc>>,
    pub security_test_ids: Vec<Uuid>,
}

// ---------- security tests ----------

fn default_draft() -> SecurityTestStatus {
    SecurityTestStatus::Draft
}

/// `SecurityTestInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityTestInput {
    pub title: String,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub preconditions: Vec<String>,
    #[serde(default)]
    pub steps: Vec<SecurityTestStep>,
    #[serde(rename = "type")]
    pub test_type: SecurityTestType,
    #[serde(default = "default_draft")]
    pub status: SecurityTestStatus,
    #[serde(default)]
    pub threat_ids: Vec<Uuid>,
    #[serde(default)]
    pub control_ids: Vec<Uuid>,
}

impl SecurityTestInput {
    /// Validates.
    pub fn into_fields(self) -> Result<t::SecurityTestFields, ApiError> {
        check_len("title", &self.title, 1, 300)?;
        if self.threat_ids.is_empty() {
            return Err(invalid(ThreatModelError::new(
                "threatIds",
                "a security test needs at least one threat",
            )));
        }
        if let Some(i) = self.steps.iter().position(|s| s.action.trim().is_empty()) {
            return Err(invalid(ThreatModelError::new(
                "steps",
                format!("step {} has no action", i + 1),
            )));
        }
        Ok(t::SecurityTestFields {
            title: self.title,
            objective: self.objective,
            preconditions: self
                .preconditions
                .into_iter()
                .filter(|p| !p.trim().is_empty())
                .collect(),
            steps: self.steps,
            test_type: self.test_type,
            status: self.status,
            threat_ids: self.threat_ids,
            control_ids: self.control_ids,
        })
    }
}

/// `SecurityTestResultInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultInput {
    pub outcome: SecurityTestOutcome,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub executed_at: Option<DateTime<Utc>>,
}

/// `SecurityTestResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultDto {
    pub id: Uuid,
    pub security_test_id: Uuid,
    pub outcome: SecurityTestOutcome,
    pub note: Option<String>,
    pub executed_at: DateTime<Utc>,
}

impl From<&t::SecurityTestResult> for ResultDto {
    fn from(r: &t::SecurityTestResult) -> Self {
        Self {
            id: r.id,
            security_test_id: r.security_test_id,
            outcome: r.outcome,
            note: r.note.clone(),
            executed_at: r.executed_at,
        }
    }
}

/// `SecurityTest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityTestDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub display_id: String,
    pub title: String,
    pub objective: Option<String>,
    pub preconditions: Vec<String>,
    pub steps: Vec<SecurityTestStep>,
    #[serde(rename = "type")]
    pub test_type: SecurityTestType,
    pub status: SecurityTestStatus,
    pub threat_ids: Vec<Uuid>,
    pub control_ids: Vec<Uuid>,
    pub origin: SecurityTestOrigin,
    pub template_id: Option<String>,
    pub template_version: Option<i32>,
    pub stale: Option<String>,
    pub last_result: Option<ResultDto>,
}

/// Body of `POST …/security-tests/generate`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    #[serde(default)]
    pub threat_ids: Option<Vec<Uuid>>,
}

/// Body of `POST …/security-tests/bulk-status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestBulkStatus {
    pub ids: Vec<Uuid>,
    pub status: SecurityTestStatus,
}

/// `GenerateSecurityTestsResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResult {
    pub created: Vec<SecurityTestDto>,
    pub skipped: usize,
}

// ---------- summary ----------

/// `MatrixCell`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatrixCell {
    pub likelihood: u8,
    pub impact: u8,
    pub count: usize,
}

/// `RefLabel`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefLabel {
    pub id: Uuid,
    pub label: String,
}

/// `ThreatModelSummary`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryDto {
    pub components: usize,
    pub assets: usize,
    pub flows: usize,
    pub crossings: usize,
    pub threats_by_status: BTreeMap<String, usize>,
    pub open_by_level: BTreeMap<String, usize>,
    pub inherent_matrix: Vec<MatrixCell>,
    pub current_matrix: Vec<MatrixCell>,
    pub controls_verified_ratio: Option<f64>,
    pub threats_without_control: Vec<RefLabel>,
    pub controls_without_test: Vec<RefLabel>,
    pub warning_count: usize,
    pub overdue_count: usize,
}

// ---------- the view: aggregate → DTOs ----------

/// Maps entities of one aggregate, with their computed fields.
pub struct View<'a> {
    agg: &'a ThreatModelAggregate,
    today: NaiveDate,
    threats: HashMap<Uuid, &'a t::Threat>,
    controls: HashMap<Uuid, &'a t::Control>,
}

impl<'a> View<'a> {
    /// A view of `agg` as of `today` (for overdue).
    pub fn new(agg: &'a ThreatModelAggregate, today: NaiveDate) -> Self {
        Self {
            agg,
            today,
            threats: agg.threats.iter().map(|x| (x.audit.id, x)).collect(),
            controls: agg.controls.iter().map(|x| (x.audit.id, x)).collect(),
        }
    }

    /// Threat by id.
    pub fn threat(&self, id: Uuid) -> Option<&'a t::Threat> {
        self.threats.get(&id).copied()
    }

    /// Controls by id.
    pub fn controls(&self) -> &HashMap<Uuid, &'a t::Control> {
        &self.controls
    }

    /// The zone DTO.
    pub fn zone(&self, z: &t::Zone) -> ZoneDto {
        ZoneDto {
            audit: (&z.audit).into(),
            name: z.fields.name.clone(),
            description: z.fields.description.clone(),
            kind: z.fields.kind,
            trust_level: z.fields.trust_level,
        }
    }

    /// The component DTO.
    pub fn component(&self, c: &t::ArchComponent) -> ArchComponentDto {
        let f = &c.fields;
        ArchComponentDto {
            audit: (&c.audit).into(),
            name: f.name.clone(),
            description: f.description.clone(),
            component_type: f.component_type,
            owner: f.owner.clone(),
            technology: f.technology.clone(),
            criticality: f.criticality,
            internet_facing: f.internet_facing,
            zone_id: f.zone_id,
            linked_component_id: f.linked_component_id,
            linked_feature_id: f.linked_feature_id,
            asset_ids: f.asset_ids.clone(),
            layer: arch::layer(f.component_type),
            warnings: warnings(arch::component_warnings(&self.agg.arch, c)),
        }
    }

    /// The asset DTO.
    pub fn asset(&self, a: &t::Asset) -> AssetDto {
        let f = &a.fields;
        AssetDto {
            audit: (&a.audit).into(),
            name: f.name.clone(),
            description: f.description.clone(),
            category: f.category,
            owner: f.owner.clone(),
            confidentiality: f.confidentiality,
            integrity: f.integrity,
            availability: f.availability,
            personal_data: f.personal_data,
            component_ids: self.agg.arch.components_with_asset(a.audit.id),
            flow_ids: self.agg.arch.flows_with_asset(a.audit.id),
            warnings: warnings(arch::asset_warnings(&self.agg.arch, a)),
        }
    }

    /// The flow DTO.
    pub fn flow(&self, fl: &t::Flow) -> FlowDto {
        let f = &fl.fields;
        let c = arch::crossing(&self.agg.arch, fl);
        FlowDto {
            audit: (&fl.audit).into(),
            source_id: f.source_id,
            target_id: f.target_id,
            relationship: f.relationship,
            label: f.label.clone(),
            protocol: f.protocol.clone(),
            bidirectional: f.bidirectional,
            authentication: f.authentication.clone(),
            encrypted: f.encrypted,
            asset_ids: f.asset_ids.clone(),
            crossing: CrossingDto {
                crosses: c.crosses,
                direction: c.direction.map(|d| d.as_str().to_owned()),
                from_trust: c.from_trust,
                to_trust: c.to_trust,
            },
            warnings: warnings(arch::flow_warnings(&self.agg.arch, fl)),
        }
    }

    fn tests_of(&self, threat: Uuid) -> Vec<&'a t::SecurityTest> {
        self.agg
            .tests
            .iter()
            .filter(|x| x.fields.threat_ids.contains(&threat))
            .collect()
    }

    /// The threat's level for a rating.
    fn level(&self, r: Option<t::Rating>) -> Option<RiskLevel> {
        r.map(|r| self.agg.model.thresholds.level(r.score()))
    }

    /// The threat DTO.
    pub fn threat_dto(&self, th: &t::Threat) -> ThreatDto {
        let f = &th.fields;
        let rating = |r: Option<t::Rating>| {
            r.map(|r| RatingDto {
                likelihood: r.likelihood,
                impact: r.impact,
            })
        };
        ThreatDto {
            audit: (&th.audit).into(),
            display_id: th.display_id(),
            title: f.title.clone(),
            category: f.category,
            scenario: f.scenario.clone(),
            attacker_objective: f.attacker_objective.clone(),
            preconditions: f.preconditions.clone(),
            consequences: f.consequences.clone(),
            targets: f
                .targets
                .iter()
                .map(|x| TargetDto {
                    kind: x.kind,
                    id: x.id,
                })
                .collect(),
            asset_ids: f.asset_ids.clone(),
            control_ids: f.control_ids.clone(),
            status: f.status,
            dismiss_reason: f.dismiss_reason.clone(),
            inherent: rating(f.inherent),
            current: rating(f.current),
            treatment: f.treatment,
            risk_owner: f.risk_owner.clone(),
            due_date: f.due_date,
            acceptance_rationale: f.acceptance_rationale.clone(),
            accepted_by: f.accepted_by.clone(),
            origin: th.origin,
            explanation: th.explanation.clone(),
            inherent_score: f.inherent.map(t::Rating::score),
            current_score: f.current.map(t::Rating::score),
            inherent_level: self.level(f.inherent),
            current_level: self.level(f.current),
            flags: risk::threat_flags(th, &self.controls, &self.tests_of(th.audit.id), self.today),
        }
    }

    /// The control DTO.
    pub fn control(&self, c: &t::Control) -> ControlDto {
        let f = &c.fields;
        ControlDto {
            audit: (&c.audit).into(),
            display_id: c.display_id(),
            name: f.name.clone(),
            description: f.description.clone(),
            control_type: f.control_type,
            category: f.category,
            status: f.status,
            owner: f.owner.clone(),
            acceptance_criteria: f.acceptance_criteria.clone(),
            component_ids: f.component_ids.clone(),
            threat_ids: f.threat_ids.clone(),
            verified_at: c.verified_at,
            security_test_ids: self
                .agg
                .tests
                .iter()
                .filter(|x| x.fields.control_ids.contains(&c.audit.id))
                .map(|x| x.audit.id)
                .collect(),
        }
    }

    /// The security test DTO.
    pub fn test(&self, s: &t::SecurityTest) -> SecurityTestDto {
        let f = &s.fields;
        SecurityTestDto {
            audit: (&s.audit).into(),
            display_id: s.display_id(),
            title: f.title.clone(),
            objective: f.objective.clone(),
            preconditions: f.preconditions.clone(),
            steps: f.steps.clone(),
            test_type: f.test_type,
            status: f.status,
            threat_ids: f.threat_ids.clone(),
            control_ids: f.control_ids.clone(),
            origin: s.origin,
            template_id: s.source.as_ref().map(|x| x.template_id.clone()),
            template_version: s.source.as_ref().map(|x| x.template_version),
            stale: staleness::stale_reason(s, &self.threats, &self.controls),
            last_result: s.last_result.as_ref().map(Into::into),
        }
    }

    /// The Overview summary.
    pub fn summary(&self) -> SummaryDto {
        let agg = self.agg;
        let mut threats_by_status = BTreeMap::new();
        for s in ["candidate", "open", "mitigated", "accepted", "dismissed"] {
            threats_by_status.insert(s.to_owned(), 0);
        }
        let mut open_by_level = BTreeMap::new();
        for l in ["low", "moderate", "high", "critical", "not-assessed"] {
            open_by_level.insert(l.to_owned(), 0);
        }
        let mut inherent: HashMap<(u8, u8), usize> = HashMap::new();
        let mut current: HashMap<(u8, u8), usize> = HashMap::new();
        let mut without_control = Vec::new();
        let mut overdue = 0;
        for th in &agg.threats {
            *threats_by_status
                .entry(th.fields.status.as_str().to_owned())
                .or_default() += 1;
            if th.fields.status != ThreatStatus::Open {
                continue;
            }
            let level = self
                .level(th.fields.current)
                .map_or("not-assessed", RiskLevel::as_str);
            *open_by_level.entry(level.to_owned()).or_default() += 1;
            if let Some(r) = th.fields.inherent {
                *inherent.entry((r.likelihood, r.impact)).or_default() += 1;
            }
            if let Some(r) = th.fields.current {
                *current.entry((r.likelihood, r.impact)).or_default() += 1;
            }
            let flags = risk::threat_flags(th, &self.controls, &[], self.today);
            if flags.contains(&ThreatFlag::NoControl) {
                without_control.push(RefLabel {
                    id: th.audit.id,
                    label: format!("{} {}", th.display_id(), th.fields.title),
                });
            }
            if flags.contains(&ThreatFlag::Overdue) {
                overdue += 1;
            }
        }
        let matrix = |m: &HashMap<(u8, u8), usize>| {
            let mut cells = Vec::with_capacity(25);
            for likelihood in (1..=5).rev() {
                for impact in 1..=5 {
                    cells.push(MatrixCell {
                        likelihood,
                        impact,
                        count: m.get(&(likelihood, impact)).copied().unwrap_or(0),
                    });
                }
            }
            cells
        };
        let verified = agg
            .controls
            .iter()
            .filter(|c| c.fields.status == ControlStatus::Verified)
            .count();
        let controls_without_test = agg
            .controls
            .iter()
            .filter(|c| {
                !agg.tests
                    .iter()
                    .any(|x| x.fields.control_ids.contains(&c.audit.id))
            })
            .map(|c| RefLabel {
                id: c.audit.id,
                label: format!("{} {}", c.display_id(), c.fields.name),
            })
            .collect();
        SummaryDto {
            components: agg.arch.components.len(),
            assets: agg.arch.assets.len(),
            flows: agg.arch.flows.len(),
            crossings: arch::crossing_count(&agg.arch),
            threats_by_status,
            open_by_level,
            inherent_matrix: matrix(&inherent),
            current_matrix: matrix(&current),
            #[allow(clippy::cast_precision_loss)]
            controls_verified_ratio: (!agg.controls.is_empty())
                .then(|| verified as f64 / agg.controls.len() as f64),
            threats_without_control: without_control,
            controls_without_test,
            warning_count: arch::warning_count(&agg.arch),
            overdue_count: overdue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tm_domain::threat::arch::Architecture;
    use tm_domain::Audit;

    fn audit() -> Audit {
        let ts = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        Audit {
            id: Uuid::new_v4(),
            version: 1,
            created_at: ts,
            updated_at: ts,
        }
    }

    fn aggregate() -> ThreatModelAggregate {
        let z0 = t::Zone {
            audit: audit(),
            fields: t::ZoneFields {
                name: "Internet".into(),
                description: None,
                kind: ZoneKind::Internet,
                trust_level: 0,
            },
        };
        let z3 = t::Zone {
            audit: audit(),
            fields: t::ZoneFields {
                name: "Internal".into(),
                description: None,
                kind: ZoneKind::Internal,
                trust_level: 3,
            },
        };
        let comp = |name: &str, zone: Uuid| t::ArchComponent {
            audit: audit(),
            fields: t::ArchComponentFields {
                name: name.into(),
                description: None,
                component_type: ArchComponentType::Api,
                owner: None,
                technology: None,
                criticality: None,
                internet_facing: false,
                zone_id: Some(zone),
                linked_component_id: None,
                linked_feature_id: None,
                asset_ids: vec![],
            },
        };
        let web = comp("Web", z0.audit.id);
        let api = comp("API", z3.audit.id);
        let flow = t::Flow {
            audit: audit(),
            fields: t::FlowFields {
                source_id: web.audit.id,
                target_id: api.audit.id,
                relationship: FlowRelationship::Calls,
                label: None,
                protocol: None,
                bidirectional: false,
                authentication: "unknown".into(),
                encrypted: TriState::Unknown,
                asset_ids: vec![],
            },
        };
        let threat = t::Threat {
            audit: audit(),
            number: 1,
            fields: t::ThreatFields {
                title: "Spoof".into(),
                category: StrideCategory::Spoofing,
                scenario: None,
                attacker_objective: None,
                preconditions: None,
                consequences: None,
                targets: vec![t::ThreatTarget {
                    kind: TargetKind::Flow,
                    id: flow.audit.id,
                }],
                asset_ids: vec![],
                control_ids: vec![],
                status: ThreatStatus::Open,
                dismiss_reason: None,
                inherent: Some(t::Rating {
                    likelihood: 4,
                    impact: 5,
                }),
                current: Some(t::Rating {
                    likelihood: 3,
                    impact: 4,
                }),
                treatment: RiskTreatment::Mitigate,
                risk_owner: None,
                due_date: NaiveDate::from_ymd_opt(2026, 1, 1),
                acceptance_rationale: None,
                accepted_by: None,
            },
            origin: ThreatOrigin::Manual,
            explanation: None,
        };
        ThreatModelAggregate {
            model: t::ThreatModel {
                audit: audit(),
                project_id: Uuid::nil(),
                fields: t::ThreatModelFields {
                    name: "TM".into(),
                    ..Default::default()
                },
                thresholds: t::Thresholds::default(),
                layout: None,
                layout_version: 0,
            },
            arch: Architecture {
                zones: vec![z0, z3],
                components: vec![web, api],
                assets: vec![],
                flows: vec![flow],
            },
            threats: vec![threat],
            controls: vec![],
            tests: vec![],
            rule_keys: Default::default(),
        }
    }

    #[test]
    fn flow_carries_crossing_and_warnings() {
        let agg = aggregate();
        let v = View::new(&agg, NaiveDate::from_ymd_opt(2026, 10, 17).unwrap());
        let f = v.flow(&agg.arch.flows[0]);
        assert_eq!(
            f.crossing,
            CrossingDto {
                crosses: true,
                direction: Some("into-more-trusted".into()),
                from_trust: Some(0),
                to_trust: Some(3)
            }
        );
        assert_eq!(f.warnings.len(), 2);
        let json = serde_json::to_value(&f).unwrap();
        assert_eq!(json["crossing"]["fromTrust"], 0);
        assert!(json.get("version").is_some(), "audit is flattened");
    }

    #[test]
    fn threat_carries_scores_levels_and_flags() {
        let agg = aggregate();
        let v = View::new(&agg, NaiveDate::from_ymd_opt(2026, 10, 17).unwrap());
        let d = v.threat_dto(&agg.threats[0]);
        assert_eq!(d.display_id, "THR-1");
        assert_eq!((d.inherent_score, d.current_score), (Some(20), Some(12)));
        assert_eq!(
            (d.inherent_level, d.current_level),
            (Some(RiskLevel::Critical), Some(RiskLevel::High))
        );
        assert!(d.flags.contains(&ThreatFlag::NoControl));
        assert!(d.flags.contains(&ThreatFlag::UnsupportedReduction));
        assert!(d.flags.contains(&ThreatFlag::Overdue));
    }

    #[test]
    fn summary_counts() {
        let agg = aggregate();
        let s = View::new(&agg, NaiveDate::from_ymd_opt(2026, 10, 17).unwrap()).summary();
        assert_eq!((s.components, s.flows, s.crossings), (2, 1, 1));
        assert_eq!(s.threats_by_status["open"], 1);
        assert_eq!(s.open_by_level["high"], 1);
        assert_eq!(s.current_matrix.len(), 25);
        assert_eq!(
            s.current_matrix
                .iter()
                .find(|c| c.likelihood == 3 && c.impact == 4)
                .unwrap()
                .count,
            1
        );
        assert_eq!(s.threats_without_control.len(), 1);
        assert_eq!(s.threats_without_control[0].label, "THR-1 Spoof");
        assert_eq!(s.controls_verified_ratio, None);
        assert_eq!(s.overdue_count, 1);
    }

    #[test]
    fn inputs_validate() {
        let z = ZoneInput {
            name: "Z".into(),
            description: None,
            kind: ZoneKind::Dmz,
            trust_level: 5,
        };
        assert_eq!(
            z.into_fields().unwrap_err().status,
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
        let e = serde_json::from_value::<ArchComponentInput>(
            serde_json::json!({"name": "x", "type": "mainframe"}),
        )
        .unwrap_err()
        .to_string();
        assert!(e.contains("web-frontend"), "names the allowed types: {e}");
        let t: ThreatInput = serde_json::from_value(serde_json::json!({
            "title": "t", "category": "spoofing", "inherent": {"likelihood": 6, "impact": 1}
        }))
        .unwrap();
        assert_eq!(t.into_fields().unwrap_err().errors[0].field, "inherent");
        let s: SecurityTestInput =
            serde_json::from_value(serde_json::json!({"title": "t", "type": "other"})).unwrap();
        assert_eq!(s.into_fields().unwrap_err().errors[0].field, "threatIds");
        let f: FlowInput = serde_json::from_value(serde_json::json!({
            "sourceId": Uuid::nil(), "targetId": Uuid::nil(), "relationship": "calls"
        }))
        .unwrap();
        let f = f.into_fields().unwrap();
        assert_eq!(
            (f.authentication.as_str(), f.encrypted),
            ("unknown", TriState::Unknown)
        );
    }
}
