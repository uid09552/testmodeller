//! Stale-test detection: does a generated test case's path still fit its model?
//! See docs/specification/04-test-generation.md#stale-tests.
//!
//! The path is rebuilt from the test case's assignments (the transition of step
//! `n` is assigned with `stepOrder = n`) and replayed from the initial state.
//! The model is deterministic — variables start at their initial values and
//! actions are plain assignments — so replaying the path decides guard
//! satisfiability exactly; there is no search and no cap.

use std::collections::HashMap;

use tm_domain::expr;
use tm_domain::{AssignmentTarget, ModelGraph, Transition};
use uuid::Uuid;

/// Why a test case is stale. The wire form is the `SCREAMING_SNAKE` code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleCode {
    /// A step has no transition assigned any more (it was deleted).
    StepUnassigned,
    /// A step's transition does not start where the previous one ended.
    StepsDisconnected,
    /// The first step does not start at the initial state.
    NotFromInitial,
    /// A guard on the path is false when the path is replayed.
    GuardUnsatisfiable,
}

impl StaleCode {
    /// Stable code used by the API and the UI.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StepUnassigned => "STEP_UNASSIGNED",
            Self::StepsDisconnected => "STEPS_DISCONNECTED",
            Self::NotFromInitial => "NOT_FROM_INITIAL",
            Self::GuardUnsatisfiable => "GUARD_UNSATISFIABLE",
        }
    }
}

/// One reason a test case is stale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleReason {
    /// Machine-readable reason.
    pub code: StaleCode,
    /// 1-based step it refers to, if one.
    pub step_order: Option<i32>,
    /// Human-readable explanation.
    pub message: String,
}

fn reason(code: StaleCode, step_order: Option<i32>, message: String) -> StaleReason {
    StaleReason {
        code,
        step_order,
        message,
    }
}

/// Checks a generated test case of `steps` steps against `graph`, given its
/// assignments in this model (target and optional 1-based step). Empty when it
/// still fits. Pure: nothing is changed.
pub fn check_path(
    graph: &ModelGraph,
    steps: usize,
    assignments: &[(AssignmentTarget, Option<i32>)],
) -> Vec<StaleReason> {
    let by_id: HashMap<Uuid, &Transition> = graph.transitions.iter().map(|t| (t.id, t)).collect();
    let mut assigned: HashMap<i32, &Transition> = HashMap::new();
    let mut used: Vec<&Transition> = Vec::new();
    for (target, order) in assignments {
        if let (AssignmentTarget::Transition(id), Some(order)) = (target, order) {
            if let Some(t) = by_id.get(id) {
                assigned.entry(*order).or_insert(t);
                used.push(t);
            }
        }
    }
    let initial = graph.initial_state().map(|s| s.id);

    // Rebuild the sequence. Test cases generated before every step was assigned
    // lack the assignment of a repeated transition; such a step is the one
    // transition of this test case that chains with its neighbours.
    let mut path: Vec<Option<&Transition>> = (1..=steps as i32)
        .map(|k| assigned.get(&k).copied())
        .collect();
    for k in 0..path.len() {
        if path[k].is_some() {
            continue;
        }
        let from = if k == 0 {
            initial
        } else {
            path[k - 1].map(|t| t.to)
        };
        let to = path.get(k + 1).copied().flatten().map(|t| t.from);
        let mut fits = used
            .iter()
            .filter(|t| Some(t.from) == from && to.is_none_or(|to| t.to == to));
        if let (Some(t), None) = (fits.next(), fits.next()) {
            path[k] = Some(t);
        }
    }

    let mut reasons = Vec::new();
    for (k, t) in path.iter().enumerate() {
        if t.is_none() {
            let n = k as i32 + 1;
            reasons.push(reason(
                StaleCode::StepUnassigned,
                Some(n),
                format!("step {n} lost its transition; it was deleted from the model"),
            ));
        }
    }
    for k in 1..path.len() {
        if let (Some(a), Some(b)) = (path[k - 1], path[k]) {
            if a.to != b.from {
                let n = k as i32 + 1;
                reasons.push(reason(
                    StaleCode::StepsDisconnected,
                    Some(n),
                    format!("step {n} no longer starts where step {k} ends"),
                ));
            }
        }
    }
    match (initial, path.first().copied().flatten()) {
        (None, _) if steps > 0 => reasons.push(reason(
            StaleCode::NotFromInitial,
            None,
            "the model has no initial state".into(),
        )),
        (Some(init), Some(first)) if first.from != init => reasons.push(reason(
            StaleCode::NotFromInitial,
            Some(1),
            "the path does not start at the initial state".into(),
        )),
        _ => {}
    }
    if !reasons.is_empty() {
        return reasons;
    }

    // Complete and connected: replay it.
    let mut env = expr::initial_env(&graph.variables);
    for (k, t) in path.iter().flatten().enumerate() {
        let n = k as i32 + 1;
        if let Some(g) = t.guard.as_deref().filter(|g| !g.trim().is_empty()) {
            let holds =
                expr::check_guard(g, &graph.variables).and_then(|e| expr::eval_guard(&e, &env));
            match holds {
                Ok(true) => {}
                Ok(false) => {
                    return vec![reason(
                        StaleCode::GuardUnsatisfiable,
                        Some(n),
                        format!("guard `{g}` of step {n} is false on this path"),
                    )]
                }
                Err(e) => {
                    return vec![reason(
                        StaleCode::GuardUnsatisfiable,
                        Some(n),
                        format!("guard `{g}` of step {n} cannot be evaluated: {e}"),
                    )]
                }
            }
        }
        if let Some(a) = t.action.as_deref().filter(|a| !a.trim().is_empty()) {
            let next = expr::check_action(a, &graph.variables)
                .and_then(|assigns| expr::apply_action(&assigns, &env));
            match next {
                Ok(e) => env = e,
                Err(e) => {
                    return vec![reason(
                        StaleCode::GuardUnsatisfiable,
                        Some(n),
                        format!("action `{a}` of step {n} cannot be applied: {e}"),
                    )]
                }
            }
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, path_to_test_case, GenerationOptions};
    use tm_domain::{CoverageCriterion, State, StateKind, Variable, VariableType};

    fn state(name: &str, kind: StateKind) -> State {
        State {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            kind,
            position: None,
        }
    }

    fn tr(from: &State, to: &State, event: &str) -> Transition {
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

    /// Start -> Out --login--> In -> Done, with a guarded retry loop on Out.
    fn model() -> ModelGraph {
        let start = state("Start", StateKind::Initial);
        let out = state("Out", StateKind::Normal);
        let inn = state("In", StateKind::Normal);
        let done = state("Done", StateKind::Final);
        let mut retry = tr(&out, &out, "wrong password");
        retry.guard = Some("attempts < 2".into());
        retry.action = Some("attempts = attempts + 1".into());
        let mut login = tr(&out, &inn, "login");
        login.guard = Some("attempts <= 2".into());
        ModelGraph {
            variables: vec![Variable {
                name: "attempts".into(),
                var_type: VariableType::Integer,
                initial: None,
            }],
            transitions: vec![
                tr(&start, &out, "open"),
                retry,
                login,
                tr(&inn, &done, "close"),
            ],
            states: vec![start, out, inn, done],
        }
    }

    /// Assignments the generator gives the path through these transitions.
    fn assigned(g: &ModelGraph, idx: &[usize]) -> Vec<(AssignmentTarget, Option<i32>)> {
        let path = crate::GeneratedPath {
            transitions: idx.iter().map(|&i| g.transitions[i].id).collect(),
            states: std::iter::once(g.states[0].id)
                .chain(idx.iter().map(|&i| g.transitions[i].to))
                .collect(),
        };
        path_to_test_case(g, "M", CoverageCriterion::Transition, 0, &path).1
    }

    fn codes(r: &[StaleReason]) -> Vec<(&'static str, Option<i32>)> {
        r.iter().map(|r| (r.code.as_str(), r.step_order)).collect()
    }

    #[test]
    fn a_healthy_path_is_not_stale() {
        let g = model();
        let a = assigned(&g, &[0, 1, 1, 2, 3]);
        assert!(check_path(&g, 5, &a).is_empty());
    }

    #[test]
    fn every_generated_test_fits_its_unchanged_model() {
        let g = model();
        let opts = GenerationOptions {
            criterion: CoverageCriterion::TransitionPair,
            seed: 7,
            max_path_length: None,
        };
        for path in generate(&g, opts).unwrap().paths {
            let (tc, a) = path_to_test_case(&g, "M", CoverageCriterion::TransitionPair, 0, &path);
            assert!(check_path(&g, tc.steps.len(), &a).is_empty());
        }
    }

    #[test]
    fn deleted_transition_leaves_a_step_unassigned() {
        let mut g = model();
        let a = assigned(&g, &[0, 2, 3]);
        let login = g.transitions[2].id;
        g.transitions.retain(|t| t.id != login);
        // The assignment cascades away with the transition.
        let a: Vec<_> = a
            .into_iter()
            .filter(|(t, _)| *t != AssignmentTarget::Transition(login))
            .collect();
        assert_eq!(
            codes(&check_path(&g, 3, &a)),
            [("STEP_UNASSIGNED", Some(2))]
        );
    }

    #[test]
    fn rewired_transition_disconnects_the_steps() {
        let mut g = model();
        let a = assigned(&g, &[0, 2, 3]);
        g.transitions[2].to = g.states[3].id; // login now goes straight to Done
        assert_eq!(
            codes(&check_path(&g, 3, &a)),
            [("STEPS_DISCONNECTED", Some(3))]
        );
    }

    #[test]
    fn moved_initial_state_is_reported() {
        let mut g = model();
        let a = assigned(&g, &[0, 2, 3]);
        g.states[0].kind = StateKind::Normal;
        g.states[1].kind = StateKind::Initial;
        assert_eq!(
            codes(&check_path(&g, 3, &a)),
            [("NOT_FROM_INITIAL", Some(1))]
        );
    }

    #[test]
    fn unsatisfiable_guard_is_reported_at_its_step() {
        let mut g = model();
        let a = assigned(&g, &[0, 1, 1, 2, 3]);
        g.transitions[2].guard = Some("attempts == 0".into());
        let r = check_path(&g, 5, &a);
        assert_eq!(codes(&r), [("GUARD_UNSATISFIABLE", Some(4))]);
        assert!(r[0].message.contains("attempts == 0"));
    }

    #[test]
    fn a_loop_assigned_once_is_still_rebuilt() {
        // Before every step was assigned, the repeated retry kept only step 2.
        let g = model();
        let retry = g.transitions[1].id;
        let a: Vec<_> = assigned(&g, &[0, 1, 1, 2, 3])
            .into_iter()
            .filter(|(t, o)| !(*t == AssignmentTarget::Transition(retry) && *o == Some(3)))
            .collect();
        assert!(check_path(&g, 5, &a).is_empty());
    }

    #[test]
    fn assignments_are_not_needed_for_an_empty_path() {
        let g = model();
        assert!(check_path(&g, 0, &[]).is_empty());
    }
}
