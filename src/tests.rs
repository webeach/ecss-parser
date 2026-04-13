//! Unit tests for the ECSS parser.

use crate::parser::parse;

// ---------------------------------------------------------------------------
// @state-variant
// ---------------------------------------------------------------------------

#[test]
fn parses_state_variant_basic() {
  let src = r#"
    @state-variant Theme {
      values: light, dark;
    }
  "#;
  let ast = parse(src).expect("should parse");
  assert_eq!(ast.rules.len(), 1);
  let rule = &ast.rules[0];
  assert_eq!(rule.kind, "state-variant");
  let sv = rule.state_variant.as_ref().unwrap();
  assert_eq!(sv.name, "Theme");
  assert_eq!(sv.values, vec!["light", "dark"]);
}

#[test]
fn parses_state_variant_quoted_values() {
  let src = r#"
    @state-variant Size {
      values: small, "extra large", medium;
    }
  "#;
  let ast = parse(src).expect("should parse");
  let sv = ast.rules[0].state_variant.as_ref().unwrap();
  assert_eq!(sv.values, vec!["small", "extra large", "medium"]);
}

// ---------------------------------------------------------------------------
// @state-def
// ---------------------------------------------------------------------------

#[test]
fn parses_state_def_no_params_no_braces() {
  let src = r#"
    @state-def Card {
      background: red;
    }
  "#;
  let ast = parse(src).expect("should parse");
  assert_eq!(ast.rules[0].kind, "state-def");
  let sd = ast.rules[0].state_def.as_ref().unwrap();
  assert_eq!(sd.name, "Card");
  assert!(sd.params.is_empty());
  assert_eq!(sd.body.len(), 1);
  let decl = sd.body[0].declaration.as_ref().unwrap();
  assert_eq!(decl.property, "background");
  assert_eq!(decl.value, "red");
  assert!(!decl.important);
}

#[test]
fn parses_state_def_empty_parens() {
  let src = r#"
    @state-def Card() {
      color: blue;
    }
  "#;
  let ast = parse(src).expect("should parse");
  let sd = ast.rules[0].state_def.as_ref().unwrap();
  assert!(sd.params.is_empty());
}

#[test]
fn parses_state_def_boolean_param() {
  let src = r#"
    @state-def Card(--is-active) {
      background: green;
    }
  "#;
  let ast = parse(src).expect("should parse");
  let sd = ast.rules[0].state_def.as_ref().unwrap();
  assert_eq!(sd.params.len(), 1);
  let p = &sd.params[0];
  assert_eq!(p.name, "--is-active");
  assert_eq!(p.param_type, "boolean");
  assert!(p.variant_name.is_none());
  assert!(p.default_value.is_none());
}

#[test]
fn parses_state_def_boolean_param_explicit_default() {
  let src = r#"
    @state-def Card(--is-active boolean: true) {
      background: green;
    }
  "#;
  let ast = parse(src).expect("should parse");
  let p = &ast.rules[0].state_def.as_ref().unwrap().params[0];
  assert_eq!(p.param_type, "boolean");
  assert_eq!(p.default_value.as_deref(), Some("true"));
}

#[test]
fn parses_state_def_variant_param() {
  let src = r#"
    @state-def Card(--theme Theme: "light") {
      background: white;
    }
  "#;
  let ast = parse(src).expect("should parse");
  let p = &ast.rules[0].state_def.as_ref().unwrap().params[0];
  assert_eq!(p.name, "--theme");
  assert_eq!(p.param_type, "variant");
  assert_eq!(p.variant_name.as_deref(), Some("Theme"));
  assert_eq!(p.default_value.as_deref(), Some("light"));
}

#[test]
fn parses_state_def_multiple_params() {
  let src = r#"
    @state-def Card(--is-active, --theme Theme: "dark") {
      background: black;
    }
  "#;
  let ast = parse(src).expect("should parse");
  let params = &ast.rules[0].state_def.as_ref().unwrap().params;
  assert_eq!(params.len(), 2);
  assert_eq!(params[0].name, "--is-active");
  assert_eq!(params[1].param_type, "variant");
}

// ---------------------------------------------------------------------------
// @if / @elseif / @else
// ---------------------------------------------------------------------------

#[test]
fn parses_if_var_condition() {
  let src = r#"
    @state-def Card(--is-active) {
      @if (--is-active) {
        color: red;
      }
    }
  "#;
  let ast = parse(src).expect("should parse");
  let body = &ast.rules[0].state_def.as_ref().unwrap().body;
  assert_eq!(body.len(), 1);
  assert_eq!(body[0].kind, "if-chain");
  let chain = body[0].if_chain.as_ref().unwrap();
  assert!(chain.else_if_clauses.is_empty());
  assert!(chain.else_body.is_none());

  let cond = &chain.if_clause.condition;
  assert_eq!(cond["kind"], "var");
  assert_eq!(cond["var"], "--is-active");
}

#[test]
fn parses_if_comparison_condition() {
  let src = r#"
    @state-def Card(--theme Theme) {
      @if (--theme == "dark") {
        background: black;
      }
    }
  "#;
  let ast = parse(src).expect("should parse");
  let chain = ast.rules[0].state_def.as_ref().unwrap().body[0]
    .if_chain
    .as_ref()
    .unwrap();
  let cond = &chain.if_clause.condition;
  assert_eq!(cond["kind"], "comparison");
  assert_eq!(cond["left"], "--theme");
  assert_eq!(cond["op"], "==");
  assert_eq!(cond["right"]["kind"], "string");
  assert_eq!(cond["right"]["value"], "dark");
}

#[test]
fn parses_if_and_condition() {
  let src = r#"
    @state-def Card(--is-active, --theme Theme) {
      @if (--is-active && --theme == "dark") {
        color: white;
      }
    }
  "#;
  let ast = parse(src).expect("should parse");
  let chain = ast.rules[0].state_def.as_ref().unwrap().body[0]
    .if_chain
    .as_ref()
    .unwrap();
  let cond = &chain.if_clause.condition;
  assert_eq!(cond["kind"], "and");
  assert_eq!(cond["left"]["kind"], "var");
  assert_eq!(cond["right"]["kind"], "comparison");
}

#[test]
fn parses_if_elseif_else_chain() {
  let src = r#"
    @state-def Card(--theme Theme) {
      @if (--theme == "light") {
        background: white;
      }
      @elseif (--theme == "dark") {
        background: black;
      }
      @else {
        background: gray;
      }
    }
  "#;
  let ast = parse(src).expect("should parse");
  let chain = ast.rules[0].state_def.as_ref().unwrap().body[0]
    .if_chain
    .as_ref()
    .unwrap();
  assert_eq!(chain.else_if_clauses.len(), 1);
  assert!(chain.else_body.is_some());
  let else_body = chain.else_body.as_ref().unwrap();
  assert_eq!(else_body.len(), 1);
  assert_eq!(else_body[0].declaration.as_ref().unwrap().value, "gray");
}

// ---------------------------------------------------------------------------
// Nested selectors
// ---------------------------------------------------------------------------

#[test]
fn parses_nested_selector_in_state_def() {
  let src = r#"
    @state-def Card(--is-active) {
      padding: 16px;

      &:hover {
        background: green;

        @if (--is-active) {
          background: yellow;
        }
      }
    }
  "#;
  let ast = parse(src).expect("should parse");
  let body = &ast.rules[0].state_def.as_ref().unwrap().body;
  assert_eq!(body.len(), 2);
  assert_eq!(body[0].kind, "declaration");
  assert_eq!(body[1].kind, "qualified-rule");
  let nested = body[1].qualified_rule.as_ref().unwrap();
  assert!(nested.selector.contains("hover"));
  assert_eq!(nested.body.len(), 2);
  assert_eq!(nested.body[1].kind, "if-chain");
}

// ---------------------------------------------------------------------------
// !important
// ---------------------------------------------------------------------------

#[test]
fn parses_important_declaration() {
  let src = r#"
    @state-def Card {
      background: red !important;
    }
  "#;
  let ast = parse(src).expect("should parse");
  let decl = ast.rules[0].state_def.as_ref().unwrap().body[0]
    .declaration
    .as_ref()
    .unwrap();
  assert!(decl.important);
  assert_eq!(decl.value, "red");
}

// ---------------------------------------------------------------------------
// Multiple rules in one file
// ---------------------------------------------------------------------------

#[test]
fn parses_multiple_top_level_rules() {
  let src = r#"
    @state-variant Theme { values: light, dark; }
    @state-def Card { background: white; }
  "#;
  let ast = parse(src).expect("should parse");
  assert_eq!(ast.rules.len(), 2);
  assert_eq!(ast.rules[0].kind, "state-variant");
  assert_eq!(ast.rules[1].kind, "state-def");
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[test]
fn returns_error_on_missing_variant_name() {
  let src = r#"
    @state-variant {
      values: a, b;
    }
  "#;
  assert!(parse(src).is_err());
}
