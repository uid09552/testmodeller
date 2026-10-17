//! Threat modeling: architecture, threats, risk, controls and security tests.
//! Pure functions only. See docs/specification/09-threat-modeling.md.

pub mod arch;
pub mod lifecycle;
pub mod risk;
pub mod rules;
pub mod staleness;
pub mod templates;
#[cfg(test)]
mod testutil;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::{string_enum, Audit};
pub use risk::{Rating, RiskLevel, Threshold, Thresholds};

string_enum!(
    /// Kind of architecture component.
    ArchComponentType {
        WebFrontend => "web-frontend",
        MobileApp => "mobile-app",
        DesktopClient => "desktop-client",
        BrowserExtension => "browser-extension",
        ApiGateway => "api-gateway",
        Api => "api",
        MessageBroker => "message-broker",
        Webhook => "webhook",
        ExternalConnector => "external-connector",
        Service => "service",
        Worker => "worker",
        Scheduler => "scheduler",
        ServerlessFunction => "serverless-function",
        RelationalDatabase => "relational-database",
        ObjectStorage => "object-storage",
        Cache => "cache",
        SearchIndex => "search-index",
        FileStore => "file-store",
        Backup => "backup",
        IdentityProvider => "identity-provider",
        AuthorizationService => "authorization-service",
        SecretsManager => "secrets-manager",
        KeyManagement => "key-management",
        AuditService => "audit-service",
        CloudService => "cloud-service",
        ContainerPlatform => "container-platform",
        CicdPipeline => "cicd-pipeline",
        ThirdPartySaas => "third-party-saas",
        Library => "library",
        HumanUser => "human-user",
        ExternalSystem => "external-system",
    }
);
string_enum!(
    /// Architecture layer a component type belongs to.
    ArchLayer {
        Client => "client",
        Integration => "integration",
        Logic => "logic",
        Data => "data",
        IdentitySecurity => "identity-security",
        Platform => "platform",
        External => "external",
    }
);
string_enum!(
    /// Business criticality of a component.
    Criticality { Low => "low", Medium => "medium", High => "high", Critical => "critical" }
);
string_enum!(
    /// Kind of exposure zone.
    ZoneKind {
        Internet => "internet",
        Partner => "partner",
        Dmz => "dmz",
        Internal => "internal",
        RestrictedData => "restricted-data",
        Management => "management",
        Identity => "identity",
        Cicd => "cicd",
        ThirdParty => "third-party",
        Other => "other",
    }
);
string_enum!(
    /// Asset category.
    AssetCategory {
        BusinessData => "business-data",
        IdentitySecrets => "identity-secrets",
        SoftwareCode => "software-code",
        Infrastructure => "infrastructure",
        SecurityAsset => "security-asset",
        OperationalAsset => "operational-asset",
        BusinessCapability => "business-capability",
    }
);
string_enum!(
    /// Confidentiality, integrity or availability requirement.
    CiaLevel { Low => "low", Medium => "medium", High => "high" }
);
string_enum!(
    /// Relationship a data flow expresses.
    FlowRelationship {
        Calls => "calls",
        SendsData => "sends-data",
        ReadsWrites => "reads-writes",
        AuthenticatesThrough => "authenticates-through",
        Trusts => "trusts",
        DependsOn => "depends-on",
        RunsOn => "runs-on",
        PublishesSubscribes => "publishes-subscribes",
        ReplicatesBacksUp => "replicates-backs-up",
        Deploys => "deploys",
    }
);
string_enum!(
    /// Yes, no or not documented.
    TriState { Yes => "yes", No => "no", Unknown => "unknown" }
);
string_enum!(
    /// STRIDE threat category.
    StrideCategory {
        Spoofing => "spoofing",
        Tampering => "tampering",
        Repudiation => "repudiation",
        InformationDisclosure => "information-disclosure",
        DenialOfService => "denial-of-service",
        ElevationOfPrivilege => "elevation-of-privilege",
    }
);
string_enum!(
    /// Lifecycle status of a threat.
    ThreatStatus {
        Candidate => "candidate",
        Open => "open",
        Mitigated => "mitigated",
        Accepted => "accepted",
        Dismissed => "dismissed",
    }
);
string_enum!(
    /// How a threat came into existence.
    ThreatOrigin { Manual => "manual", Rule => "rule" }
);
string_enum!(
    /// How a risk is treated.
    RiskTreatment {
        Undecided => "undecided",
        Mitigate => "mitigate",
        Avoid => "avoid",
        Transfer => "transfer",
        Accept => "accept",
    }
);
string_enum!(
    /// Computed marker on a threat.
    ThreatFlag {
        NoControl => "no-control",
        PlannedControlsOnly => "planned-controls-only",
        UnsupportedReduction => "unsupported-reduction",
        Overdue => "overdue",
        FailingTest => "failing-test",
    }
);
string_enum!(
    /// What a security control does.
    ControlType {
        Preventive => "preventive",
        Detective => "detective",
        Corrective => "corrective",
        Compensating => "compensating",
    }
);
string_enum!(
    /// Area of a security control.
    ControlCategory {
        IdentityAccess => "identity-access",
        DataProtection => "data-protection",
        ApplicationNetwork => "application-network",
        DetectionResponse => "detection-response",
        SecureDevelopment => "secure-development",
    }
);
string_enum!(
    /// Implementation status of a security control.
    ControlStatus { Planned => "planned", Implemented => "implemented", Verified => "verified" }
);
string_enum!(
    /// Kind of security test.
    SecurityTestType {
        NegativeAuthorization => "negative-authorization",
        Authentication => "authentication",
        InputValidation => "input-validation",
        TransportSecurity => "transport-security",
        ConfigurationReview => "configuration-review",
        Resilience => "resilience",
        LoggingAudit => "logging-audit",
        Other => "other",
    }
);
string_enum!(
    /// Lifecycle status of a security test.
    SecurityTestStatus { Draft => "draft", Ready => "ready", Obsolete => "obsolete" }
);
string_enum!(
    /// How a security test came into existence.
    SecurityTestOrigin { Manual => "manual", Generated => "generated" }
);
string_enum!(
    /// Recorded outcome of a security test run.
    SecurityTestOutcome { Passed => "passed", Failed => "failed", NotRun => "not-run" }
);
string_enum!(
    /// Kind of entity a threat targets.
    TargetKind { Component => "component", Flow => "flow" }
);

/// Display id prefix for threats.
pub const THREAT_PREFIX: &str = "THR";
/// Display id prefix for controls.
pub const CONTROL_PREFIX: &str = "CTL";
/// Display id prefix for security tests.
pub const TEST_PREFIX: &str = "SEC";

/// Formats a display id such as `THR-3`.
pub fn display_id(prefix: &str, number: i32) -> String {
    format!("{prefix}-{number}")
}

/// Editable fields of a threat model.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThreatModelFields {
    /// Name.
    pub name: String,
    /// Scope (markdown).
    pub scope: Option<String>,
    /// Objectives (markdown).
    pub objectives: Option<String>,
    /// Assumptions (markdown).
    pub assumptions: Option<String>,
    /// Exclusions (markdown).
    pub exclusions: Option<String>,
}

/// A project's threat model.
#[derive(Debug, Clone, PartialEq)]
pub struct ThreatModel {
    /// Audit fields.
    pub audit: Audit,
    /// Owning project.
    pub project_id: Uuid,
    /// Editable fields.
    pub fields: ThreatModelFields,
    /// Score-to-level mapping.
    pub thresholds: Thresholds,
    /// Diagram layout, opaque.
    pub layout: Option<serde_json::Value>,
    /// Version of the layout, independent of `audit.version`.
    pub layout_version: i32,
}

/// Editable fields of an exposure zone.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneFields {
    /// Unique name within the threat model.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Kind.
    pub kind: ZoneKind,
    /// Trust level 0 (untrusted) to 4.
    pub trust_level: u8,
}

/// Exposure zone.
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    /// Audit fields.
    pub audit: Audit,
    /// Editable fields.
    pub fields: ZoneFields,
}

/// Editable fields of an architecture component.
#[derive(Debug, Clone, PartialEq)]
pub struct ArchComponentFields {
    /// Unique name within the threat model.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Type.
    pub component_type: ArchComponentType,
    /// Owner.
    pub owner: Option<String>,
    /// Technology.
    pub technology: Option<String>,
    /// Criticality.
    pub criticality: Option<Criticality>,
    /// Reachable from the internet.
    pub internet_facing: bool,
    /// Zone it runs in.
    pub zone_id: Option<Uuid>,
    /// Linked Component of the same project.
    pub linked_component_id: Option<Uuid>,
    /// Linked Feature of the same project.
    pub linked_feature_id: Option<Uuid>,
    /// Assets it handles.
    pub asset_ids: Vec<Uuid>,
}

/// Architecture component.
#[derive(Debug, Clone, PartialEq)]
pub struct ArchComponent {
    /// Audit fields.
    pub audit: Audit,
    /// Editable fields.
    pub fields: ArchComponentFields,
}

/// Editable fields of an asset.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetFields {
    /// Unique name within the threat model.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Category.
    pub category: AssetCategory,
    /// Owner.
    pub owner: Option<String>,
    /// Confidentiality requirement.
    pub confidentiality: Option<CiaLevel>,
    /// Integrity requirement.
    pub integrity: Option<CiaLevel>,
    /// Availability requirement.
    pub availability: Option<CiaLevel>,
    /// Contains personal data.
    pub personal_data: bool,
}

/// Asset.
#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    /// Audit fields.
    pub audit: Audit,
    /// Editable fields.
    pub fields: AssetFields,
}

/// Documented authentication of a flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthState {
    /// Explicitly none.
    None,
    /// Not documented.
    Unknown,
    /// A method is documented.
    Documented,
}

/// Editable fields of a data flow.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowFields {
    /// Source component.
    pub source_id: Uuid,
    /// Target component.
    pub target_id: Uuid,
    /// Relationship.
    pub relationship: FlowRelationship,
    /// Label.
    pub label: Option<String>,
    /// Protocol.
    pub protocol: Option<String>,
    /// Data goes both ways.
    pub bidirectional: bool,
    /// `none`, `unknown` or a method.
    pub authentication: String,
    /// Encrypted in transit.
    pub encrypted: TriState,
    /// Assets carried.
    pub asset_ids: Vec<Uuid>,
}

impl FlowFields {
    /// Interprets the authentication text.
    pub fn auth_state(&self) -> AuthState {
        let a = self.authentication.trim();
        if a.is_empty() || a.eq_ignore_ascii_case("unknown") {
            AuthState::Unknown
        } else if a.eq_ignore_ascii_case("none") {
            AuthState::None
        } else {
            AuthState::Documented
        }
    }
}

/// Data flow between two components.
#[derive(Debug, Clone, PartialEq)]
pub struct Flow {
    /// Audit fields.
    pub audit: Audit,
    /// Editable fields.
    pub fields: FlowFields,
}

/// Something a threat targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThreatTarget {
    /// Component or flow.
    pub kind: TargetKind,
    /// Its id.
    pub id: Uuid,
}

/// Why a rule created a threat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleExplanation {
    /// Rule id.
    pub rule_id: String,
    /// Ruleset version.
    pub rule_version: i32,
    /// Human-readable rule name.
    pub rule_name: String,
    /// Facts that matched.
    pub facts: Vec<String>,
    /// Information that would confirm or dismiss the candidate.
    pub missing: Vec<String>,
    /// Recommended controls.
    pub recommendations: Vec<String>,
}

/// Editable fields of a threat.
#[derive(Debug, Clone, PartialEq)]
pub struct ThreatFields {
    /// Title.
    pub title: String,
    /// STRIDE category.
    pub category: StrideCategory,
    /// Threat scenario.
    pub scenario: Option<String>,
    /// Attacker objective.
    pub attacker_objective: Option<String>,
    /// Preconditions.
    pub preconditions: Option<String>,
    /// Consequences.
    pub consequences: Option<String>,
    /// Targets, at least one.
    pub targets: Vec<ThreatTarget>,
    /// Affected assets.
    pub asset_ids: Vec<Uuid>,
    /// Addressing controls.
    pub control_ids: Vec<Uuid>,
    /// Status.
    pub status: ThreatStatus,
    /// Why it was dismissed.
    pub dismiss_reason: Option<String>,
    /// Rating before controls.
    pub inherent: Option<Rating>,
    /// Rating with existing controls.
    pub current: Option<Rating>,
    /// Treatment.
    pub treatment: RiskTreatment,
    /// Risk owner.
    pub risk_owner: Option<String>,
    /// Due date.
    pub due_date: Option<NaiveDate>,
    /// Why the risk is accepted.
    pub acceptance_rationale: Option<String>,
    /// Who accepted the risk.
    pub accepted_by: Option<String>,
}

/// Threat.
#[derive(Debug, Clone, PartialEq)]
pub struct Threat {
    /// Audit fields.
    pub audit: Audit,
    /// Number of the display id.
    pub number: i32,
    /// Editable fields.
    pub fields: ThreatFields,
    /// Manual or rule.
    pub origin: ThreatOrigin,
    /// Rule explanation, for rule threats.
    pub explanation: Option<RuleExplanation>,
}

impl Threat {
    /// `THR-n`.
    pub fn display_id(&self) -> String {
        display_id(THREAT_PREFIX, self.number)
    }
}

/// Editable fields of a security control.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlFields {
    /// Name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Type.
    pub control_type: ControlType,
    /// Category.
    pub category: ControlCategory,
    /// Status.
    pub status: ControlStatus,
    /// Owner.
    pub owner: Option<String>,
    /// Acceptance criteria.
    pub acceptance_criteria: Option<String>,
    /// Components it applies to.
    pub component_ids: Vec<Uuid>,
    /// Threats it addresses.
    pub threat_ids: Vec<Uuid>,
}

/// Security control.
#[derive(Debug, Clone, PartialEq)]
pub struct Control {
    /// Audit fields.
    pub audit: Audit,
    /// Number of the display id.
    pub number: i32,
    /// Editable fields.
    pub fields: ControlFields,
    /// When it became verified.
    pub verified_at: Option<DateTime<Utc>>,
}

impl Control {
    /// `CTL-n`.
    pub fn display_id(&self) -> String {
        display_id(CONTROL_PREFIX, self.number)
    }
}

/// Step of a security test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityTestStep {
    /// What to do.
    pub action: String,
    /// What must happen.
    pub expected: String,
}

/// Editable fields of a security test.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityTestFields {
    /// Title.
    pub title: String,
    /// Objective.
    pub objective: Option<String>,
    /// Preconditions.
    pub preconditions: Vec<String>,
    /// Ordered steps.
    pub steps: Vec<SecurityTestStep>,
    /// Type.
    pub test_type: SecurityTestType,
    /// Status.
    pub status: SecurityTestStatus,
    /// Threats it verifies.
    pub threat_ids: Vec<Uuid>,
    /// Controls it verifies.
    pub control_ids: Vec<Uuid>,
}

/// Where a generated test came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateSource {
    /// Template id.
    pub template_id: String,
    /// Template version.
    pub template_version: i32,
    /// Threat it was generated for.
    pub threat_id: Uuid,
    /// Control it was generated for.
    pub control_id: Option<Uuid>,
}

impl TemplateSource {
    /// Deduplication key `<template>|<threat>|<control or ->`.
    pub fn key(&self) -> String {
        let control = self
            .control_id
            .map_or_else(|| "-".to_owned(), |c| c.to_string());
        format!("{}|{}|{}", self.template_id, self.threat_id, control)
    }
}

/// Recorded result of a security test.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityTestResult {
    /// Id.
    pub id: Uuid,
    /// Test.
    pub security_test_id: Uuid,
    /// Outcome.
    pub outcome: SecurityTestOutcome,
    /// Note.
    pub note: Option<String>,
    /// When it ran.
    pub executed_at: DateTime<Utc>,
}

/// Security test.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityTest {
    /// Audit fields.
    pub audit: Audit,
    /// Number of the display id.
    pub number: i32,
    /// Editable fields.
    pub fields: SecurityTestFields,
    /// Manual or generated.
    pub origin: SecurityTestOrigin,
    /// Template source, for generated tests.
    pub source: Option<TemplateSource>,
    /// Latest result.
    pub last_result: Option<SecurityTestResult>,
}

impl SecurityTest {
    /// `SEC-n`.
    pub fn display_id(&self) -> String {
        display_id(TEST_PREFIX, self.number)
    }
}

/// Validation failure of threat model input.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{field}: {message}")]
pub struct ThreatModelError {
    /// Offending field (camelCase, as in the API).
    pub field: &'static str,
    /// What is wrong.
    pub message: String,
}

impl ThreatModelError {
    /// Shorthand constructor.
    pub fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn enums_round_trip_through_strings_and_serde() {
        for t in [
            "web-frontend",
            "cicd-pipeline",
            "human-user",
            "relational-database",
        ] {
            let parsed = ArchComponentType::from_str(t).unwrap();
            assert_eq!(parsed.as_str(), t);
            let json = serde_json::to_string(&parsed).unwrap();
            assert_eq!(json, format!("\"{t}\""));
            assert_eq!(
                serde_json::from_str::<ArchComponentType>(&json).unwrap(),
                parsed
            );
        }
        assert_eq!(
            StrideCategory::from_str("information-disclosure").unwrap(),
            StrideCategory::InformationDisclosure
        );
        assert_eq!(SecurityTestOutcome::NotRun.as_str(), "not-run");
        assert_eq!(TriState::from_str("yes").unwrap(), TriState::Yes);
        assert!(ArchComponentType::from_str("mainframe").is_err());
    }

    #[test]
    fn rule_explanation_uses_camel_case() {
        let e = RuleExplanation {
            rule_id: "R".into(),
            rule_version: 1,
            rule_name: "n".into(),
            facts: vec![],
            missing: vec![],
            recommendations: vec![],
        };
        let v = serde_json::to_value(&e).unwrap();
        assert!(v.get("ruleId").is_some() && v.get("ruleVersion").is_some());
        assert_eq!(serde_json::from_value::<RuleExplanation>(v).unwrap(), e);
    }

    #[test]
    fn auth_state_interprets_text() {
        let mut f = FlowFields {
            source_id: Uuid::nil(),
            target_id: Uuid::nil(),
            relationship: FlowRelationship::Calls,
            label: None,
            protocol: None,
            bidirectional: false,
            authentication: String::new(),
            encrypted: TriState::Unknown,
            asset_ids: vec![],
        };
        assert_eq!(f.auth_state(), AuthState::Unknown);
        f.authentication = "None".into();
        assert_eq!(f.auth_state(), AuthState::None);
        f.authentication = "mTLS".into();
        assert_eq!(f.auth_state(), AuthState::Documented);
    }

    #[test]
    fn template_key_format() {
        let t = Uuid::from_u128(1);
        let s = TemplateSource {
            template_id: "AUTH-FLOW".into(),
            template_version: 1,
            threat_id: t,
            control_id: None,
        };
        assert_eq!(s.key(), format!("AUTH-FLOW|{t}|-"));
    }
}
