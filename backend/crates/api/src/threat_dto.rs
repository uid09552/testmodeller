//! Wire types of the threat model resources (tag `ThreatModel` in openapi.yaml)
//! and their mapping to the domain. Computed fields are derived from the
//! aggregate on every read (docs/specification/09-threat-modeling.md).

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use tm_domain::threat::{
    self as t, arch, assurance, risk, staleness, AiOrigin, ArchComponentType, AssetCategory,
    CiaLevel, Classification, Confidence, ControlCategory, ControlFlag, ControlStatus, ControlType,
    Criticality, EvidenceKind, EvidenceOwnerKind, FlowRelationship, RiskLevel, RiskTreatment,
    RuleExplanation, SecurityTestOrigin, SecurityTestOutcome, SecurityTestStatus, SecurityTestStep,
    SecurityTestType, StrideCategory, TargetKind, ThreatFlag, ThreatModelError, ThreatOrigin,
    ThreatStatus, TriState, ZoneKind,
};
use tm_storage::ThreatModelAggregate;
use uuid::Uuid;

use crate::dto::{check_len, AuditDto};
use crate::error::{ApiError, FieldError};

/// Origin of entities stored before AI proposals existed.
fn manual_origin() -> AiOrigin {
    AiOrigin::Manual
}

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
    /// Absent keeps the stored value (off for a new threat model).
    #[serde(default)]
    pub ai_enabled: Option<bool>,
}

impl ThreatModelInput {
    /// Validates; absent thresholds mean `fallback`, an absent `aiEnabled` means `current_ai`.
    pub fn into_parts(
        self,
        fallback: t::Thresholds,
        current_ai: bool,
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
                ai_enabled: self.ai_enabled.unwrap_or(current_ai),
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
    #[serde(default)]
    pub ai_enabled: bool,
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
            ai_enabled: m.fields.ai_enabled,
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
    #[serde(default = "manual_origin")]
    pub origin: AiOrigin,
    #[serde(default)]
    pub source: Option<String>,
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
    #[serde(default = "manual_origin")]
    pub origin: AiOrigin,
    #[serde(default)]
    pub source: Option<String>,
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
    #[serde(default)]
    pub classification: Option<Classification>,
    #[serde(default)]
    pub data_location: Option<String>,
    #[serde(default)]
    pub retention: Option<String>,
    #[serde(default = "default_unknown")]
    pub encryption_at_rest: TriState,
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
            classification: self.classification,
            data_location: blank_to_none(self.data_location),
            retention: blank_to_none(self.retention),
            encryption_at_rest: self.encryption_at_rest,
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
    pub classification: Option<Classification>,
    pub data_location: Option<String>,
    pub retention: Option<String>,
    pub encryption_at_rest: TriState,
    pub component_ids: Vec<Uuid>,
    pub flow_ids: Vec<Uuid>,
    pub warnings: Vec<WarningDto>,
    #[serde(default = "manual_origin")]
    pub origin: AiOrigin,
    #[serde(default)]
    pub source: Option<String>,
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
    #[serde(default)]
    pub data_classification: Option<Classification>,
    #[serde(default)]
    pub authorization_model: Option<String>,
    #[serde(default)]
    pub entry_point: bool,
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
            data_classification: self.data_classification,
            authorization_model: blank_to_none(self.authorization_model),
            entry_point: self.entry_point,
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
    pub data_classification: Option<Classification>,
    pub authorization_model: Option<String>,
    pub entry_point: bool,
    pub crossing: CrossingDto,
    pub warnings: Vec<WarningDto>,
    #[serde(default = "manual_origin")]
    pub origin: AiOrigin,
    #[serde(default)]
    pub source: Option<String>,
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
    #[serde(default)]
    pub residual: Option<RatingDto>,
    #[serde(default)]
    pub threat_source: Option<String>,
    #[serde(default)]
    pub detection: Option<String>,
    #[serde(default)]
    pub existing_mitigations: Option<String>,
    #[serde(default)]
    pub confidence: Option<Confidence>,
    #[serde(default)]
    pub acceptance_expires: Option<NaiveDate>,
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
            residual: rating(self.residual, "residual")?,
            threat_source: blank_to_none(self.threat_source),
            detection: blank_to_none(self.detection),
            existing_mitigations: blank_to_none(self.existing_mitigations),
            confidence: self.confidence,
            acceptance_expires: self.acceptance_expires,
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
    pub residual: Option<RatingDto>,
    pub threat_source: Option<String>,
    pub detection: Option<String>,
    pub existing_mitigations: Option<String>,
    pub confidence: Option<Confidence>,
    pub acceptance_expires: Option<NaiveDate>,
    pub accepted_on: Option<NaiveDate>,
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
    pub residual_score: Option<u8>,
    pub residual_level: Option<RiskLevel>,
    pub effective_status: ThreatStatus,
    pub affected_boundaries: Vec<CrossingDto>,
    pub flags: Vec<ThreatFlag>,
    /// Why a residual flag is shown (the linked controls that are not verified).
    pub residual_facts: Vec<String>,
    #[serde(default)]
    pub source: Option<String>,
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
    pub review_by: Option<NaiveDate>,
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
            review_by: self.review_by,
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
    pub review_by: Option<NaiveDate>,
    pub component_ids: Vec<Uuid>,
    pub threat_ids: Vec<Uuid>,
    pub verified_at: Option<DateTime<Utc>>,
    pub security_test_ids: Vec<Uuid>,
    pub evidence_count: usize,
    pub flags: Vec<ControlFlag>,
    #[serde(default = "manual_origin")]
    pub origin: AiOrigin,
    #[serde(default)]
    pub source: Option<String>,
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
    #[serde(default)]
    pub source: Option<String>,
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
    pub residual_matrix: Vec<MatrixCell>,
    pub completeness: Vec<FindingDto>,
    pub expiring_acceptances: Vec<RefLabel>,
    pub expired_acceptances: usize,
    pub verification_overdue: usize,
}

/// `CompletenessFinding`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FindingDto {
    pub code: String,
    pub message: String,
    pub entity_kind: String,
    pub entity_id: Uuid,
}

// ---------- evidence ----------

/// `EvidenceInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceInput {
    pub kind: EvidenceKind,
    pub title: String,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    pub evidence_date: NaiveDate,
    #[serde(default)]
    pub reviewer: Option<String>,
}

impl EvidenceInput {
    /// Validates.
    pub fn into_fields(self) -> Result<assurance::EvidenceFields, ApiError> {
        check_len("title", &self.title, 1, 300)?;
        if let Some(l) = &self.link {
            check_len("link", l, 0, 2000)?;
        }
        if let Some(n) = &self.note {
            check_len("note", n, 0, 4000)?;
        }
        let f = assurance::EvidenceFields {
            kind: self.kind,
            title: self.title,
            link: blank_to_none(self.link),
            note: blank_to_none(self.note),
            evidence_date: self.evidence_date,
            reviewer: blank_to_none(self.reviewer),
        };
        assurance::check_evidence(&f).map_err(invalid)?;
        Ok(f)
    }
}

/// `Evidence`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceDto {
    #[serde(flatten)]
    pub audit: AuditDto,
    pub kind: EvidenceKind,
    pub title: String,
    pub link: Option<String>,
    pub note: Option<String>,
    pub evidence_date: NaiveDate,
    pub reviewer: Option<String>,
    pub owner_kind: EvidenceOwnerKind,
    pub owner_id: Uuid,
}

impl From<&assurance::Evidence> for EvidenceDto {
    fn from(e: &assurance::Evidence) -> Self {
        Self {
            audit: (&e.audit).into(),
            kind: e.fields.kind,
            title: e.fields.title.clone(),
            link: e.fields.link.clone(),
            note: e.fields.note.clone(),
            evidence_date: e.fields.evidence_date,
            reviewer: e.fields.reviewer.clone(),
            owner_kind: e.owner_kind,
            owner_id: e.owner_id,
        }
    }
}

// ---------- baselines ----------

/// Version of the snapshot format.
pub const SNAPSHOT_VERSION: i32 = 1;

/// `BaselineInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineInput {
    pub name: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// `Baseline`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineDto {
    pub id: Uuid,
    pub name: String,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub threat_count: i32,
    pub control_count: i32,
    pub schema_version: i32,
}

impl From<&tm_storage::BaselineMeta> for BaselineDto {
    fn from(m: &tm_storage::BaselineMeta) -> Self {
        Self {
            id: m.id,
            name: m.name.clone(),
            note: m.note.clone(),
            created_at: m.created_at,
            created_by: m.created_by.clone(),
            threat_count: m.threat_count,
            control_count: m.control_count,
            schema_version: m.schema_version,
        }
    }
}

/// `ThreatModelSnapshot`: the read model of one threat model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDto {
    pub threat_model: ThreatModelDto,
    pub zones: Vec<ZoneDto>,
    pub components: Vec<ArchComponentDto>,
    pub assets: Vec<AssetDto>,
    pub flows: Vec<FlowDto>,
    pub threats: Vec<ThreatDto>,
    pub controls: Vec<ControlDto>,
    pub security_tests: Vec<SecurityTestDto>,
    #[serde(default)]
    pub evidence: Vec<EvidenceDto>,
}

impl SnapshotDto {
    /// Refreshes what depends on the clock and on the stored ratings: scores,
    /// levels and effective status. Flags and warnings stay as they were when
    /// the snapshot was taken.
    pub fn refresh(&mut self, today: NaiveDate) {
        let thresholds = thresholds_of(&self.threat_model);
        let score = |r: &Option<RatingDto>| r.map(|r| r.likelihood * r.impact);
        let level = |s: Option<u8>| s.map(|s| thresholds.level(s));
        for th in &mut self.threats {
            th.inherent_score = score(&th.inherent);
            th.current_score = score(&th.current);
            th.residual_score = score(&th.residual);
            th.inherent_level = level(th.inherent_score);
            th.current_level = level(th.current_score);
            th.residual_level = level(th.residual_score);
            let lapsed = th.status == ThreatStatus::Accepted
                && th.acceptance_expires.is_some_and(|d| d < today);
            th.effective_status = if lapsed {
                ThreatStatus::Open
            } else {
                th.status
            };
        }
    }

    /// Digests for the comparison.
    pub fn digests(&self) -> (Vec<assurance::ThreatDigest>, Vec<assurance::ControlDigest>) {
        let threats = self
            .threats
            .iter()
            .map(|t| assurance::ThreatDigest {
                id: t.audit.id,
                display_id: t.display_id.clone(),
                title: t.title.clone(),
                status: t.effective_status.to_string(),
                inherent: t.inherent_level.map(|l| l.to_string()),
                current: t.current_level.map(|l| l.to_string()),
                residual: t.residual_level.map(|l| l.to_string()),
                treatment: t.treatment.to_string(),
            })
            .collect();
        let controls = self
            .controls
            .iter()
            .map(|c| assurance::ControlDigest {
                id: c.audit.id,
                display_id: c.display_id.clone(),
                status: c.status.to_string(),
            })
            .collect();
        (threats, controls)
    }
}

fn thresholds_of(m: &ThreatModelDto) -> t::Thresholds {
    t::Thresholds::new(
        m.thresholds
            .iter()
            .map(|x| t::Threshold {
                level: x.level,
                min: x.min,
                max: x.max,
            })
            .collect(),
    )
    .unwrap_or_default()
}

/// `BaselineDetail`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineDetailDto {
    #[serde(flatten)]
    pub meta: BaselineDto,
    pub snapshot: SnapshotDto,
}

/// `ComparedThreat`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparedThreatDto {
    pub id: Uuid,
    pub display_id: String,
    pub title: String,
    pub changes: Vec<FieldChangeDto>,
}

/// One entry of `ComparedThreat.changes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChangeDto {
    pub field: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// Entry of `BaselineComparison.controlsChanged`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlChangeDto {
    pub id: Uuid,
    pub display_id: String,
    pub before: String,
    pub after: String,
}

/// `BaselineComparison`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonDto {
    pub added: Vec<ComparedThreatDto>,
    pub removed: Vec<ComparedThreatDto>,
    pub changed: Vec<ComparedThreatDto>,
    pub controls_changed: Vec<ControlChangeDto>,
    pub levels_baseline: BTreeMap<String, usize>,
    pub levels_live: BTreeMap<String, usize>,
}

impl From<assurance::Comparison> for ComparisonDto {
    fn from(c: assurance::Comparison) -> Self {
        let threat = |x: assurance::ComparedThreat| ComparedThreatDto {
            id: x.id,
            display_id: x.display_id,
            title: x.title,
            changes: x
                .changes
                .into_iter()
                .map(|c| FieldChangeDto {
                    field: c.field.to_owned(),
                    before: c.before,
                    after: c.after,
                })
                .collect(),
        };
        Self {
            added: c.added.into_iter().map(threat).collect(),
            removed: c.removed.into_iter().map(threat).collect(),
            changed: c.changed.into_iter().map(threat).collect(),
            controls_changed: c
                .controls_changed
                .into_iter()
                .map(|x| ControlChangeDto {
                    id: x.id,
                    display_id: x.display_id,
                    before: x.before,
                    after: x.after,
                })
                .collect(),
            levels_baseline: c.levels_baseline,
            levels_live: c.levels_live,
        }
    }
}

// ---------- the view: aggregate → DTOs ----------

/// Maps entities of one aggregate, with their computed fields.
pub struct View<'a> {
    agg: &'a ThreatModelAggregate,
    today: NaiveDate,
    threats: HashMap<Uuid, &'a t::Threat>,
    controls: HashMap<Uuid, &'a t::Control>,
    evidence_counts: HashMap<Uuid, usize>,
}

impl<'a> View<'a> {
    /// A view of `agg` as of `today` (for overdue).
    pub fn new(agg: &'a ThreatModelAggregate, today: NaiveDate) -> Self {
        Self {
            agg,
            today,
            threats: agg.threats.iter().map(|x| (x.audit.id, x)).collect(),
            controls: agg.controls.iter().map(|x| (x.audit.id, x)).collect(),
            evidence_counts: {
                let mut m = HashMap::new();
                for e in &agg.evidence {
                    *m.entry(e.owner_id).or_insert(0) += 1;
                }
                m
            },
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
            origin: z.provenance.origin,
            source: z.provenance.source.clone(),
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
            origin: c.provenance.origin,
            source: c.provenance.source.clone(),
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
            origin: a.provenance.origin,
            source: a.provenance.source.clone(),
            audit: (&a.audit).into(),
            name: f.name.clone(),
            description: f.description.clone(),
            category: f.category,
            owner: f.owner.clone(),
            confidentiality: f.confidentiality,
            integrity: f.integrity,
            availability: f.availability,
            personal_data: f.personal_data,
            classification: f.classification,
            data_location: f.data_location.clone(),
            retention: f.retention.clone(),
            encryption_at_rest: f.encryption_at_rest,
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
            origin: fl.provenance.origin,
            source: fl.provenance.source.clone(),
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
            data_classification: f.data_classification,
            authorization_model: f.authorization_model.clone(),
            entry_point: f.entry_point,
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

    /// The trust-boundary crossings a threat is exposed to: the crossing flows
    /// it targets, or those of the components it targets.
    fn affected_boundaries(&self, th: &t::Threat) -> Vec<CrossingDto> {
        let a = &self.agg.arch;
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for target in &th.fields.targets {
            let flows: Vec<&t::Flow> = match target.kind {
                TargetKind::Flow => a.flow(target.id).into_iter().collect(),
                TargetKind::Component => a
                    .flows
                    .iter()
                    .filter(|f| f.fields.source_id == target.id || f.fields.target_id == target.id)
                    .collect(),
            };
            for f in flows {
                let c = arch::crossing(a, f);
                if c.crosses && seen.insert(f.audit.id) {
                    out.push(CrossingDto {
                        crosses: true,
                        direction: c.direction.map(|d| d.as_str().to_owned()),
                        from_trust: c.from_trust,
                        to_trust: c.to_trust,
                    });
                }
            }
        }
        out
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
            source: th.source_model.clone(),
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
            residual: rating(f.residual),
            threat_source: f.threat_source.clone(),
            detection: f.detection.clone(),
            existing_mitigations: f.existing_mitigations.clone(),
            confidence: f.confidence,
            acceptance_expires: f.acceptance_expires,
            accepted_on: th.accepted_on,
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
            residual_score: f.residual.map(t::Rating::score),
            residual_level: self.level(f.residual),
            effective_status: risk::effective_status(th, self.today),
            affected_boundaries: self.affected_boundaries(th),
            flags: risk::threat_flags(th, &self.controls, &self.tests_of(th.audit.id), self.today),
            residual_facts: risk::unsupported_residual_facts(th, &self.controls),
        }
    }

    fn evidence_count(&self, id: Uuid) -> usize {
        self.evidence_counts.get(&id).copied().unwrap_or(0)
    }

    /// Evidence DTOs of the aggregate.
    pub fn evidence(&self) -> Vec<EvidenceDto> {
        self.agg.evidence.iter().map(Into::into).collect()
    }

    /// The whole read model, as stored in a baseline.
    pub fn snapshot(&self) -> SnapshotDto {
        let a = self.agg;
        SnapshotDto {
            threat_model: (&a.model).into(),
            zones: a.arch.zones.iter().map(|x| self.zone(x)).collect(),
            components: a
                .arch
                .components
                .iter()
                .map(|x| self.component(x))
                .collect(),
            assets: a.arch.assets.iter().map(|x| self.asset(x)).collect(),
            flows: a.arch.flows.iter().map(|x| self.flow(x)).collect(),
            threats: a.threats.iter().map(|x| self.threat_dto(x)).collect(),
            controls: a.controls.iter().map(|x| self.control(x)).collect(),
            security_tests: a.tests.iter().map(|x| self.test(x)).collect(),
            evidence: self.evidence(),
        }
    }

    /// Digests of the live model for the comparison with a baseline.
    pub fn digests(&self) -> (Vec<assurance::ThreatDigest>, Vec<assurance::ControlDigest>) {
        let threats = self
            .agg
            .threats
            .iter()
            .map(|x| assurance::threat_digest(x, &self.agg.model.thresholds, self.today))
            .collect();
        let controls = self
            .agg
            .controls
            .iter()
            .map(|c| assurance::ControlDigest {
                id: c.audit.id,
                display_id: c.display_id(),
                status: c.fields.status.to_string(),
            })
            .collect();
        (threats, controls)
    }

    /// The control DTO.
    pub fn control(&self, c: &t::Control) -> ControlDto {
        let f = &c.fields;
        ControlDto {
            origin: c.provenance.origin,
            source: c.provenance.source.clone(),
            audit: (&c.audit).into(),
            display_id: c.display_id(),
            name: f.name.clone(),
            description: f.description.clone(),
            control_type: f.control_type,
            category: f.category,
            status: f.status,
            owner: f.owner.clone(),
            acceptance_criteria: f.acceptance_criteria.clone(),
            review_by: f.review_by,
            component_ids: f.component_ids.clone(),
            threat_ids: f.threat_ids.clone(),
            verified_at: c.verified_at,
            evidence_count: self.evidence_count(c.audit.id),
            flags: assurance::control_flags(c, self.evidence_count(c.audit.id), self.today),
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
            source: s.source_model.clone(),
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
        let mut residual: HashMap<(u8, u8), usize> = HashMap::new();
        let mut without_control = Vec::new();
        let mut overdue = 0;
        let mut expiring = Vec::new();
        let mut expired = 0;
        for th in &agg.threats {
            let status = risk::effective_status(th, self.today);
            *threats_by_status
                .entry(status.as_str().to_owned())
                .or_default() += 1;
            if risk::acceptance_expired(th, self.today) {
                expired += 1;
            }
            if th.fields.status == ThreatStatus::Accepted {
                if let Some(d) = th.fields.acceptance_expires {
                    if d >= self.today
                        && d <= self.today + chrono::Duration::days(assurance::EXPIRING_DAYS)
                    {
                        expiring.push(RefLabel {
                            id: th.audit.id,
                            label: format!("{} {}", th.display_id(), th.fields.title),
                        });
                    }
                }
            }
            if status != ThreatStatus::Open {
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
            if let Some(r) = th.fields.residual {
                *residual.entry((r.likelihood, r.impact)).or_default() += 1;
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
            residual_matrix: matrix(&residual),
            completeness: assurance::completeness(
                &agg.arch,
                &agg.threats,
                &agg.controls,
                &self.evidence_counts,
                self.today,
            )
            .into_iter()
            .map(|f| FindingDto {
                code: f.code.to_owned(),
                message: f.message,
                entity_kind: f.entity_kind.to_owned(),
                entity_id: f.entity_id,
            })
            .collect(),
            expiring_acceptances: expiring,
            expired_acceptances: expired,
            verification_overdue: agg
                .controls
                .iter()
                .filter(|c| {
                    assurance::control_flags(c, self.evidence_count(c.audit.id), self.today)
                        .contains(&ControlFlag::VerificationOverdue)
                })
                .count(),
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
            provenance: t::Provenance::default(),
            fields: t::ZoneFields {
                name: "Internet".into(),
                description: None,
                kind: ZoneKind::Internet,
                trust_level: 0,
            },
        };
        let z3 = t::Zone {
            audit: audit(),
            provenance: t::Provenance::default(),
            fields: t::ZoneFields {
                name: "Internal".into(),
                description: None,
                kind: ZoneKind::Internal,
                trust_level: 3,
            },
        };
        let comp = |name: &str, zone: Uuid| t::ArchComponent {
            audit: audit(),
            provenance: t::Provenance::default(),
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
            provenance: t::Provenance::default(),
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
                data_classification: None,
                authorization_model: None,
                entry_point: false,
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
                residual: None,
                threat_source: None,
                detection: None,
                existing_mitigations: None,
                confidence: None,
                treatment: RiskTreatment::Mitigate,
                risk_owner: None,
                due_date: NaiveDate::from_ymd_opt(2026, 1, 1),
                acceptance_rationale: None,
                accepted_by: None,
                acceptance_expires: None,
            },
            origin: ThreatOrigin::Manual,
            explanation: None,
            accepted_on: None,
            source_model: None,
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
            evidence: vec![],
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

    #[test]
    fn threat_maps_effective_status_residual_and_boundaries() {
        let mut agg = aggregate();
        agg.threats[0].fields.residual = t::Rating::new(2, 2, "residual").ok();
        agg.threats[0].fields.treatment = RiskTreatment::Accept;
        agg.threats[0].fields.status = ThreatStatus::Accepted;
        agg.threats[0].fields.acceptance_expires = NaiveDate::from_ymd_opt(2026, 10, 16);
        let v = View::new(&agg, NaiveDate::from_ymd_opt(2026, 10, 17).unwrap());
        let d = v.threat_dto(&agg.threats[0]);
        assert_eq!(d.status, ThreatStatus::Accepted);
        assert_eq!(d.effective_status, ThreatStatus::Open);
        assert!(d.flags.contains(&ThreatFlag::AcceptanceExpired));
        assert_eq!(d.residual_score, Some(4));
        assert_eq!(d.residual_level, Some(RiskLevel::Low));
        assert_eq!(d.affected_boundaries.len(), 1);
    }

    #[test]
    fn snapshot_refresh_recomputes_levels_and_lapsed_acceptance() {
        let agg = aggregate();
        let v = View::new(&agg, NaiveDate::from_ymd_opt(2026, 10, 17).unwrap());
        let mut snap = v.snapshot();
        snap.threats[0].current_level = None;
        snap.threats[0].status = ThreatStatus::Accepted;
        snap.threats[0].acceptance_expires = NaiveDate::from_ymd_opt(2026, 10, 1);
        snap.refresh(NaiveDate::from_ymd_opt(2026, 10, 17).unwrap());
        assert_eq!(snap.threats[0].current_level, Some(RiskLevel::High));
        assert_eq!(snap.threats[0].effective_status, ThreatStatus::Open);
        let json = serde_json::to_value(&snap).unwrap();
        let back: SnapshotDto = serde_json::from_value(json).unwrap();
        assert_eq!(back.threats.len(), 1);
    }

    #[test]
    fn ai_flag_is_kept_unless_the_input_says_otherwise() {
        let input = |ai: Option<bool>| ThreatModelInput {
            name: "TM".into(),
            scope: None,
            objectives: None,
            assumptions: None,
            exclusions: None,
            thresholds: None,
            ai_enabled: ai,
        };
        let parts = |ai, current| {
            input(ai)
                .into_parts(t::Thresholds::default(), current)
                .unwrap()
                .0
                .ai_enabled
        };
        assert!(parts(None, true), "absent keeps the stored value");
        assert!(!parts(None, false));
        assert!(!parts(Some(false), true), "explicit off wins");
        assert!(parts(Some(true), false), "explicit on wins");
    }

    #[test]
    fn old_snapshots_without_provenance_still_read() {
        let agg = aggregate();
        let v = View::new(&agg, NaiveDate::from_ymd_opt(2026, 10, 17).unwrap());
        let mut json = serde_json::to_value(v.snapshot()).unwrap();
        // Threats and tests always had an origin; the others gained it with AI.
        for (list, had_origin) in [
            ("zones", false),
            ("components", false),
            ("assets", false),
            ("flows", false),
            ("controls", false),
            ("threats", true),
            ("securityTests", true),
        ] {
            for item in json[list].as_array_mut().unwrap() {
                let o = item.as_object_mut().unwrap();
                if !had_origin {
                    o.remove("origin");
                }
                o.remove("source");
            }
        }
        let back: SnapshotDto = serde_json::from_value(json).expect("a baseline from before AI");
        assert_eq!(back.zones[0].origin, AiOrigin::Manual);
    }
}
