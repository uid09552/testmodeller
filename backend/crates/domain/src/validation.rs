//! Model validation (FR-012). See docs/specification/02-domain-model.md#invariants.

use std::collections::{HashMap, HashSet, VecDeque};

use uuid::Uuid;

use crate::expr::{check_action, check_guard};
use crate::graph::{ModelGraph, StateKind};

/// Severity of a validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Blocks test generation.
    Error,
    /// Informational.
    Warning,
}

impl Severity {
    /// Wire representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

/// Problem found in a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    /// Severity.
    pub severity: Severity,
    /// Stable machine-readable code.
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
    /// Affected state.
    pub state_id: Option<Uuid>,
    /// Affected transition.
    pub transition_id: Option<Uuid>,
}

impl ValidationIssue {
    fn error(code: &'static str, message: String) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message,
            state_id: None,
            transition_id: None,
        }
    }

    fn warning(code: &'static str, message: String) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(code, message)
        }
    }

    fn at_state(mut self, id: Uuid) -> Self {
        self.state_id = Some(id);
        self
    }

    fn at_transition(mut self, id: Uuid) -> Self {
        self.transition_id = Some(id);
        self
    }
}

/// Issues that make a graph structurally unusable (duplicate ids, dangling references).
/// These are rejected on save; all other issues are reported by [`validate`] only.
pub fn structural_issues(graph: &ModelGraph) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut ids = HashSet::new();
    for s in &graph.states {
        if !ids.insert(s.id) {
            issues.push(
                ValidationIssue::error("DUPLICATE_ID", format!("duplicate id {}", s.id))
                    .at_state(s.id),
            );
        }
    }
    let state_ids: HashSet<Uuid> = graph.states.iter().map(|s| s.id).collect();
    for t in &graph.transitions {
        if !ids.insert(t.id) {
            issues.push(
                ValidationIssue::error("DUPLICATE_ID", format!("duplicate id {}", t.id))
                    .at_transition(t.id),
            );
        }
        for end in [t.from, t.to] {
            if !state_ids.contains(&end) {
                issues.push(
                    ValidationIssue::error(
                        "UNKNOWN_STATE",
                        format!("transition '{}' references unknown state {end}", t.event),
                    )
                    .at_transition(t.id),
                );
            }
        }
    }
    issues
}

/// Validates a model graph; an empty result means valid.
pub fn validate(graph: &ModelGraph) -> Vec<ValidationIssue> {
    let mut issues = structural_issues(graph);

    let mut names = HashSet::new();
    for v in &graph.variables {
        if !names.insert(v.name.as_str()) {
            issues.push(ValidationIssue::error(
                "BAD_VARIABLE",
                format!("duplicate variable '{}'", v.name),
            ));
        }
        if v.initial_value().is_none() {
            issues.push(ValidationIssue::error(
                "BAD_VARIABLE",
                format!("initial value of '{}' does not match its type", v.name),
            ));
        }
    }

    let initial: Vec<_> = graph
        .states
        .iter()
        .filter(|s| s.kind == StateKind::Initial)
        .collect();
    match initial.len() {
        0 => issues.push(ValidationIssue::error(
            "NO_INITIAL_STATE",
            "model has no initial state".into(),
        )),
        1 => {}
        _ => {
            for s in &initial[1..] {
                issues.push(
                    ValidationIssue::error(
                        "MULTIPLE_INITIAL_STATES",
                        format!("state '{}' is an additional initial state", s.name),
                    )
                    .at_state(s.id),
                );
            }
        }
    }

    for t in &graph.transitions {
        if let Some(g) = t.guard.as_deref().filter(|g| !g.trim().is_empty()) {
            if let Err(e) = check_guard(g, &graph.variables) {
                issues.push(
                    ValidationIssue::error("BAD_GUARD", format!("guard of '{}': {e}", t.event))
                        .at_transition(t.id),
                );
            }
        }
        if let Some(a) = t.action.as_deref() {
            if let Err(e) = check_action(a, &graph.variables) {
                issues.push(
                    ValidationIssue::error("BAD_ACTION", format!("action of '{}': {e}", t.event))
                        .at_transition(t.id),
                );
            }
        }
    }

    if let [start] = initial[..] {
        let mut out: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for t in &graph.transitions {
            out.entry(t.from).or_default().push(t.to);
        }
        let mut seen = HashSet::from([start.id]);
        let mut queue = VecDeque::from([start.id]);
        while let Some(s) = queue.pop_front() {
            for next in out.get(&s).into_iter().flatten() {
                if seen.insert(*next) {
                    queue.push_back(*next);
                }
            }
        }
        for s in &graph.states {
            if !seen.contains(&s.id) {
                issues.push(
                    ValidationIssue::warning(
                        "UNREACHABLE_STATE",
                        format!("state '{}' is not reachable from the initial state", s.name),
                    )
                    .at_state(s.id),
                );
            }
        }
    }

    let has_outgoing: HashSet<Uuid> = graph.transitions.iter().map(|t| t.from).collect();
    for s in &graph.states {
        if s.kind != StateKind::Final && !has_outgoing.contains(&s.id) {
            issues.push(
                ValidationIssue::warning(
                    "DEAD_END",
                    format!("non-final state '{}' has no outgoing transitions", s.name),
                )
                .at_state(s.id),
            );
        }
    }

    issues
}

/// True if any issue has error severity.
pub fn has_errors(issues: &[ValidationIssue]) -> bool {
    issues.iter().any(|i| i.severity == Severity::Error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{State, Transition, Variable, VariableType};

    fn state(name: &str, kind: StateKind) -> State {
        State {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            kind,
            position: None,
        }
    }

    fn transition(from: &State, to: &State, event: &str) -> Transition {
        Transition {
            id: Uuid::new_v4(),
            from: from.id,
            to: to.id,
            event: event.into(),
            guard: None,
            action: None,
            expected: None,
        }
    }

    fn codes(issues: &[ValidationIssue]) -> Vec<&'static str> {
        issues.iter().map(|i| i.code).collect()
    }

    #[test]
    fn fr012_valid_model_has_no_issues() {
        let a = state("A", StateKind::Initial);
        let b = state("B", StateKind::Final);
        let g = ModelGraph {
            transitions: vec![transition(&a, &b, "go")],
            states: vec![a, b],
            variables: vec![],
        };
        assert!(validate(&g).is_empty());
    }

    #[test]
    fn fr012_reports_initial_reachability_and_dead_ends() {
        let a = state("A", StateKind::Initial);
        let b = state("B", StateKind::Normal);
        let c = state("C", StateKind::Initial);
        let g = ModelGraph {
            transitions: vec![transition(&a, &b, "go")],
            states: vec![a, b, c],
            variables: vec![],
        };
        let c = codes(&validate(&g));
        assert!(c.contains(&"MULTIPLE_INITIAL_STATES"));
        assert!(c.contains(&"DEAD_END"));
        assert!(!c.contains(&"UNREACHABLE_STATE"));

        let g = ModelGraph {
            states: vec![state("X", StateKind::Normal)],
            ..Default::default()
        };
        assert!(codes(&validate(&g)).contains(&"NO_INITIAL_STATE"));
    }

    #[test]
    fn fr012_reports_unreachable_state() {
        let a = state("A", StateKind::Initial);
        let b = state("B", StateKind::Final);
        let c = state("C", StateKind::Final);
        let g = ModelGraph {
            transitions: vec![transition(&a, &b, "go")],
            states: vec![a, b, c],
            variables: vec![],
        };
        assert_eq!(codes(&validate(&g)), vec!["UNREACHABLE_STATE"]);
    }

    #[test]
    fn fr012_reports_bad_expressions_and_dangling_references() {
        let a = state("A", StateKind::Initial);
        let b = state("B", StateKind::Final);
        let mut t = transition(&a, &b, "go");
        t.guard = Some("n >".into());
        t.action = Some("n = true".into());
        let mut dangling = transition(&a, &b, "bad");
        dangling.to = Uuid::new_v4();
        let g = ModelGraph {
            transitions: vec![t, dangling],
            states: vec![a, b],
            variables: vec![Variable {
                name: "n".into(),
                var_type: VariableType::Integer,
                initial: None,
            }],
        };
        let issues = validate(&g);
        let c = codes(&issues);
        assert!(c.contains(&"BAD_GUARD"));
        assert!(c.contains(&"BAD_ACTION"));
        assert!(c.contains(&"UNKNOWN_STATE"));
        assert!(has_errors(&issues));
        assert_eq!(codes(&structural_issues(&g)), vec!["UNKNOWN_STATE"]);
    }
}
