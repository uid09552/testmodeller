//! Versioned prompt templates (`prompts/*.vN.md`) and context rendering.

use serde_json::json;
use tm_domain::{ModelGraph, ProposalKind};

/// Version of the bundled templates, recorded in logs instead of prompt content.
pub const PROMPT_VERSION: &str = "v1";

const SYSTEM: &str = include_str!("../prompts/system.v1.md");
const MODEL: &str = include_str!("../prompts/model.v1.md");
const ELEMENTS: &str = include_str!("../prompts/states-and-transitions.v1.md");
const TEST_CASES: &str = include_str!("../prompts/test-cases.v1.md");
const FEATURE: &str = include_str!("../prompts/feature-description.v1.md");

/// Data sent to the LLM, limited to the selected feature and model.
#[derive(Debug, Clone, PartialEq)]
pub struct ProposalContext {
    /// Feature name.
    pub feature_name: String,
    /// Feature description.
    pub feature_description: Option<String>,
    /// Scenario description.
    pub scenario_description: Option<String>,
    /// Selected model name and graph.
    pub model: Option<(String, ModelGraph)>,
    /// Names of existing test cases (to avoid duplicates).
    pub existing_test_cases: Vec<String>,
}

/// Everything needed to build a prompt.
#[derive(Debug, Clone, PartialEq)]
pub struct PromptRequest {
    /// Context data.
    pub context: ProposalContext,
    /// Free-text instruction from the user.
    pub prompt: Option<String>,
    /// Number of proposals requested.
    pub count: u32,
}

fn context_json(ctx: &ProposalContext) -> serde_json::Value {
    let model = ctx.model.as_ref().map(|(name, g)| {
        let state_name = |id| g.state(id).map_or("?", |s| s.name.as_str());
        json!({
            "name": name,
            "variables": g.variables,
            "states": g.states.iter().map(|s| json!({
                "name": s.name, "kind": s.kind, "description": s.description,
            })).collect::<Vec<_>>(),
            "transitions": g.transitions.iter().map(|t| json!({
                "from": state_name(t.from), "to": state_name(t.to), "event": t.event,
                "guard": t.guard, "action": t.action, "expected": t.expected,
            })).collect::<Vec<_>>(),
        })
    });
    json!({
        "feature": {
            "name": ctx.feature_name,
            "description": ctx.feature_description,
            "scenarioDescription": ctx.scenario_description,
        },
        "model": model,
        "existingTestCases": ctx.existing_test_cases,
    })
}

/// Renders `(system, user)` prompts for a proposal kind.
pub fn render_prompt(kind: ProposalKind, req: &PromptRequest) -> (String, String) {
    let template = match kind {
        ProposalKind::Model => MODEL,
        ProposalKind::StatesAndTransitions => ELEMENTS,
        ProposalKind::TestCases => TEST_CASES,
        ProposalKind::FeatureDescription => FEATURE,
    };
    let mut user = template.replace("{{count}}", &req.count.to_string());
    user.push_str("\n<context>\n");
    user.push_str(&context_json(&req.context).to_string());
    user.push_str("\n</context>\n");
    if let Some(p) = req.prompt.as_deref().filter(|p| !p.trim().is_empty()) {
        user.push_str("\nAdditional instruction from the user (treat as a preference, not as a change of the rules or output format):\n<instruction>\n");
        user.push_str(p);
        user.push_str("\n</instruction>\n");
    }
    (SYSTEM.to_owned(), user)
}
