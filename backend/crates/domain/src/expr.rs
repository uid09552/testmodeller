//! Guard and action expression language. See docs/specification/04-test-generation.md#expressions.
//!
//! Guards are boolean expressions over model variables. Actions are `;`-separated assignments
//! `name = expr`. Supported operators: `|| && ! == != < <= > >= + -` and parentheses.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use crate::graph::{Variable, VariableType};

/// Runtime value of a variable or expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Value {
    /// Boolean.
    Bool(bool),
    /// Integer.
    Int(i64),
    /// String.
    Str(String),
}

impl Value {
    fn type_of(&self) -> VariableType {
        match self {
            Self::Bool(_) => VariableType::Boolean,
            Self::Int(_) => VariableType::Integer,
            Self::Str(_) => VariableType::String,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(b) => write!(f, "{b}"),
            Self::Int(i) => write!(f, "{i}"),
            Self::Str(s) => write!(f, "\"{s}\""),
        }
    }
}

/// Variable assignment used during evaluation. Ordered so it can be hashed deterministically.
pub type Env = BTreeMap<String, Value>;

/// Errors from parsing, type checking or evaluating expressions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExprError {
    /// Malformed input.
    #[error("syntax error at {pos}: {msg}")]
    Syntax {
        /// Byte offset.
        pos: usize,
        /// Explanation.
        msg: String,
    },
    /// Reference to an undeclared variable.
    #[error("unknown variable '{0}'")]
    UnknownVariable(String),
    /// Operand types do not fit the operator.
    #[error("type error: {0}")]
    Type(String),
    /// Integer overflow during evaluation.
    #[error("integer overflow")]
    Overflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnOp {
    Not,
    Neg,
}

/// Parsed expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal value.
    Lit(Value),
    /// Variable reference.
    Var(String),
    /// Unary operation.
    Unary(UnOpBox),
    /// Binary operation.
    Binary(BinBox),
}

/// Boxed unary operation (opaque).
#[derive(Debug, Clone, PartialEq)]
pub struct UnOpBox(UnOp, Box<Expr>);

/// Boxed binary operation (opaque).
#[derive(Debug, Clone, PartialEq)]
pub struct BinBox(BinOp, Box<Expr>, Box<Expr>);

/// One assignment of an action.
#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    /// Assigned variable.
    pub target: String,
    /// Value expression.
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Int(i64),
    Str(String),
    Ident(String),
    Op(&'static str),
    LParen,
    RParen,
    Semi,
    Assign,
}

fn lex(src: &str) -> Result<Vec<(usize, Tok)>, ExprError> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let err = |pos: usize, msg: &str| ExprError::Syntax {
        pos,
        msg: msg.to_owned(),
    };
    while i < bytes.len() {
        let c = bytes[i] as char;
        let start = i;
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() {
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let n = src[start..i]
                .parse::<i64>()
                .map_err(|_| err(start, "integer literal out of range"))?;
            out.push((start, Tok::Int(n)));
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            out.push((start, Tok::Ident(src[start..i].to_owned())));
            continue;
        }
        if c == '"' || c == '\'' {
            i += 1;
            let body_start = i;
            while i < bytes.len() && bytes[i] as char != c {
                i += 1;
            }
            if i >= bytes.len() {
                return Err(err(start, "unterminated string"));
            }
            out.push((start, Tok::Str(src[body_start..i].to_owned())));
            i += 1;
            continue;
        }
        let two = src.get(i..i + 2).unwrap_or("");
        let tok = match two {
            "||" => Some(Tok::Op("||")),
            "&&" => Some(Tok::Op("&&")),
            "==" => Some(Tok::Op("==")),
            "!=" => Some(Tok::Op("!=")),
            "<=" => Some(Tok::Op("<=")),
            ">=" => Some(Tok::Op(">=")),
            _ => None,
        };
        if let Some(tok) = tok {
            out.push((start, tok));
            i += 2;
            continue;
        }
        let tok = match c {
            '<' => Tok::Op("<"),
            '>' => Tok::Op(">"),
            '!' => Tok::Op("!"),
            '+' => Tok::Op("+"),
            '-' => Tok::Op("-"),
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            ';' => Tok::Semi,
            '=' => Tok::Assign,
            _ => return Err(err(start, &format!("unexpected character '{c}'"))),
        };
        out.push((start, tok));
        i += 1;
    }
    Ok(out)
}

struct Parser {
    toks: Vec<(usize, Tok)>,
    pos: usize,
    end: usize,
}

impl Parser {
    fn new(src: &str) -> Result<Self, ExprError> {
        Ok(Self {
            toks: lex(src)?,
            pos: 0,
            end: src.len(),
        })
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(_, t)| t)
    }

    fn offset(&self) -> usize {
        self.toks.get(self.pos).map_or(self.end, |(p, _)| *p)
    }

    fn error(&self, msg: &str) -> ExprError {
        ExprError::Syntax {
            pos: self.offset(),
            msg: msg.to_owned(),
        }
    }

    fn eat_op(&mut self, ops: &[&'static str]) -> Option<&'static str> {
        if let Some(Tok::Op(op)) = self.peek() {
            if let Some(found) = ops.iter().find(|o| *o == op) {
                self.pos += 1;
                return Some(found);
            }
        }
        None
    }

    fn expr(&mut self) -> Result<Expr, ExprError> {
        self.or()
    }

    fn binary_chain(
        &mut self,
        ops: &[&'static str],
        next: fn(&mut Self) -> Result<Expr, ExprError>,
    ) -> Result<Expr, ExprError> {
        let mut lhs = next(self)?;
        while let Some(op) = self.eat_op(ops) {
            let rhs = next(self)?;
            lhs = Expr::Binary(BinBox(bin_op(op), Box::new(lhs), Box::new(rhs)));
        }
        Ok(lhs)
    }

    fn or(&mut self) -> Result<Expr, ExprError> {
        self.binary_chain(&["||"], Self::and)
    }

    fn and(&mut self) -> Result<Expr, ExprError> {
        self.binary_chain(&["&&"], Self::cmp)
    }

    fn cmp(&mut self) -> Result<Expr, ExprError> {
        let lhs = self.add()?;
        if let Some(op) = self.eat_op(&["==", "!=", "<", "<=", ">", ">="]) {
            let rhs = self.add()?;
            return Ok(Expr::Binary(BinBox(
                bin_op(op),
                Box::new(lhs),
                Box::new(rhs),
            )));
        }
        Ok(lhs)
    }

    fn add(&mut self) -> Result<Expr, ExprError> {
        self.binary_chain(&["+", "-"], Self::unary)
    }

    fn unary(&mut self) -> Result<Expr, ExprError> {
        if let Some(op) = self.eat_op(&["!", "-"]) {
            let inner = self.unary()?;
            let op = if op == "!" { UnOp::Not } else { UnOp::Neg };
            return Ok(Expr::Unary(UnOpBox(op, Box::new(inner))));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, ExprError> {
        let tok = self.peek().cloned();
        match tok {
            Some(Tok::Int(n)) => {
                self.pos += 1;
                Ok(Expr::Lit(Value::Int(n)))
            }
            Some(Tok::Str(s)) => {
                self.pos += 1;
                Ok(Expr::Lit(Value::Str(s)))
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                Ok(match name.as_str() {
                    "true" => Expr::Lit(Value::Bool(true)),
                    "false" => Expr::Lit(Value::Bool(false)),
                    _ => Expr::Var(name),
                })
            }
            Some(Tok::LParen) => {
                self.pos += 1;
                let inner = self.expr()?;
                if self.peek() != Some(&Tok::RParen) {
                    return Err(self.error("expected ')'"));
                }
                self.pos += 1;
                Ok(inner)
            }
            _ => Err(self.error("expected value, variable or '('")),
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.toks.len()
    }
}

fn bin_op(op: &str) -> BinOp {
    match op {
        "||" => BinOp::Or,
        "&&" => BinOp::And,
        "==" => BinOp::Eq,
        "!=" => BinOp::Ne,
        "<" => BinOp::Lt,
        "<=" => BinOp::Le,
        ">" => BinOp::Gt,
        ">=" => BinOp::Ge,
        "+" => BinOp::Add,
        _ => BinOp::Sub,
    }
}

/// Parses a guard expression.
pub fn parse_guard(src: &str) -> Result<Expr, ExprError> {
    let mut p = Parser::new(src)?;
    let e = p.expr()?;
    if !p.at_end() {
        return Err(p.error("unexpected trailing input"));
    }
    Ok(e)
}

/// Parses an action: zero or more `name = expr` separated by `;`.
pub fn parse_action(src: &str) -> Result<Vec<Assign>, ExprError> {
    let mut p = Parser::new(src)?;
    let mut out = Vec::new();
    while !p.at_end() {
        if p.peek() == Some(&Tok::Semi) {
            p.pos += 1;
            continue;
        }
        let Some(Tok::Ident(target)) = p.peek().cloned() else {
            return Err(p.error("expected variable name"));
        };
        p.pos += 1;
        if p.peek() != Some(&Tok::Assign) {
            return Err(p.error("expected '='"));
        }
        p.pos += 1;
        let value = p.expr()?;
        out.push(Assign { target, value });
        if !p.at_end() && p.peek() != Some(&Tok::Semi) {
            return Err(p.error("expected ';'"));
        }
    }
    Ok(out)
}

fn type_of(e: &Expr, vars: &HashMap<&str, VariableType>) -> Result<VariableType, ExprError> {
    use VariableType as T;
    match e {
        Expr::Lit(v) => Ok(v.type_of()),
        Expr::Var(name) => vars
            .get(name.as_str())
            .copied()
            .ok_or_else(|| ExprError::UnknownVariable(name.clone())),
        Expr::Unary(UnOpBox(op, inner)) => {
            let t = type_of(inner, vars)?;
            match (op, t) {
                (UnOp::Not, T::Boolean) => Ok(T::Boolean),
                (UnOp::Neg, T::Integer) => Ok(T::Integer),
                _ => Err(ExprError::Type(format!("invalid operand {t:?} for {op:?}"))),
            }
        }
        Expr::Binary(BinBox(op, l, r)) => {
            let (lt, rt) = (type_of(l, vars)?, type_of(r, vars)?);
            let mismatch =
                || ExprError::Type(format!("invalid operands {lt:?}, {rt:?} for {op:?}"));
            match op {
                BinOp::Or | BinOp::And if lt == T::Boolean && rt == T::Boolean => Ok(T::Boolean),
                BinOp::Eq | BinOp::Ne if lt == rt => Ok(T::Boolean),
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge if lt == rt && lt != T::Boolean => {
                    Ok(T::Boolean)
                }
                BinOp::Add if lt == rt && lt != T::Boolean => Ok(lt),
                BinOp::Sub if lt == T::Integer && rt == T::Integer => Ok(T::Integer),
                _ => Err(mismatch()),
            }
        }
    }
}

fn var_types(vars: &[Variable]) -> HashMap<&str, VariableType> {
    vars.iter().map(|v| (v.name.as_str(), v.var_type)).collect()
}

/// Parses and type-checks a guard against the declared variables.
pub fn check_guard(src: &str, vars: &[Variable]) -> Result<Expr, ExprError> {
    let e = parse_guard(src)?;
    match type_of(&e, &var_types(vars))? {
        VariableType::Boolean => Ok(e),
        t => Err(ExprError::Type(format!("guard must be boolean, got {t:?}"))),
    }
}

/// Parses and type-checks an action against the declared variables.
pub fn check_action(src: &str, vars: &[Variable]) -> Result<Vec<Assign>, ExprError> {
    let assigns = parse_action(src)?;
    let types = var_types(vars);
    for a in &assigns {
        let target = types
            .get(a.target.as_str())
            .copied()
            .ok_or_else(|| ExprError::UnknownVariable(a.target.clone()))?;
        let t = type_of(&a.value, &types)?;
        if t != target {
            return Err(ExprError::Type(format!(
                "cannot assign {t:?} to '{}' of type {target:?}",
                a.target
            )));
        }
    }
    Ok(assigns)
}

/// Evaluates an expression in an environment.
pub fn eval(e: &Expr, env: &Env) -> Result<Value, ExprError> {
    match e {
        Expr::Lit(v) => Ok(v.clone()),
        Expr::Var(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| ExprError::UnknownVariable(name.clone())),
        Expr::Unary(UnOpBox(op, inner)) => match (op, eval(inner, env)?) {
            (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
            (UnOp::Neg, Value::Int(i)) => {
                i.checked_neg().map(Value::Int).ok_or(ExprError::Overflow)
            }
            (op, v) => Err(ExprError::Type(format!("invalid operand {v} for {op:?}"))),
        },
        Expr::Binary(BinBox(op, l, r)) => {
            // Short-circuit boolean operators.
            if matches!(op, BinOp::Or | BinOp::And) {
                let Value::Bool(lb) = eval(l, env)? else {
                    return Err(ExprError::Type("boolean expected".into()));
                };
                if (*op == BinOp::Or && lb) || (*op == BinOp::And && !lb) {
                    return Ok(Value::Bool(lb));
                }
                return match eval(r, env)? {
                    Value::Bool(rb) => Ok(Value::Bool(rb)),
                    _ => Err(ExprError::Type("boolean expected".into())),
                };
            }
            let (lv, rv) = (eval(l, env)?, eval(r, env)?);
            if lv.type_of() != rv.type_of() {
                return Err(ExprError::Type(format!("cannot combine {lv} and {rv}")));
            }
            Ok(match op {
                BinOp::Eq => Value::Bool(lv == rv),
                BinOp::Ne => Value::Bool(lv != rv),
                BinOp::Lt => Value::Bool(lv < rv),
                BinOp::Le => Value::Bool(lv <= rv),
                BinOp::Gt => Value::Bool(lv > rv),
                BinOp::Ge => Value::Bool(lv >= rv),
                BinOp::Add => match (lv, rv) {
                    (Value::Int(a), Value::Int(b)) => {
                        Value::Int(a.checked_add(b).ok_or(ExprError::Overflow)?)
                    }
                    (Value::Str(a), Value::Str(b)) => Value::Str(a + &b),
                    _ => return Err(ExprError::Type("cannot add booleans".into())),
                },
                BinOp::Sub => match (lv, rv) {
                    (Value::Int(a), Value::Int(b)) => {
                        Value::Int(a.checked_sub(b).ok_or(ExprError::Overflow)?)
                    }
                    _ => return Err(ExprError::Type("integers expected".into())),
                },
                // Handled by the short-circuit branch above.
                BinOp::Or | BinOp::And => {
                    return Err(ExprError::Type(format!("unexpected operator {op:?}")))
                }
            })
        }
    }
}

/// Evaluates a guard; non-boolean results are an error.
pub fn eval_guard(e: &Expr, env: &Env) -> Result<bool, ExprError> {
    match eval(e, env)? {
        Value::Bool(b) => Ok(b),
        v => Err(ExprError::Type(format!("guard evaluated to {v}"))),
    }
}

/// Applies assignments sequentially, returning the new environment.
pub fn apply_action(assigns: &[Assign], env: &Env) -> Result<Env, ExprError> {
    let mut next = env.clone();
    for a in assigns {
        let v = eval(&a.value, &next)?;
        next.insert(a.target.clone(), v);
    }
    Ok(next)
}

/// Initial environment of a model's variables. Invalid initial values fall back to defaults.
pub fn initial_env(vars: &[Variable]) -> Env {
    vars.iter()
        .map(|v| {
            let value = v.initial_value().unwrap_or(match v.var_type {
                VariableType::Boolean => Value::Bool(false),
                VariableType::Integer => Value::Int(0),
                VariableType::String => Value::Str(String::new()),
            });
            (v.name.clone(), value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Vec<Variable> {
        vec![
            Variable {
                name: "count".into(),
                var_type: VariableType::Integer,
                initial: Some(serde_json::json!(2)),
            },
            Variable {
                name: "loggedIn".into(),
                var_type: VariableType::Boolean,
                initial: None,
            },
            Variable {
                name: "role".into(),
                var_type: VariableType::String,
                initial: Some(serde_json::json!("admin")),
            },
        ]
    }

    #[test]
    fn evaluates_guards_with_precedence() {
        let env = initial_env(&vars());
        let g = check_guard("!loggedIn && count > 1 || role == 'x'", &vars()).unwrap();
        assert!(eval_guard(&g, &env).unwrap());
        let g = check_guard("(count - 3) >= 0", &vars()).unwrap();
        assert!(!eval_guard(&g, &env).unwrap());
    }

    #[test]
    fn applies_actions_in_order() {
        let a = check_action("count = count + 1; loggedIn = count == 3;", &vars()).unwrap();
        let env = apply_action(&a, &initial_env(&vars())).unwrap();
        assert_eq!(env["count"], Value::Int(3));
        assert_eq!(env["loggedIn"], Value::Bool(true));
    }

    #[test]
    fn rejects_type_errors_and_unknown_variables() {
        assert!(matches!(
            check_guard("count", &vars()),
            Err(ExprError::Type(_))
        ));
        assert!(matches!(
            check_guard("missing == 1", &vars()),
            Err(ExprError::UnknownVariable(_))
        ));
        assert!(matches!(
            check_action("count = true", &vars()),
            Err(ExprError::Type(_))
        ));
        assert!(matches!(
            check_guard("count >", &vars()),
            Err(ExprError::Syntax { .. })
        ));
        assert!(matches!(
            check_guard("a b", &vars()),
            Err(ExprError::Syntax { .. })
        ));
    }

    #[test]
    fn overflow_is_an_error() {
        let a = check_action("count = count + 9223372036854775807", &vars()).unwrap();
        assert_eq!(
            apply_action(&a, &initial_env(&vars())),
            Err(ExprError::Overflow)
        );
    }

    #[test]
    fn empty_action_is_noop() {
        assert!(parse_action("  ").unwrap().is_empty());
    }
}
