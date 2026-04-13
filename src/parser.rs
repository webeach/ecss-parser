//! Core ECSS parser.

use cssparser::{
  AtRuleParser, BasicParseErrorKind, CowRcStr, DeclarationParser,
  ParseError as CssParseError, ParseErrorKind, Parser, ParserInput,
  QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser,
  Token,
};

use crate::ast::{
  CssDeclaration, CssQualifiedRule, CssRawAtRule, EcssRule, EcssStylesheet,
  IfChain, IfClause, Span, StateDef, StateDefItem, StateParam, StateVariant,
};
use crate::condition::parse_condition;
use crate::error::{EcssError, ParseError};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn parse(source: &str) -> Result<EcssStylesheet, ParseError> {
  let mut input = ParserInput::new(source);
  let mut parser = Parser::new(&mut input);
  let mut top_level = TopLevelParser;
  let iter = StyleSheetParser::new(&mut parser, &mut top_level);

  let mut rules = Vec::new();
  for result in iter {
    match result {
      Ok(rule) => rules.push(rule),
      Err((err, _)) => {
        let loc = err.location;
        return Err(ParseError::new(
          format_error(&err.kind),
          loc.line,
          loc.column,
        ));
      }
    }
  }

  Ok(EcssStylesheet { rules })
}

fn format_error(kind: &ParseErrorKind<EcssError>) -> String {
  match kind {
    ParseErrorKind::Basic(b) => match b {
      BasicParseErrorKind::UnexpectedToken(t) => {
        format!("Unexpected token: {t:?}")
      }
      BasicParseErrorKind::EndOfInput => "Unexpected end of input".into(),
      BasicParseErrorKind::AtRuleBodyInvalid => "Invalid at-rule body".into(),
      BasicParseErrorKind::AtRuleInvalid(n) => format!("Unknown at-rule: @{n}"),
      BasicParseErrorKind::QualifiedRuleInvalid => {
        "Invalid qualified rule".into()
      }
    },
    ParseErrorKind::Custom(e) => e.to_string(),
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_span(
  start: cssparser::SourceLocation,
  end: cssparser::SourceLocation,
) -> Span {
  Span {
    line: start.line,
    column: start.column,
    end_line: end.line,
    end_column: end.column,
  }
}

/// Collect remaining tokens as a raw string (for selectors and unknown preludes).
fn collect_raw(input: &mut Parser<'_, '_>) -> String {
  let mut out = String::new();
  while let Ok(tok) = input.next_including_whitespace_and_comments() {
    append_token(&mut out, tok);
  }
  out.trim().to_owned()
}

fn append_token(out: &mut String, tok: &Token<'_>) {
  match tok {
    Token::Ident(s) => out.push_str(s),
    Token::AtKeyword(s) => {
      out.push('@');
      out.push_str(s);
    }
    Token::Hash(s) | Token::IDHash(s) => {
      out.push('#');
      out.push_str(s);
    }
    Token::QuotedString(s) => {
      out.push('"');
      out.push_str(s);
      out.push('"');
    }
    Token::UnquotedUrl(s) => {
      out.push_str("url(");
      out.push_str(s);
      out.push(')');
    }
    Token::Delim(c) => out.push(*c),
    Token::Number { value, .. } => out.push_str(&value.to_string()),
    Token::Percentage { unit_value, .. } => {
      out.push_str(&(unit_value * 100.0).to_string());
      out.push('%');
    }
    Token::Dimension { value, unit, .. } => {
      out.push_str(&value.to_string());
      out.push_str(unit);
    }
    Token::WhiteSpace(_) => out.push(' '),
    Token::Comment(_) | Token::BadString(_) | Token::BadUrl(_) => {}
    Token::Colon => out.push(':'),
    Token::Semicolon => out.push(';'),
    Token::Comma => out.push(','),
    Token::IncludeMatch => out.push_str("~="),
    Token::DashMatch => out.push_str("|="),
    Token::PrefixMatch => out.push_str("^="),
    Token::SuffixMatch => out.push_str("$="),
    Token::SubstringMatch => out.push_str("*="),
    Token::CDO => out.push_str("<!--"),
    Token::CDC => out.push_str("-->"),
    Token::Function(name) => {
      out.push_str(name);
      out.push('(');
    }
    Token::ParenthesisBlock => out.push('('),
    Token::SquareBracketBlock => out.push('['),
    Token::CurlyBracketBlock => out.push('{'),
    Token::CloseParenthesis => out.push(')'),
    Token::CloseSquareBracket => out.push(']'),
    Token::CloseCurlyBracket => out.push('}'),
  }
}

// ---------------------------------------------------------------------------
// Prelude types (no lifetimes — all data owned)
// ---------------------------------------------------------------------------

enum AtPrelude {
  StateVariant {
    name: String,
    loc: cssparser::SourceLocation,
  },
  StateDef {
    name: String,
    params: Vec<StateParam>,
    loc: cssparser::SourceLocation,
  },
  Unknown {
    name: String,
    prelude: String,
    loc: cssparser::SourceLocation,
  },
}

enum BodyAtPrelude {
  If {
    condition: crate::ast::ConditionExpr,
    loc: cssparser::SourceLocation,
  },
  ElseIf {
    condition: crate::ast::ConditionExpr,
    loc: cssparser::SourceLocation,
  },
  Else {
    _loc: cssparser::SourceLocation,
  },
  Unknown {
    name: String,
    prelude: String,
    loc: cssparser::SourceLocation,
  },
}

// ---------------------------------------------------------------------------
// Helper: parse an ident and own the string, with a custom error fallback
// ---------------------------------------------------------------------------

fn parse_ident_owned<'i>(
  input: &mut Parser<'i, '_>,
  err: EcssError,
) -> Result<String, CssParseError<'i, EcssError>> {
  // .map() converts CowRcStr<'i> to String, ending the borrow.
  // Then map_err can borrow input again.
  let result = input.expect_ident().map(ToString::to_string);
  result.map_err(|_| input.new_custom_error(err))
}

// ---------------------------------------------------------------------------
// Top-level parser
// ---------------------------------------------------------------------------

struct TopLevelParser;

impl<'i> AtRuleParser<'i> for TopLevelParser {
  type Prelude = AtPrelude;
  type AtRule = EcssRule;
  type Error = EcssError;

  fn parse_prelude<'t>(
    &mut self,
    name: CowRcStr<'i>,
    input: &mut Parser<'i, 't>,
  ) -> Result<AtPrelude, CssParseError<'i, EcssError>> {
    let loc = input.current_source_location();
    match name.as_ref() {
      "state-variant" => {
        let n = parse_ident_owned(input, EcssError::ExpectedIdent)?;
        Ok(AtPrelude::StateVariant { name: n, loc })
      }
      "state-def" => {
        // In CSS tokenization, `Name(` is a Function token, not Ident + ParenthesisBlock.
        // `Name` alone (without `(`) is an Ident token.
        let tok = input.next()?.clone();
        let (n, params) = match tok {
          Token::Ident(name) => {
            // @state-def Name { ... }  — no params
            (name.to_string(), Vec::new())
          }
          Token::Function(name) => {
            // @state-def Name(...) { ... }
            let params = input.parse_nested_block(parse_state_def_params)?;
            (name.to_string(), params)
          }
          other => {
            return Err(
              input.new_error(BasicParseErrorKind::UnexpectedToken(other)),
            );
          }
        };
        Ok(AtPrelude::StateDef {
          name: n,
          params,
          loc,
        })
      }
      _ => {
        let prelude = collect_raw(input);
        Ok(AtPrelude::Unknown {
          name: name.to_string(),
          prelude,
          loc,
        })
      }
    }
  }

  fn parse_block<'t>(
    &mut self,
    prelude: AtPrelude,
    _start: &cssparser::ParserState,
    input: &mut Parser<'i, 't>,
  ) -> Result<EcssRule, CssParseError<'i, EcssError>> {
    match prelude {
      AtPrelude::StateVariant { name, loc } => {
        let values = parse_variant_block(input)?;
        let end = input.current_source_location();
        Ok(EcssRule::variant(StateVariant {
          name,
          values,
          span: make_span(loc, end),
        }))
      }
      AtPrelude::StateDef { name, params, loc } => {
        let body = parse_rule_body(input)?;
        let end = input.current_source_location();
        Ok(EcssRule::state_def(StateDef {
          name,
          params,
          body,
          span: make_span(loc, end),
        }))
      }
      AtPrelude::Unknown { name, prelude, loc } => {
        let block = collect_raw(input);
        let end = input.current_source_location();
        Ok(EcssRule::at_rule(CssRawAtRule {
          name,
          prelude,
          block: Some(block),
          span: make_span(loc, end),
        }))
      }
    }
  }

  fn rule_without_block(
    &mut self,
    prelude: AtPrelude,
    _start: &cssparser::ParserState,
  ) -> Result<EcssRule, ()> {
    match prelude {
      AtPrelude::Unknown { name, prelude, loc } => {
        Ok(EcssRule::at_rule(CssRawAtRule {
          name,
          prelude,
          block: None,
          span: make_span(loc, loc),
        }))
      }
      _ => Err(()),
    }
  }
}

impl<'i> QualifiedRuleParser<'i> for TopLevelParser {
  type Prelude = (String, cssparser::SourceLocation);
  type QualifiedRule = EcssRule;
  type Error = EcssError;

  fn parse_prelude<'t>(
    &mut self,
    input: &mut Parser<'i, 't>,
  ) -> Result<(String, cssparser::SourceLocation), CssParseError<'i, EcssError>>
  {
    let loc = input.current_source_location();
    Ok((collect_raw(input), loc))
  }

  fn parse_block<'t>(
    &mut self,
    prelude: (String, cssparser::SourceLocation),
    _start: &cssparser::ParserState,
    input: &mut Parser<'i, 't>,
  ) -> Result<EcssRule, CssParseError<'i, EcssError>> {
    let (selector, loc) = prelude;
    let body = parse_rule_body(input)?;
    let end = input.current_source_location();
    Ok(EcssRule::qualified(CssQualifiedRule {
      selector,
      body,
      span: make_span(loc, end),
    }))
  }
}

// ---------------------------------------------------------------------------
// @state-variant block parser
// ---------------------------------------------------------------------------

fn parse_variant_block<'i>(
  input: &mut Parser<'i, '_>,
) -> Result<Vec<String>, CssParseError<'i, EcssError>> {
  let kw = parse_ident_owned(input, EcssError::ExpectedValues)?;
  if kw != "values" {
    return Err(input.new_custom_error(EcssError::ExpectedValues));
  }
  input
    .expect_colon()
    .map_err(|_| input.new_custom_error(EcssError::ExpectedValues))?;

  let mut values = Vec::new();
  loop {
    input.skip_whitespace();
    if input.is_exhausted() {
      break;
    }
    if input.try_parse(Parser::expect_semicolon).is_ok() {
      break;
    }
    let tok = input.next()?.clone();
    let val = match tok {
      Token::Ident(s) | Token::QuotedString(s) => s.to_string(),
      other => {
        return Err(
          input.new_error(BasicParseErrorKind::UnexpectedToken(other)),
        );
      }
    };
    values.push(val);
    input.skip_whitespace();
    let _ = input.try_parse(Parser::expect_comma);
  }

  Ok(values)
}

// ---------------------------------------------------------------------------
// @state-def parameter list parser
// ---------------------------------------------------------------------------

fn parse_state_def_params<'i>(
  input: &mut Parser<'i, '_>,
) -> Result<Vec<StateParam>, CssParseError<'i, EcssError>> {
  let mut params = Vec::new();

  loop {
    input.skip_whitespace();
    if input.is_exhausted() {
      break;
    }

    // Parameter name must start with "--"
    let tok = input.next()?.clone();
    let param_name = match tok {
      Token::Ident(s) if s.starts_with("--") => s.to_string(),
      other => {
        return Err(
          input.new_error(BasicParseErrorKind::UnexpectedToken(other)),
        );
      }
    };

    // Optional type/variant: an ident not starting with "--"
    let type_ident: Option<String> = {
      let checkpoint = input.state();
      input.skip_whitespace();
      let result = input.try_parse(|i| {
        i.skip_whitespace();
        let t = i.next()?.clone();
        match t {
          Token::Ident(s) if !s.starts_with("--") => Ok(s.to_string()),
          other => Err(i.new_error::<EcssError>(
            BasicParseErrorKind::UnexpectedToken(other),
          )),
        }
      });
      if let Ok(s) = result {
        Some(s)
      } else {
        input.reset(&checkpoint);
        None
      }
    };

    // Optional default value after ":"
    let default_value: Option<String> = {
      let checkpoint = input.state();
      input.skip_whitespace();
      let result: Result<String, _> = input.try_parse(|i| {
        i.skip_whitespace();
        let colon = i.next()?.clone();
        match colon {
          Token::Colon => {}
          other => {
            return Err(i.new_error::<EcssError>(
              BasicParseErrorKind::UnexpectedToken(other),
            ));
          }
        }
        i.skip_whitespace();
        let tok = i.next()?.clone();
        match tok {
          Token::Ident(s) | Token::QuotedString(s) => Ok(s.to_string()),
          other => Err(i.new_error::<EcssError>(
            BasicParseErrorKind::UnexpectedToken(other),
          )),
        }
      });
      if let Ok(s) = result {
        Some(s)
      } else {
        input.reset(&checkpoint);
        None
      }
    };

    let (param_type, variant_name): (String, Option<String>) =
      match type_ident.as_deref() {
        None | Some("boolean") => ("boolean".to_owned(), None),
        Some(name) => ("variant".to_owned(), Some(name.to_owned())),
      };

    params.push(StateParam {
      name: param_name,
      param_type,
      variant_name,
      default_value,
    });

    input.skip_whitespace();
    if input.try_parse(Parser::expect_comma).is_err() {
      break;
    }
  }

  Ok(params)
}

// ---------------------------------------------------------------------------
// Rule body parser
// ---------------------------------------------------------------------------

fn parse_rule_body<'i>(
  input: &mut Parser<'i, '_>,
) -> Result<Vec<StateDefItem>, CssParseError<'i, EcssError>> {
  let mut items: Vec<StateDefItem> = Vec::new();
  {
    let mut body_parser = BodyParser { items: &mut items };
    let rule_parser = RuleBodyParser::new(input, &mut body_parser);
    for result in rule_parser {
      match result {
        Ok(()) => {}
        Err((err, _)) => return Err(err),
      }
    }
  }
  Ok(items)
}

// ---------------------------------------------------------------------------
// BodyParser
// ---------------------------------------------------------------------------

struct BodyParser<'a> {
  items: &'a mut Vec<StateDefItem>,
}

impl<'i> DeclarationParser<'i> for BodyParser<'_> {
  type Declaration = ();
  type Error = EcssError;

  fn parse_value<'t>(
    &mut self,
    name: CowRcStr<'i>,
    input: &mut Parser<'i, 't>,
    _state: &cssparser::ParserState,
  ) -> Result<(), CssParseError<'i, EcssError>> {
    let loc = input.current_source_location();
    let mut raw = String::new();

    while !input.is_exhausted() {
      match input.next_including_whitespace() {
        Ok(tok) => append_token(&mut raw, tok),
        Err(_) => break,
      }
    }

    let end = input.current_source_location();
    let raw = raw.trim();
    let value_lower = raw.to_lowercase();
    let (value, important) =
      if let Some(v) = strip_important_suffix(raw, &value_lower) {
        (v, true)
      } else {
        (raw.to_owned(), false)
      };

    self.items.push(StateDefItem::declaration(CssDeclaration {
      property: name.to_string(),
      value,
      important,
      span: make_span(loc, end),
    }));
    Ok(())
  }
}

fn strip_important_suffix(_raw: &str, raw_lower: &str) -> Option<String> {
  // Handles: "red !important" or "red!important"
  if let Some(rest) = raw_lower.strip_suffix("important") {
    let rest = rest.trim_end();
    if let Some(rest2) = rest.strip_suffix('!') {
      return Some(rest2.trim_end().to_owned());
    }
  }
  None
}

impl<'i> QualifiedRuleParser<'i> for BodyParser<'_> {
  type Prelude = (String, cssparser::SourceLocation);
  type QualifiedRule = ();
  type Error = EcssError;

  fn parse_prelude<'t>(
    &mut self,
    input: &mut Parser<'i, 't>,
  ) -> Result<(String, cssparser::SourceLocation), CssParseError<'i, EcssError>>
  {
    let loc = input.current_source_location();
    Ok((collect_raw(input), loc))
  }

  fn parse_block<'t>(
    &mut self,
    prelude: (String, cssparser::SourceLocation),
    _start: &cssparser::ParserState,
    input: &mut Parser<'i, 't>,
  ) -> Result<(), CssParseError<'i, EcssError>> {
    let (selector, loc) = prelude;
    let body = parse_rule_body(input)?;
    let end = input.current_source_location();
    self.items.push(StateDefItem::qualified(CssQualifiedRule {
      selector,
      body,
      span: make_span(loc, end),
    }));
    Ok(())
  }
}

impl<'i> AtRuleParser<'i> for BodyParser<'_> {
  type Prelude = BodyAtPrelude;
  type AtRule = ();
  type Error = EcssError;

  fn parse_prelude<'t>(
    &mut self,
    name: CowRcStr<'i>,
    input: &mut Parser<'i, 't>,
  ) -> Result<BodyAtPrelude, CssParseError<'i, EcssError>> {
    let loc = input.current_source_location();
    match name.as_ref() {
      "if" => {
        // Prelude is `(condition)` — consume ParenthesisBlock first, then parse nested.
        input.expect_parenthesis_block()?;
        let condition = input.parse_nested_block(parse_condition)?;
        Ok(BodyAtPrelude::If { condition, loc })
      }
      "elseif" => {
        input.expect_parenthesis_block()?;
        let condition = input.parse_nested_block(parse_condition)?;
        Ok(BodyAtPrelude::ElseIf { condition, loc })
      }
      "else" => Ok(BodyAtPrelude::Else { _loc: loc }),
      _ => {
        let prelude = collect_raw(input);
        Ok(BodyAtPrelude::Unknown {
          name: name.to_string(),
          prelude,
          loc,
        })
      }
    }
  }

  fn parse_block<'t>(
    &mut self,
    prelude: BodyAtPrelude,
    _start: &cssparser::ParserState,
    input: &mut Parser<'i, 't>,
  ) -> Result<(), CssParseError<'i, EcssError>> {
    match prelude {
      BodyAtPrelude::If { condition, loc } => {
        let body = parse_rule_body(input)?;
        let end = input.current_source_location();
        self.items.push(StateDefItem::if_chain(IfChain {
          if_clause: IfClause {
            condition: condition.to_json(),
            body,
            span: make_span(loc, end),
          },
          else_if_clauses: Vec::new(),
          else_body: None,
          span: make_span(loc, end),
        }));
      }
      BodyAtPrelude::ElseIf { condition, loc } => {
        let body = parse_rule_body(input)?;
        let end = input.current_source_location();
        match self.items.last_mut().and_then(|i| i.if_chain.as_mut()) {
          Some(chain) if chain.else_body.is_none() => {
            chain.span.end_line = end.line;
            chain.span.end_column = end.column;
            chain.else_if_clauses.push(IfClause {
              condition: condition.to_json(),
              body,
              span: make_span(loc, end),
            });
          }
          Some(_) => {
            return Err(input.new_custom_error(EcssError::Custom(
              "@elseif after @else".into(),
            )));
          }
          None => {
            return Err(input.new_custom_error(EcssError::UnexpectedElse));
          }
        }
      }
      BodyAtPrelude::Else { _loc: _ } => {
        let body = parse_rule_body(input)?;
        let end = input.current_source_location();
        match self.items.last_mut().and_then(|i| i.if_chain.as_mut()) {
          Some(chain) if chain.else_body.is_none() => {
            chain.span.end_line = end.line;
            chain.span.end_column = end.column;
            chain.else_body = Some(body);
          }
          Some(_) => {
            return Err(
              input
                .new_custom_error(EcssError::Custom("Duplicate @else".into())),
            );
          }
          None => {
            return Err(input.new_custom_error(EcssError::UnexpectedElse));
          }
        }
      }
      BodyAtPrelude::Unknown { name, prelude, loc } => {
        let block = collect_raw(input);
        let end = input.current_source_location();
        self.items.push(StateDefItem::at_rule(CssRawAtRule {
          name,
          prelude,
          block: Some(block),
          span: make_span(loc, end),
        }));
      }
    }
    Ok(())
  }

  fn rule_without_block(
    &mut self,
    prelude: BodyAtPrelude,
    _start: &cssparser::ParserState,
  ) -> Result<(), ()> {
    match prelude {
      BodyAtPrelude::Unknown { name, prelude, loc } => {
        self.items.push(StateDefItem::at_rule(CssRawAtRule {
          name,
          prelude,
          block: None,
          span: make_span(loc, loc),
        }));
        Ok(())
      }
      _ => Err(()),
    }
  }
}

impl RuleBodyItemParser<'_, (), EcssError> for BodyParser<'_> {
  fn parse_qualified(&self) -> bool {
    true
  }
  fn parse_declarations(&self) -> bool {
    true
  }
}
