//! ECSS AST types.
//!
//! napi-rs v3 does not support `#[napi(object)]` on Rust enums with data or Box<T>.
//! Strategy:
//!   - Non-recursive structs: `#[napi(object)]`
//!   - Recursive `ConditionExpr`: derives `serde::Serialize`, stored as `serde_json::Value`
//!     in `IfClause`. TypeScript sees the condition as `unknown` and can access it at runtime.
//!   - Enum variants are represented as tagged structs with a `kind: String` discriminant.

use napi_derive::napi;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Source location
// ---------------------------------------------------------------------------

#[napi(object)]
#[derive(Debug, Clone)]
pub struct Span {
  pub line: u32,
  pub column: u32,
  pub end_line: u32,
  pub end_column: u32,
}

// ---------------------------------------------------------------------------
// Top-level stylesheet
// ---------------------------------------------------------------------------

#[napi(object)]
#[derive(Debug, Clone)]
pub struct EcssStylesheet {
  pub rules: Vec<EcssRule>,
}

// ---------------------------------------------------------------------------
// Top-level rule  (tagged union)
// ---------------------------------------------------------------------------

#[napi(object)]
#[derive(Debug, Clone)]
pub struct EcssRule {
  /// Discriminant: "state-variant" | "state-def" | "qualified-rule" | "at-rule"
  pub kind: String,

  pub state_variant: Option<StateVariant>,
  pub state_def: Option<StateDef>,
  pub qualified_rule: Option<CssQualifiedRule>,
  pub at_rule: Option<CssRawAtRule>,
}

impl EcssRule {
  pub fn variant(v: StateVariant) -> Self {
    Self {
      kind: "state-variant".into(),
      state_variant: Some(v),
      state_def: None,
      qualified_rule: None,
      at_rule: None,
    }
  }

  pub fn state_def(d: StateDef) -> Self {
    Self {
      kind: "state-def".into(),
      state_variant: None,
      state_def: Some(d),
      qualified_rule: None,
      at_rule: None,
    }
  }

  pub fn qualified(q: CssQualifiedRule) -> Self {
    Self {
      kind: "qualified-rule".into(),
      state_variant: None,
      state_def: None,
      qualified_rule: Some(q),
      at_rule: None,
    }
  }

  pub fn at_rule(r: CssRawAtRule) -> Self {
    Self {
      kind: "at-rule".into(),
      state_variant: None,
      state_def: None,
      qualified_rule: None,
      at_rule: Some(r),
    }
  }
}

// ---------------------------------------------------------------------------
// @state-variant
// ---------------------------------------------------------------------------

#[napi(object)]
#[derive(Debug, Clone)]
pub struct StateVariant {
  pub name: String,
  pub values: Vec<String>,
  pub span: Span,
}

// ---------------------------------------------------------------------------
// @state-def
// ---------------------------------------------------------------------------

#[napi(object)]
#[derive(Debug, Clone)]
pub struct StateDef {
  pub name: String,
  pub params: Vec<StateParam>,
  pub body: Vec<StateDefItem>,
  pub span: Span,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct StateParam {
  pub name: String,
  /// "boolean" | "variant"
  pub param_type: String,
  /// For variant params: the @state-variant name (e.g. "Theme")
  pub variant_name: Option<String>,
  /// Default value as a string ("true"/"false" for boolean; quoted value for variant)
  pub default_value: Option<String>,
}

// ---------------------------------------------------------------------------
// Items inside @state-def / @if blocks  (tagged union)
// ---------------------------------------------------------------------------

#[napi(object)]
#[derive(Debug, Clone)]
pub struct StateDefItem {
  /// Discriminant: "declaration" | "qualified-rule" | "if-chain" | "at-rule"
  pub kind: String,

  pub declaration: Option<CssDeclaration>,
  pub qualified_rule: Option<CssQualifiedRule>,
  pub if_chain: Option<IfChain>,
  pub at_rule: Option<CssRawAtRule>,
}

impl StateDefItem {
  pub fn declaration(d: CssDeclaration) -> Self {
    Self {
      kind: "declaration".into(),
      declaration: Some(d),
      qualified_rule: None,
      if_chain: None,
      at_rule: None,
    }
  }

  pub fn qualified(q: CssQualifiedRule) -> Self {
    Self {
      kind: "qualified-rule".into(),
      declaration: None,
      qualified_rule: Some(q),
      if_chain: None,
      at_rule: None,
    }
  }

  pub fn if_chain(c: IfChain) -> Self {
    Self {
      kind: "if-chain".into(),
      declaration: None,
      qualified_rule: None,
      if_chain: Some(c),
      at_rule: None,
    }
  }

  pub fn at_rule(r: CssRawAtRule) -> Self {
    Self {
      kind: "at-rule".into(),
      declaration: None,
      qualified_rule: None,
      if_chain: None,
      at_rule: Some(r),
    }
  }
}

// ---------------------------------------------------------------------------
// CSS primitives
// ---------------------------------------------------------------------------

#[napi(object)]
#[derive(Debug, Clone)]
pub struct CssDeclaration {
  pub property: String,
  pub value: String,
  pub important: bool,
  pub span: Span,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct CssQualifiedRule {
  /// Raw selector text, e.g. "&:hover", "span", ".class > div"
  pub selector: String,
  pub body: Vec<StateDefItem>,
  pub span: Span,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct CssRawAtRule {
  pub name: String,
  pub prelude: String,
  pub block: Option<String>,
  pub span: Span,
}

// ---------------------------------------------------------------------------
// @if / @elseif / @else
// ---------------------------------------------------------------------------

#[napi(object)]
#[derive(Debug, Clone)]
pub struct IfChain {
  pub if_clause: IfClause,
  pub else_if_clauses: Vec<IfClause>,
  pub else_body: Option<Vec<StateDefItem>>,
  pub span: Span,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct IfClause {
  /// JSON-serialized `ConditionExpr`. TypeScript sees `unknown`; use a type-guard to narrow.
  pub condition: serde_json::Value,
  pub body: Vec<StateDefItem>,
  pub span: Span,
}

// ---------------------------------------------------------------------------
// Condition expressions (not exposed directly via napi — serialized to JSON)
//
// Shape of the JSON:
//   { kind: "var",        var: "--name" }
//   { kind: "comparison", left: "--name", op: "==" | "!=", right: { kind, value } }
//   { kind: "and",        left: <ConditionExpr>, right: <ConditionExpr> }
//   { kind: "or",         left: <ConditionExpr>, right: <ConditionExpr> }
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConditionExpr {
  #[serde(rename = "var")]
  Var { var: String },

  #[serde(rename = "comparison")]
  Comparison {
    left: String,
    op: String,
    right: ConditionValue,
  },

  #[serde(rename = "and")]
  And {
    left: Box<ConditionExpr>,
    right: Box<ConditionExpr>,
  },

  #[serde(rename = "or")]
  Or {
    left: Box<ConditionExpr>,
    right: Box<ConditionExpr>,
  },
}

impl ConditionExpr {
  pub fn var(name: String) -> Self {
    Self::Var { var: name }
  }

  pub fn comparison(left: String, op: String, right: ConditionValue) -> Self {
    Self::Comparison { left, op, right }
  }

  pub fn and(left: ConditionExpr, right: ConditionExpr) -> Self {
    Self::And {
      left: Box::new(left),
      right: Box::new(right),
    }
  }

  pub fn or(left: ConditionExpr, right: ConditionExpr) -> Self {
    Self::Or {
      left: Box::new(left),
      right: Box::new(right),
    }
  }

  /// # Panics
  ///
  /// Panics if `ConditionExpr` fails to serialize, which should never happen
  /// since all its fields are JSON-compatible primitives.
  pub fn to_json(&self) -> serde_json::Value {
    serde_json::to_value(self).expect("ConditionExpr is always serializable")
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionValue {
  /// "string" | "boolean" | "ident"
  pub kind: String,
  pub value: String,
}

impl ConditionValue {
  pub fn string(s: String) -> Self {
    Self {
      kind: "string".into(),
      value: s,
    }
  }

  pub fn boolean(b: bool) -> Self {
    Self {
      kind: "boolean".into(),
      value: if b { "true".into() } else { "false".into() },
    }
  }

  pub fn ident(s: String) -> Self {
    Self {
      kind: "ident".into(),
      value: s,
    }
  }
}
