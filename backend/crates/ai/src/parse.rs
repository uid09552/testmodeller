//! Parsing and validation of LLM output into domain types.

use std::collections::HashMap;

use serde::Deserialize;
use tm_domain::validation::{structural_issues, validate, Severity};
use tm_domain::{
    ModelGraph, Priority, ProposalKind, State, StateKind, TestCaseData, TestCaseStatus, TestStep,
    Transition, Variable,
};
use uuid::Uuid;

use crate::prompts::ProposalContext;
use crate::AiError;

/// Validated proposal content.
#[derive(Debug, Clone, PartialEq)]
pub enum ProposalContent {
    /// A new model.
    Model {
        /// Name.
        name: String,
        /// Description.
        description: Option<String>,
        /// Graph with fresh ids.
        graph: ModelGraph,
    },
    /// New states and transitions for the context model; transitions may reference existing states.
    Elements {
        /// New states.
        states: Vec<State>,
        /// New transitions.
        transitions: Vec<Transition>,
    },
    /// A test case.
    TestCase(TestCaseData),
    /// A scenario description.
    FeatureDescription(String),
}

/// One proposal produced by the LLM.
#[derive(Debug, Clone, PartialEq)]
pub struct ProposalDraft {
    /// Content.
    pub content: ProposalContent,
    /// Explanation.
    pub rationale: Option<String>,
}

#[derive(Deserialize)]
struct Envelope {
    proposals: Vec<RawProposal>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawProposal {
    rationale: Option<String>,
    model: Option<RawModel>,
    #[serde(default)]
    states: Vec<RawState>,
    #[serde(default)]
    transitions: Vec<RawTransition>,
    test_case: Option<RawTestCase>,
    scenario_description: Option<String>,
}

#[derive(Deserialize)]
struct RawModel {
    name: String,
    description: Option<String>,
    #[serde(default)]
    variables: Vec<Variable>,
    states: Vec<RawState>,
    #[serde(default)]
    transitions: Vec<RawTransition>,
}

#[derive(Deserialize)]
struct RawState {
    name: String,
    kind: StateKind,
    description: Option<String>,
}

#[derive(Deserialize)]
struct RawTransition {
    from: String,
    to: String,
    event: String,
    guard: Option<String>,
    action: Option<String>,
    expected: Option<String>,
}

#[derive(Deserialize)]
struct RawTestCase {
    name: String,
    description: Option<String>,
    preconditions: Option<String>,
    priority: Option<Priority>,
    steps: Vec<RawStep>,
}

#[derive(Deserialize)]
struct RawStep {
    action: String,
    expected: String,
}

fn invalid(msg: impl Into<String>) -> AiError {
    AiError::InvalidOutput(msg.into())
}

/// Strips an optional markdown code fence around the JSON.
fn strip_fence(s: &str) -> &str {
    let t = s.trim();
    let Some(rest) = t.strip_prefix("```") else {
        return t;
    };
    let rest = rest.split_once('\n').map_or("", |(_, body)| body);
    rest.trim_end().trim_end_matches("```").trim()
}

fn non_empty(s: &str, what: &str) -> Result<(), AiError> {
    if s.trim().is_empty() {
        return Err(invalid(format!("{what} must not be empty")));
    }
    Ok(())
}

fn build_states(raw: Vec<RawState>) -> Result<Vec<State>, AiError> {
    raw.into_iter()
        .map(|s| {
            non_empty(&s.name, "state name")?;
            Ok(State {
                id: Uuid::new_v4(),
                name: s.name,
                description: s.description,
                kind: s.kind,
                position: None,
            })
        })
        .collect()
}

fn build_transitions(
    raw: Vec<RawTransition>,
    names: &HashMap<String, Uuid>,
) -> Result<Vec<Transition>, AiError> {
    let resolve = |n: &str| {
        names
            .get(n)
            .copied()
            .ok_or_else(|| invalid(format!("transition references unknown state '{n}'")))
    };
    raw.into_iter()
        .map(|t| {
            non_empty(&t.event, "transition event")?;
            Ok(Transition {
                id: Uuid::new_v4(),
                from: resolve(&t.from)?,
                to: resolve(&t.to)?,
                event: t.event,
                guard: t.guard.filter(|g| !g.trim().is_empty()),
                action: t.action.filter(|a| !a.trim().is_empty()),
                expected: t.expected,
            })
        })
        .collect()
}

fn name_map<'a>(states: impl Iterator<Item = &'a State>) -> Result<HashMap<String, Uuid>, AiError> {
    let mut map = HashMap::new();
    for s in states {
        if map.insert(s.name.clone(), s.id).is_some() {
            return Err(invalid(format!("duplicate state name '{}'", s.name)));
        }
    }
    Ok(map)
}

/// Rejects graphs with structural or expression errors. Reachability warnings are allowed.
fn check_graph(graph: &ModelGraph, require_initial: bool) -> Result<(), AiError> {
    let issues = if require_initial {
        validate(graph)
    } else {
        let mut issues = validate(graph);
        issues.retain(|i| i.code != "NO_INITIAL_STATE");
        issues
    };
    let errors: Vec<String> = issues
        .iter()
        .chain(structural_issues(graph).iter())
        .filter(|i| i.severity == Severity::Error)
        .map(|i| i.message.clone())
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(invalid(errors.join("; ")))
    }
}

fn parse_one(
    kind: ProposalKind,
    raw: RawProposal,
    ctx: &ProposalContext,
) -> Result<ProposalContent, AiError> {
    match kind {
        ProposalKind::Model => {
            let m = raw.model.ok_or_else(|| invalid("missing 'model'"))?;
            non_empty(&m.name, "model name")?;
            let states = build_states(m.states)?;
            let names = name_map(states.iter())?;
            let transitions = build_transitions(m.transitions, &names)?;
            let graph = ModelGraph {
                variables: m.variables,
                states,
                transitions,
            };
            check_graph(&graph, true)?;
            Ok(ProposalContent::Model {
                name: m.name,
                description: m.description,
                graph,
            })
        }
        ProposalKind::StatesAndTransitions => {
            let existing = ctx
                .model
                .as_ref()
                .map(|(_, g)| g.clone())
                .unwrap_or_default();
            let states = build_states(raw.states)?;
            if states.iter().any(|s| s.kind == StateKind::Initial) {
                return Err(invalid("new states must not be initial"));
            }
            let names = name_map(existing.states.iter().chain(states.iter()))?;
            let transitions = build_transitions(raw.transitions, &names)?;
            if states.is_empty() && transitions.is_empty() {
                return Err(invalid("proposal adds no states or transitions"));
            }
            let mut merged = existing;
            merged.states.extend(states.iter().cloned());
            merged.transitions.extend(transitions.iter().cloned());
            check_graph(&merged, false)?;
            Ok(ProposalContent::Elements {
                states,
                transitions,
            })
        }
        ProposalKind::TestCases => {
            let tc = raw.test_case.ok_or_else(|| invalid("missing 'testCase'"))?;
            non_empty(&tc.name, "test case name")?;
            if tc.steps.is_empty() {
                return Err(invalid("test case has no steps"));
            }
            Ok(ProposalContent::TestCase(TestCaseData {
                name: tc.name.chars().take(300).collect(),
                description: tc.description,
                preconditions: tc.preconditions,
                priority: tc.priority,
                status: TestCaseStatus::Draft,
                tags: Vec::new(),
                steps: tc
                    .steps
                    .into_iter()
                    .enumerate()
                    .map(|(i, s)| TestStep {
                        order: i as i32 + 1,
                        action: s.action,
                        expected: s.expected,
                    })
                    .collect(),
                implementation_url: None,
                backlog_url: None,
            }))
        }
        ProposalKind::FeatureDescription => {
            let text = raw
                .scenario_description
                .ok_or_else(|| invalid("missing 'scenarioDescription'"))?;
            non_empty(&text, "scenarioDescription")?;
            Ok(ProposalContent::FeatureDescription(text))
        }
    }
}

/// Parses an LLM answer into validated proposals. Any invalid proposal rejects the whole answer.
pub fn parse_proposals(
    kind: ProposalKind,
    answer: &str,
    ctx: &ProposalContext,
) -> Result<Vec<ProposalDraft>, AiError> {
    let envelope: Envelope = serde_json::from_str(strip_fence(answer))
        .map_err(|e| invalid(format!("not the requested JSON shape: {e}")))?;
    if envelope.proposals.is_empty() {
        return Err(invalid("no proposals returned"));
    }
    envelope
        .proposals
        .into_iter()
        .enumerate()
        .map(|(i, raw)| {
            let rationale = raw.rationale.clone();
            parse_one(kind, raw, ctx)
                .map(|content| ProposalDraft { content, rationale })
                .map_err(|e| invalid(format!("proposal {}: {e}", i + 1)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(model: Option<ModelGraph>) -> ProposalContext {
        ProposalContext {
            feature_name: "Login".into(),
            feature_description: None,
            scenario_description: None,
            model: model.map(|g| ("Login".into(), g)),
            existing_test_cases: vec![],
        }
    }

    #[test]
    fn fr030_parses_model_with_name_references() {
        let answer = r#"```json
        {"proposals":[{"rationale":"basic","model":{"name":"Login","states":[
          {"name":"Out","kind":"initial"},{"name":"In","kind":"final"}],
          "transitions":[{"from":"Out","to":"In","event":"login","expected":"Dashboard shown"}]}}]}
        ```"#;
        let drafts = parse_proposals(ProposalKind::Model, answer, &ctx(None)).unwrap();
        let ProposalContent::Model { graph, .. } = &drafts[0].content else {
            panic!("expected model");
        };
        assert_eq!(graph.transitions[0].from, graph.states[0].id);
        assert_eq!(drafts[0].rationale.as_deref(), Some("basic"));
    }

    #[test]
    fn rejects_unknown_states_and_bad_guards() {
        let answer = r#"{"proposals":[{"model":{"name":"M","states":[{"name":"A","kind":"initial"}],
          "transitions":[{"from":"A","to":"Z","event":"go"}]}}]}"#;
        assert!(parse_proposals(ProposalKind::Model, answer, &ctx(None)).is_err());
        let answer = r#"{"proposals":[{"model":{"name":"M","states":[{"name":"A","kind":"initial"}],
          "transitions":[{"from":"A","to":"A","event":"go","guard":"x >"}]}}]}"#;
        assert!(parse_proposals(ProposalKind::Model, answer, &ctx(None)).is_err());
    }

    #[test]
    fn fr031_elements_reference_existing_states() {
        let a = State {
            id: Uuid::new_v4(),
            name: "Start".into(),
            description: None,
            kind: StateKind::Initial,
            position: None,
        };
        let graph = ModelGraph {
            states: vec![a.clone()],
            ..Default::default()
        };
        let answer = r#"{"proposals":[{"states":[{"name":"Error","kind":"final"}],
          "transitions":[{"from":"Start","to":"Error","event":"timeout"}]}]}"#;
        let drafts = parse_proposals(
            ProposalKind::StatesAndTransitions,
            answer,
            &ctx(Some(graph)),
        )
        .unwrap();
        let ProposalContent::Elements { transitions, .. } = &drafts[0].content else {
            panic!("expected elements");
        };
        assert_eq!(transitions[0].from, a.id);
    }

    #[test]
    fn fr032_parses_test_cases() {
        let answer = r#"{"proposals":[{"testCase":{"name":"Wrong password","priority":"high",
          "steps":[{"action":"enter wrong password","expected":"error shown"}]}}]}"#;
        let drafts = parse_proposals(ProposalKind::TestCases, answer, &ctx(None)).unwrap();
        let ProposalContent::TestCase(tc) = &drafts[0].content else {
            panic!("expected test case");
        };
        assert_eq!(tc.steps[0].order, 1);
        assert_eq!(tc.priority, Some(Priority::High));
    }
}
