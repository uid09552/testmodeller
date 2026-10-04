//! Domain entities, validation and expression language. No I/O. See docs/specification/02-domain-model.md.

pub mod entities;
pub mod expr;
pub mod graph;
pub mod validation;

pub use entities::*;
pub use graph::{ModelGraph, Position, State, StateKind, Transition, Variable, VariableType};
pub use validation::{validate, Severity, ValidationIssue};
