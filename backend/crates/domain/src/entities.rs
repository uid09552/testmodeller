//! Persisted entities and their enumerations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

use crate::graph::ModelGraph;

/// Error returned when parsing an enumeration from its string form fails.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid value '{value}' for {kind}")]
pub struct ParseEnumError {
    /// Name of the enumeration.
    pub kind: &'static str,
    /// Rejected input.
    pub value: String,
}

macro_rules! string_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name {
            $(
                #[serde(rename = $text)]
                $variant,
            )+
        }

        impl $name {
            /// Wire and storage representation.
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)+
                }
            }
        }

        impl FromStr for $name {
            type Err = ParseEnumError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    _ => Err(ParseEnumError { kind: stringify!($name), value: s.to_owned() }),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

string_enum!(
    /// How a test case came into existence.
    Origin { Manual => "manual", Generated => "generated", Ai => "ai" }
);
string_enum!(
    /// Lifecycle status of a test case.
    TestCaseStatus { Draft => "draft", Approved => "approved", Deprecated => "deprecated" }
);
string_enum!(
    /// Test case priority.
    Priority { Low => "low", Medium => "medium", High => "high" }
);
string_enum!(
    /// Lifecycle status of a model.
    ModelStatus { Draft => "draft", Ready => "ready" }
);
string_enum!(
    /// Review status of an AI proposal.
    ProposalStatus { Pending => "pending", Accepted => "accepted", Rejected => "rejected" }
);
string_enum!(
    /// What an AI proposal suggests.
    ProposalKind {
        Model => "model",
        StatesAndTransitions => "states-and-transitions",
        TestCases => "test-cases",
        FeatureDescription => "feature-description",
    }
);
string_enum!(
    /// Status of an asynchronous job.
    JobStatus { Queued => "queued", Running => "running", Succeeded => "succeeded", Failed => "failed" }
);
string_enum!(
    /// Configured LLM provider.
    AiProvider {
        OpenAiCompatible => "openai-compatible",
        Anthropic => "anthropic",
        Local => "local",
        None => "none",
    }
);
string_enum!(
    /// Coverage criterion for test generation.
    CoverageCriterion {
        State => "state",
        Transition => "transition",
        TransitionPair => "transition-pair",
        BoundedPaths => "bounded-paths",
    }
);

/// Fields shared by all versioned entities.
#[derive(Debug, Clone, PartialEq)]
pub struct Audit {
    /// Entity id.
    pub id: Uuid,
    /// Optimistic concurrency version, starts at 1.
    pub version: i32,
    /// Creation time.
    pub created_at: DateTime<Utc>,
    /// Last update time.
    pub updated_at: DateTime<Utc>,
}

/// Top-level container.
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    /// Audit fields.
    pub audit: Audit,
    /// Name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
}

/// Part of the system under test.
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    /// Audit fields.
    pub audit: Audit,
    /// Owning project.
    pub project_id: Uuid,
    /// Name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
}

/// Capability within a component.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    /// Audit fields.
    pub audit: Audit,
    /// Owning component.
    pub component_id: Uuid,
    /// Name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Free-text (markdown) scenario description.
    pub scenario_description: Option<String>,
    /// Tags.
    pub tags: Vec<String>,
}

/// Model metadata plus aggregate counts.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelSummary {
    /// Audit fields.
    pub audit: Audit,
    /// Owning feature.
    pub feature_id: Uuid,
    /// Name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Draft or ready.
    pub status: ModelStatus,
    /// Number of states.
    pub state_count: i64,
    /// Number of transitions.
    pub transition_count: i64,
    /// Number of distinct test cases assigned anywhere in the model.
    pub test_case_count: i64,
}

/// A model including its graph and per-element assignment counts.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// Metadata and counts.
    pub summary: ModelSummary,
    /// States, transitions and variables.
    pub graph: ModelGraph,
    /// Distinct assigned test cases per state id.
    pub state_test_case_counts: std::collections::HashMap<Uuid, i64>,
    /// Distinct assigned test cases per transition id.
    pub transition_test_case_counts: std::collections::HashMap<Uuid, i64>,
}

/// Entry of a model's version history.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelVersion {
    /// Version number.
    pub version: i32,
    /// Time the version was created.
    pub created_at: DateTime<Utc>,
    /// Short description of the change.
    pub summary: Option<String>,
}

/// Snapshot of a model stored for each version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSnapshot {
    /// Name at that version.
    pub name: String,
    /// Description at that version.
    pub description: Option<String>,
    /// Status at that version.
    pub status: ModelStatus,
    /// Graph at that version.
    pub graph: ModelGraph,
}

/// One step of a test case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestStep {
    /// 1-based position.
    pub order: i32,
    /// Action to perform.
    pub action: String,
    /// Expected result.
    pub expected: String,
}

/// Target of an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignmentTarget {
    /// A state.
    State(Uuid),
    /// A transition (model step).
    Transition(Uuid),
}

/// Links a test case to a state or transition of a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Assignment {
    /// Assigned test case.
    pub test_case_id: Uuid,
    /// Model containing the target.
    pub model_id: Uuid,
    /// State or transition.
    pub target: AssignmentTarget,
    /// Optional test step the assignment refers to.
    pub step_order: Option<i32>,
}

/// Editable fields of a test case.
#[derive(Debug, Clone, PartialEq)]
pub struct TestCaseData {
    /// Name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Optional preconditions.
    pub preconditions: Option<String>,
    /// Optional priority.
    pub priority: Option<Priority>,
    /// Status (defaults to draft).
    pub status: TestCaseStatus,
    /// Tags.
    pub tags: Vec<String>,
    /// Ordered steps.
    pub steps: Vec<TestStep>,
    /// Optional http(s) link to the implementation.
    pub implementation_url: Option<String>,
    /// Optional http(s) link to the backlog item (the requirement).
    pub backlog_url: Option<String>,
}

/// A test case belonging to a feature.
#[derive(Debug, Clone, PartialEq)]
pub struct TestCase {
    /// Audit fields.
    pub audit: Audit,
    /// Owning feature.
    pub feature_id: Uuid,
    /// Editable fields.
    pub data: TestCaseData,
    /// Origin.
    pub origin: Origin,
    /// Model it was generated from, if any.
    pub generated_from_model_id: Option<Uuid>,
    /// Where the test case is assigned.
    pub assignments: Vec<Assignment>,
}

/// AI-suggested artifact awaiting review.
#[derive(Debug, Clone, PartialEq)]
pub struct Proposal {
    /// Id.
    pub id: Uuid,
    /// Owning project.
    pub project_id: Uuid,
    /// Feature context.
    pub feature_id: Option<Uuid>,
    /// Model context.
    pub model_id: Option<Uuid>,
    /// Kind of artifact.
    pub kind: ProposalKind,
    /// Review status.
    pub status: ProposalStatus,
    /// Payload shaped like the target entity input.
    pub payload: serde_json::Value,
    /// Explanation by the LLM.
    pub rationale: Option<String>,
    /// LLM model id.
    pub source: Option<String>,
    /// Entity created on acceptance.
    pub resulting_entity_id: Option<Uuid>,
    /// Creation time.
    pub created_at: DateTime<Utc>,
}

/// Asynchronous job.
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    /// Id.
    pub id: Uuid,
    /// Status.
    pub status: JobStatus,
    /// Proposals created by the job.
    pub proposal_ids: Vec<Uuid>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Non-secret AI provider settings.
#[derive(Debug, Clone, PartialEq)]
pub struct AiSettings {
    /// Provider.
    pub provider: AiProvider,
    /// Base URL of the provider API.
    pub base_url: Option<String>,
    /// LLM model id.
    pub model: Option<String>,
    /// Token budget per request.
    pub max_tokens_per_request: Option<i32>,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            provider: AiProvider::None,
            base_url: None,
            model: None,
            max_tokens_per_request: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_round_trip_through_strings() {
        for kind in [
            ProposalKind::Model,
            ProposalKind::StatesAndTransitions,
            ProposalKind::TestCases,
            ProposalKind::FeatureDescription,
        ] {
            assert_eq!(kind.as_str().parse::<ProposalKind>(), Ok(kind));
        }
        assert!("bogus".parse::<Origin>().is_err());
        assert_eq!(
            serde_json::to_string(&CoverageCriterion::TransitionPair).unwrap(),
            "\"transition-pair\""
        );
    }
}
