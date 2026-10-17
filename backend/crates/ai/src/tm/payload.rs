//! Typed payloads of threat model proposals. Every struct rejects unknown
//! fields, which is how AI is kept from setting statuses, treatments,
//! acceptances, evidence or thresholds: a field that is not declared here
//! cannot arrive (07-ai-integration.md, "Threat model assistant").

use serde::{Deserialize, Serialize};
use tm_domain::threat::{
    ArchComponentType, AssetCategory, CiaLevel, Classification, ControlCategory, ControlType,
    Criticality, FlowRelationship, SecurityTestStep, SecurityTestType, StrideCategory,
    ThreatTarget, TriState, ZoneKind,
};
use uuid::Uuid;

/// What a request asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TmKind {
    /// Zones, assets, components and flows from a description.
    #[serde(rename = "tm-architecture")]
    Architecture,
    /// Additional threats.
    #[serde(rename = "tm-threats")]
    Threats,
    /// Ratings of existing threats.
    #[serde(rename = "tm-ratings")]
    Ratings,
    /// Controls for threats.
    #[serde(rename = "tm-controls")]
    Controls,
    /// Tailored security tests.
    #[serde(rename = "tm-security-tests")]
    SecurityTests,
    /// Advisory findings.
    #[serde(rename = "tm-review")]
    Review,
}

impl TmKind {
    /// Wire and storage representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Architecture => "tm-architecture",
            Self::Threats => "tm-threats",
            Self::Ratings => "tm-ratings",
            Self::Controls => "tm-controls",
            Self::SecurityTests => "tm-security-tests",
            Self::Review => "tm-review",
        }
    }

    /// Parses the wire representation.
    pub fn parse(s: &str) -> Option<Self> {
        [
            Self::Architecture,
            Self::Threats,
            Self::Ratings,
            Self::Controls,
            Self::SecurityTests,
            Self::Review,
        ]
        .into_iter()
        .find(|k| k.as_str() == s)
    }

    /// Whether accepting a proposal of this kind creates or changes anything.
    pub fn is_actionable(self) -> bool {
        self != Self::Review
    }
}

/// A new zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ZonePayload {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub kind: ZoneKind,
    pub trust_level: u8,
}

/// A new asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AssetPayload {
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
}

/// A new component. `zone` and `assets` refer to zones and assets by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ComponentPayload {
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
    pub zone: Option<String>,
    #[serde(default)]
    pub assets: Vec<String>,
}

/// A new flow. `source`, `target` and `assets` refer to components and assets by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FlowPayload {
    pub source: String,
    pub target: String,
    pub relationship: FlowRelationship,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub authentication: Option<String>,
    #[serde(default)]
    pub encrypted: Option<TriState>,
    #[serde(default)]
    pub assets: Vec<String>,
    #[serde(default)]
    pub data_classification: Option<Classification>,
    #[serde(default)]
    pub entry_point: bool,
}

/// `tm-architecture`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ArchitecturePayload {
    #[serde(default)]
    pub zones: Vec<ZonePayload>,
    #[serde(default)]
    pub assets: Vec<AssetPayload>,
    #[serde(default)]
    pub components: Vec<ComponentPayload>,
    #[serde(default)]
    pub flows: Vec<FlowPayload>,
}

/// `tm-threats`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ThreatPayload {
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
    pub threat_source: Option<String>,
    pub targets: Vec<ThreatTarget>,
    #[serde(default)]
    pub asset_ids: Vec<Uuid>,
}

/// Which rating of a threat a `tm-ratings` proposal sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RatingBasis {
    Inherent,
    Current,
    Residual,
}

/// `tm-ratings`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RatingPayload {
    pub threat_id: Uuid,
    pub rating: RatingBasis,
    pub likelihood: u8,
    pub impact: u8,
}

/// `tm-controls`: a new control, or a link of an existing control to threats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ControlPayload {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "type")]
    pub control_type: Option<ControlType>,
    #[serde(default)]
    pub category: Option<ControlCategory>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub acceptance_criteria: Option<String>,
    #[serde(default)]
    pub component_ids: Vec<Uuid>,
    pub threat_ids: Vec<Uuid>,
    /// Link this existing control instead of creating one.
    #[serde(default)]
    pub link_control_id: Option<Uuid>,
}

/// `tm-security-tests`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TestPayload {
    pub title: String,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(rename = "type")]
    pub test_type: SecurityTestType,
    #[serde(default)]
    pub preconditions: Vec<String>,
    #[serde(default)]
    pub steps: Vec<SecurityTestStep>,
    pub threat_ids: Vec<Uuid>,
    #[serde(default)]
    pub control_ids: Vec<Uuid>,
}

/// One finding of `tm-review`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Finding {
    pub text: String,
    #[serde(default)]
    pub entity_ids: Vec<Uuid>,
}

/// `tm-review`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReviewPayload {
    pub findings: Vec<Finding>,
}

/// A validated payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmPayload {
    Architecture(ArchitecturePayload),
    Threat(ThreatPayload),
    Rating(RatingPayload),
    Control(ControlPayload),
    Test(TestPayload),
    Review(ReviewPayload),
}

impl TmPayload {
    /// The JSON stored with the proposal.
    pub fn to_json(&self) -> serde_json::Value {
        let v = match self {
            Self::Architecture(p) => serde_json::to_value(p),
            Self::Threat(p) => serde_json::to_value(p),
            Self::Rating(p) => serde_json::to_value(p),
            Self::Control(p) => serde_json::to_value(p),
            Self::Test(p) => serde_json::to_value(p),
            Self::Review(p) => serde_json::to_value(p),
        };
        v.unwrap_or(serde_json::Value::Null)
    }

    /// Parses stored or edited JSON of a kind, rejecting unknown fields.
    pub fn from_json(kind: TmKind, v: serde_json::Value) -> Result<Self, serde_json::Error> {
        Ok(match kind {
            TmKind::Architecture => Self::Architecture(serde_json::from_value(v)?),
            TmKind::Threats => Self::Threat(serde_json::from_value(v)?),
            TmKind::Ratings => Self::Rating(serde_json::from_value(v)?),
            TmKind::Controls => Self::Control(serde_json::from_value(v)?),
            TmKind::SecurityTests => Self::Test(serde_json::from_value(v)?),
            TmKind::Review => Self::Review(serde_json::from_value(v)?),
        })
    }
}
