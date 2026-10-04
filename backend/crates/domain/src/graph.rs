//! State machine graph of a model.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::expr::Value;

/// Kind of a state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StateKind {
    /// Start state; exactly one per model.
    Initial,
    /// Ordinary state.
    Normal,
    /// End state.
    Final,
}

impl StateKind {
    /// Wire and storage representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Normal => "normal",
            Self::Final => "final",
        }
    }
}

impl std::str::FromStr for StateKind {
    type Err = crate::ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "initial" => Ok(Self::Initial),
            "normal" => Ok(Self::Normal),
            "final" => Ok(Self::Final),
            _ => Err(crate::ParseEnumError {
                kind: "StateKind",
                value: s.to_owned(),
            }),
        }
    }
}

/// Canvas position of a state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate.
    pub y: f64,
}

/// Node of a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    /// Id.
    pub id: Uuid,
    /// Name.
    pub name: String,
    /// Optional description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Kind.
    pub kind: StateKind,
    /// Canvas position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
}

/// Edge of a model (model step).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    /// Id.
    pub id: Uuid,
    /// Source state id.
    pub from: Uuid,
    /// Target state id.
    pub to: Uuid,
    /// Triggering event.
    pub event: String,
    /// Optional guard expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<String>,
    /// Optional action (assignments).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// Optional expected result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
}

/// Type of a model variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VariableType {
    /// `true` / `false`.
    Boolean,
    /// 64-bit signed integer.
    Integer,
    /// Text.
    String,
}

/// Model variable used by guards and actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    /// Name, an identifier.
    pub name: String,
    /// Type.
    #[serde(rename = "type")]
    pub var_type: VariableType,
    /// Initial value; defaults to `false`, `0` or `""`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial: Option<serde_json::Value>,
}

impl Variable {
    /// Typed initial value, or `None` if `initial` does not match the declared type.
    pub fn initial_value(&self) -> Option<Value> {
        match (&self.initial, self.var_type) {
            (None | Some(serde_json::Value::Null), VariableType::Boolean) => {
                Some(Value::Bool(false))
            }
            (None | Some(serde_json::Value::Null), VariableType::Integer) => Some(Value::Int(0)),
            (None | Some(serde_json::Value::Null), VariableType::String) => {
                Some(Value::Str(String::new()))
            }
            (Some(serde_json::Value::Bool(b)), VariableType::Boolean) => Some(Value::Bool(*b)),
            (Some(serde_json::Value::Number(n)), VariableType::Integer) => {
                n.as_i64().map(Value::Int)
            }
            (Some(serde_json::Value::String(s)), VariableType::String) => {
                Some(Value::Str(s.clone()))
            }
            _ => None,
        }
    }
}

/// States, transitions and variables of a model.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModelGraph {
    /// Variables.
    #[serde(default)]
    pub variables: Vec<Variable>,
    /// States.
    #[serde(default)]
    pub states: Vec<State>,
    /// Transitions.
    #[serde(default)]
    pub transitions: Vec<Transition>,
}

impl ModelGraph {
    /// Looks up a state by id.
    pub fn state(&self, id: Uuid) -> Option<&State> {
        self.states.iter().find(|s| s.id == id)
    }

    /// Looks up a transition by id.
    pub fn transition(&self, id: Uuid) -> Option<&Transition> {
        self.transitions.iter().find(|t| t.id == id)
    }

    /// The initial state if there is exactly one.
    pub fn initial_state(&self) -> Option<&State> {
        let mut initial = self.states.iter().filter(|s| s.kind == StateKind::Initial);
        match (initial.next(), initial.next()) {
            (Some(s), None) => Some(s),
            _ => None,
        }
    }
}
