//! Comparison of two versions of a model (FR-014). Elements are matched by id (variables by
//! name); presentation (positions, the editor layout) is ignored.

use std::collections::HashMap;

use serde::Serialize;
use uuid::Uuid;

use crate::entities::ModelSnapshot;
use crate::graph::{State, Transition, Variable};

/// An element present on both sides with different content.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Changed<T> {
    /// The element before.
    pub before: T,
    /// The element after.
    pub after: T,
    /// Names of the fields that differ.
    pub fields: Vec<&'static str>,
}

/// Added, removed and changed elements of one kind.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diff<T> {
    /// Only in the later version.
    pub added: Vec<T>,
    /// Only in the earlier version.
    pub removed: Vec<T>,
    /// In both, with differences.
    pub changed: Vec<Changed<T>>,
}

impl<T> Default for Diff<T> {
    fn default() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            changed: Vec::new(),
        }
    }
}

impl<T> Diff<T> {
    /// True when nothing differs.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

/// Differences of name, description and status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaChanges {
    /// Names of the fields that differ: `name`, `description`, `status`.
    pub fields: Vec<&'static str>,
}

/// What differs between two versions of a model.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelComparison {
    /// Name, description and status.
    pub meta: MetaChanges,
    /// States, matched by id.
    pub states: Diff<State>,
    /// Transitions, matched by id.
    pub transitions: Diff<Transition>,
    /// Variables, matched by name.
    pub variables: Diff<Variable>,
}

impl ModelComparison {
    /// True when the versions are equal in everything compared.
    pub fn is_empty(&self) -> bool {
        self.meta.fields.is_empty()
            && self.states.is_empty()
            && self.transitions.is_empty()
            && self.variables.is_empty()
    }
}

fn diff_by<T: Clone, K: std::hash::Hash + Eq>(
    before: &[T],
    after: &[T],
    key: impl Fn(&T) -> K,
    fields: impl Fn(&T, &T) -> Vec<&'static str>,
) -> Diff<T> {
    let old: HashMap<K, &T> = before.iter().map(|t| (key(t), t)).collect();
    let new: HashMap<K, &T> = after.iter().map(|t| (key(t), t)).collect();
    let mut out = Diff::default();
    for a in after {
        match old.get(&key(a)) {
            None => out.added.push(a.clone()),
            Some(b) => {
                let f = fields(b, a);
                if !f.is_empty() {
                    out.changed.push(Changed {
                        before: (*b).clone(),
                        after: a.clone(),
                        fields: f,
                    });
                }
            }
        }
    }
    out.removed = before
        .iter()
        .filter(|b| !new.contains_key(&key(b)))
        .cloned()
        .collect();
    out
}

fn state_fields(a: &State, b: &State) -> Vec<&'static str> {
    let mut f = Vec::new();
    if a.name != b.name {
        f.push("name");
    }
    if a.kind != b.kind {
        f.push("kind");
    }
    if a.description != b.description {
        f.push("description");
    }
    if a.requirements != b.requirements {
        f.push("requirements");
    }
    f
}

fn transition_fields(a: &Transition, b: &Transition) -> Vec<&'static str> {
    let mut f = Vec::new();
    if a.from != b.from {
        f.push("from");
    }
    if a.to != b.to {
        f.push("to");
    }
    if a.event != b.event {
        f.push("event");
    }
    if a.guard != b.guard {
        f.push("guard");
    }
    if a.action != b.action {
        f.push("action");
    }
    if a.expected != b.expected {
        f.push("expected");
    }
    if a.weight != b.weight {
        f.push("weight");
    }
    if a.requirements != b.requirements {
        f.push("requirements");
    }
    f
}

fn variable_fields(a: &Variable, b: &Variable) -> Vec<&'static str> {
    let mut f = Vec::new();
    if a.var_type != b.var_type {
        f.push("type");
    }
    if a.initial != b.initial {
        f.push("initial");
    }
    f
}

/// Compares `before` with `after`.
pub fn compare(before: &ModelSnapshot, after: &ModelSnapshot) -> ModelComparison {
    let mut meta = Vec::new();
    if before.name != after.name {
        meta.push("name");
    }
    if before.description != after.description {
        meta.push("description");
    }
    if before.status != after.status {
        meta.push("status");
    }
    ModelComparison {
        meta: MetaChanges { fields: meta },
        states: diff_by(
            &before.graph.states,
            &after.graph.states,
            |s: &State| s.id,
            state_fields,
        ),
        transitions: diff_by(
            &before.graph.transitions,
            &after.graph.transitions,
            |t: &Transition| t.id,
            transition_fields,
        ),
        variables: diff_by(
            &before.graph.variables,
            &after.graph.variables,
            |v: &Variable| v.name.clone(),
            variable_fields,
        ),
    }
}

/// Ids of the elements a comparison touches, for highlighting.
pub fn touched_ids(c: &ModelComparison) -> Vec<Uuid> {
    let mut ids: Vec<Uuid> = Vec::new();
    ids.extend(c.states.added.iter().map(|s| s.id));
    ids.extend(c.states.removed.iter().map(|s| s.id));
    ids.extend(c.states.changed.iter().map(|s| s.after.id));
    ids.extend(c.transitions.added.iter().map(|t| t.id));
    ids.extend(c.transitions.removed.iter().map(|t| t.id));
    ids.extend(c.transitions.changed.iter().map(|t| t.after.id));
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::ModelStatus;
    use crate::graph::{ModelGraph, StateKind};

    fn state(id: u128, name: &str) -> State {
        State {
            id: Uuid::from_u128(id),
            name: name.into(),
            description: None,
            kind: StateKind::Normal,
            position: None,
            requirements: Vec::new(),
        }
    }

    fn snap(states: Vec<State>, transitions: Vec<Transition>) -> ModelSnapshot {
        ModelSnapshot {
            name: "M".into(),
            description: None,
            status: ModelStatus::Draft,
            graph: ModelGraph {
                variables: vec![],
                states,
                transitions,
            },
            layout: None,
        }
    }

    fn tr(id: u128, from: u128, to: u128, event: &str) -> Transition {
        Transition {
            id: Uuid::from_u128(id),
            from: Uuid::from_u128(from),
            to: Uuid::from_u128(to),
            event: event.into(),
            guard: None,
            action: None,
            expected: None,
            weight: None,
            requirements: Vec::new(),
        }
    }

    #[test]
    fn equal_versions_have_no_differences_and_positions_are_ignored() {
        let a = snap(vec![state(1, "A")], vec![]);
        let mut b = a.clone();
        b.graph.states[0].position = Some(crate::graph::Position { x: 5.0, y: 5.0 });
        b.layout = Some(serde_json::json!({ "v": 1 }));
        assert!(compare(&a, &b).is_empty());
    }

    #[test]
    fn added_removed_and_changed_are_found_by_id() {
        let a = snap(vec![state(1, "A"), state(2, "B")], vec![tr(10, 1, 2, "go")]);
        let mut b = snap(
            vec![state(1, "A renamed"), state(3, "C")],
            vec![tr(10, 1, 3, "go")],
        );
        b.name = "M2".into();
        let c = compare(&a, &b);
        assert_eq!(c.meta.fields, ["name"]);
        assert_eq!(c.states.added.len(), 1);
        assert_eq!(c.states.removed[0].name, "B");
        assert_eq!(c.states.changed[0].fields, ["name"]);
        assert_eq!(c.transitions.changed[0].fields, ["to"]);
        assert_eq!(touched_ids(&c).len(), 4);
    }
}
