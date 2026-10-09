//! One step of an interactive simulation (see docs/specification/04-test-generation.md#simulation).
//!
//! Uses the generator's own `Machine::step`, so a transition is enabled in a
//! simulation exactly when generation could take it.

use tm_domain::expr::{self, Env};
use tm_domain::validation::{has_errors, validate};
use tm_domain::{ModelGraph, StateKind, ValidationIssue};
use uuid::Uuid;

use crate::{Machine, Step};

/// An outgoing transition of the current state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimTransition {
    /// Transition id.
    pub transition_id: Uuid,
    /// Its guard holds and its action can be applied.
    pub enabled: bool,
    /// Why not, when blocked.
    pub reason: Option<String>,
}

/// Where the simulation is after a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimStep {
    /// Current state.
    pub state_id: Uuid,
    /// Variable values.
    pub env: Env,
    /// The current state is final.
    pub is_final: bool,
    /// Outgoing transitions, in model order.
    pub transitions: Vec<SimTransition>,
}

/// Why a step cannot be computed.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SimError {
    /// The model has validation errors (e.g. no or several initial states, bad expressions).
    #[error("model has validation errors")]
    InvalidModel(Vec<ValidationIssue>),
    /// `stateId` is not a state of the model.
    #[error("unknown state {0}")]
    UnknownState(Uuid),
    /// `take` is not an outgoing transition of the current state.
    #[error("transition {0} does not leave the current state")]
    NotOutgoing(Uuid),
    /// `take` is blocked.
    #[error("{0}")]
    Blocked(String),
}

/// Starts (without `state`) or continues a simulation; with `take`, takes that transition first.
pub fn simulate(
    graph: &ModelGraph,
    state: Option<Uuid>,
    env: Option<Env>,
    take: Option<Uuid>,
) -> Result<SimStep, SimError> {
    let issues = validate(graph);
    if has_errors(&issues) {
        return Err(SimError::InvalidModel(
            issues
                .into_iter()
                .filter(|i| i.severity == tm_domain::Severity::Error)
                .collect(),
        ));
    }
    let machine = Machine::new(graph, 0).ok_or_else(|| SimError::InvalidModel(Vec::new()))?;
    let (mut current, mut env) = match state {
        None => (machine.initial, expr::initial_env(&graph.variables)),
        Some(id) => (
            graph
                .states
                .iter()
                .position(|s| s.id == id)
                .ok_or(SimError::UnknownState(id))?,
            env.unwrap_or_else(|| expr::initial_env(&graph.variables)),
        ),
    };
    if let Some(id) = take {
        let t = graph
            .transitions
            .iter()
            .position(|t| t.id == id)
            .filter(|&t| machine.transitions[t].from == current)
            .ok_or(SimError::NotOutgoing(id))?;
        match machine.step(&env, t) {
            Step::Taken(next) => {
                env = next;
                current = machine.transitions[t].to;
            }
            Step::Blocked(reason) => return Err(SimError::Blocked(reason)),
        }
    }
    let transitions = (0..graph.transitions.len())
        .filter(|&t| machine.transitions[t].from == current)
        .map(|t| {
            let (enabled, reason) = match machine.step(&env, t) {
                Step::Taken(_) => (true, None),
                Step::Blocked(r) => (false, Some(r)),
            };
            SimTransition {
                transition_id: graph.transitions[t].id,
                enabled,
                reason,
            }
        })
        .collect();
    let state = &graph.states[current];
    Ok(SimStep {
        state_id: state.id,
        env,
        is_final: state.kind == StateKind::Final,
        transitions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, GenerationOptions};
    use proptest::prelude::*;
    use tm_domain::expr::Value;
    use tm_domain::{CoverageCriterion, State, Transition, Variable, VariableType};

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

    /// Start -> Out; Out --retry [attempts < 2] / attempts++ --> Out; Out --lock [attempts >= 2]--> Locked.
    fn model() -> ModelGraph {
        let start = state("Start", StateKind::Initial);
        let out = state("Out", StateKind::Normal);
        let locked = state("Locked", StateKind::Final);
        let stuck = state("Stuck", StateKind::Normal);
        let mut retry = tr(&out, &out, "retry");
        retry.guard = Some("attempts < 2".into());
        retry.action = Some("attempts = attempts + 1".into());
        let mut lock = tr(&out, &locked, "lock");
        lock.guard = Some("attempts >= 2".into());
        ModelGraph {
            variables: vec![Variable {
                name: "attempts".into(),
                var_type: VariableType::Integer,
                initial: None,
            }],
            transitions: vec![
                tr(&start, &out, "open"),
                retry,
                lock,
                tr(&start, &stuck, "away"),
            ],
            states: vec![start, out, locked, stuck],
        }
    }

    #[test]
    fn starts_at_the_initial_state_with_initial_values() {
        let g = model();
        let s = simulate(&g, None, None, None).unwrap();
        assert_eq!(s.state_id, g.states[0].id);
        assert_eq!(s.env.get("attempts"), Some(&Value::Int(0)));
        assert_eq!(s.transitions.len(), 2);
        assert!(s.transitions.iter().all(|t| t.enabled));
        assert!(!s.is_final);
    }

    #[test]
    fn lists_blocked_transitions_with_the_false_guard() {
        let g = model();
        let s = simulate(&g, None, None, Some(g.transitions[0].id)).unwrap();
        assert_eq!(s.state_id, g.states[1].id);
        let lock = s
            .transitions
            .iter()
            .find(|t| t.transition_id == g.transitions[2].id)
            .unwrap();
        assert!(!lock.enabled);
        assert_eq!(
            lock.reason.as_deref(),
            Some("guard `attempts >= 2` is false")
        );
    }

    #[test]
    fn applies_actions_and_ends_in_a_final_state() {
        let g = model();
        let mut s = simulate(&g, None, None, Some(g.transitions[0].id)).unwrap();
        for _ in 0..2 {
            s = simulate(
                &g,
                Some(s.state_id),
                Some(s.env.clone()),
                Some(g.transitions[1].id),
            )
            .unwrap();
        }
        assert_eq!(s.env.get("attempts"), Some(&Value::Int(2)));
        let retry = s
            .transitions
            .iter()
            .find(|t| t.transition_id == g.transitions[1].id)
            .unwrap();
        assert!(!retry.enabled);
        let done = simulate(&g, Some(s.state_id), Some(s.env), Some(g.transitions[2].id)).unwrap();
        assert!(done.is_final);
        assert!(done.transitions.is_empty());
    }

    #[test]
    fn a_dead_end_has_no_enabled_transition() {
        let g = model();
        let s = simulate(&g, None, None, Some(g.transitions[3].id)).unwrap();
        assert!(!s.is_final);
        assert!(s.transitions.is_empty());
    }

    #[test]
    fn rejects_blocked_and_foreign_transitions_and_invalid_models() {
        let g = model();
        let at_out = simulate(&g, None, None, Some(g.transitions[0].id)).unwrap();
        assert_eq!(
            simulate(
                &g,
                Some(at_out.state_id),
                Some(at_out.env.clone()),
                Some(g.transitions[2].id)
            ),
            Err(SimError::Blocked("guard `attempts >= 2` is false".into()))
        );
        assert_eq!(
            simulate(
                &g,
                Some(at_out.state_id),
                Some(at_out.env),
                Some(g.transitions[0].id)
            ),
            Err(SimError::NotOutgoing(g.transitions[0].id))
        );
        let mut two = g.clone();
        two.states[1].kind = StateKind::Initial;
        assert!(
            matches!(simulate(&two, None, None, None), Err(SimError::InvalidModel(i)) if !i.is_empty())
        );
    }

    proptest! {
        /// Same semantics as generation: every generated path can be replayed step by step.
        #[test]
        fn generated_paths_replay(seed in any::<u64>(), crit in 0usize..3) {
            let g = model();
            let criterion = [CoverageCriterion::State, CoverageCriterion::Transition, CoverageCriterion::TransitionPair][crit];
            let out = generate(&g, GenerationOptions { criterion, seed, max_path_length: None }).unwrap();
            for path in out.paths {
                let mut s = simulate(&g, None, None, None).unwrap();
                for t in &path.transitions {
                    prop_assert!(s.transitions.iter().any(|x| x.transition_id == *t && x.enabled));
                    s = simulate(&g, Some(s.state_id), Some(s.env.clone()), Some(*t)).unwrap();
                }
                prop_assert_eq!(Some(&s.state_id), path.states.last());
            }
        }
    }
}
