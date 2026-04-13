//! Recursive-descent parser for ECSS @if conditions.
//!
//! Grammar (JS-style operator precedence):
//! ```text
//!   expression   = or_expr
//!   or_expr      = and_expr ("||" and_expr)*
//!   and_expr     = compare_expr ("&&" compare_expr)*
//!   compare_expr = primary (("==" | "!=") value)?
//!   primary      = "(" expression ")" | variable
//!   variable     = "--" ident      (implicit == true when used alone)
//!   value        = quoted_string | "true" | "false" | ident
//! ```

use cssparser::{ParseError as CssParseError, Parser, Token};

use crate::ast::{ConditionExpr, ConditionValue};
use crate::error::EcssError;

type CondResult<'i, T> = std::result::Result<T, CssParseError<'i, EcssError>>;

pub fn parse_condition<'i>(
  input: &mut Parser<'i, '_>,
) -> CondResult<'i, ConditionExpr> {
  parse_or_expr(input)
}

fn parse_or_expr<'i>(
  input: &mut Parser<'i, '_>,
) -> CondResult<'i, ConditionExpr> {
  let mut left = parse_and_expr(input)?;

  loop {
    let checkpoint = input.state();
    if try_consume_double_delim(input, '|', '|') {
      let right = parse_and_expr(input)?;
      left = ConditionExpr::or(left, right);
    } else {
      input.reset(&checkpoint);
      break;
    }
  }

  Ok(left)
}

fn parse_and_expr<'i>(
  input: &mut Parser<'i, '_>,
) -> CondResult<'i, ConditionExpr> {
  let mut left = parse_compare_expr(input)?;

  loop {
    let checkpoint = input.state();
    if try_consume_double_delim(input, '&', '&') {
      let right = parse_compare_expr(input)?;
      left = ConditionExpr::and(left, right);
    } else {
      input.reset(&checkpoint);
      break;
    }
  }

  Ok(left)
}

fn parse_compare_expr<'i>(
  input: &mut Parser<'i, '_>,
) -> CondResult<'i, ConditionExpr> {
  let primary = parse_primary(input)?;

  // Try to read comparison operator
  let op: Option<String> = {
    let checkpoint = input.state();
    let result = input.try_parse(|i| -> CondResult<'_, String> {
      let first = i.next()?.clone();
      let second = i.next()?.clone();
      match (&first, &second) {
        (Token::Delim('='), Token::Delim('=')) => Ok("==".to_string()),
        (Token::Delim('!'), Token::Delim('=')) => Ok("!=".to_string()),
        _ => Err(i.new_custom_error(EcssError::InvalidCondition(
          "expected == or !=".into(),
        ))),
      }
    });
    if let Ok(op) = result {
      Some(op)
    } else {
      input.reset(&checkpoint);
      None
    }
  };

  match op {
    Some(op_str) => {
      let var_name = match &primary {
        ConditionExpr::Var { var } => var.clone(),
        _ => {
          return Err(input.new_custom_error(EcssError::InvalidCondition(
            "Left side of comparison must be a variable".into(),
          )));
        }
      };

      let rhs = parse_value(input)?;
      Ok(ConditionExpr::comparison(var_name, op_str, rhs))
    }
    None => Ok(primary),
  }
}

fn parse_primary<'i>(
  input: &mut Parser<'i, '_>,
) -> CondResult<'i, ConditionExpr> {
  // Grouped: ( expr )
  let checkpoint = input.state();
  let grouped = input.try_parse(|i| {
    i.expect_parenthesis_block()?;
    i.parse_nested_block(parse_or_expr)
  });
  if let Ok(expr) = grouped {
    return Ok(expr);
  }
  input.reset(&checkpoint);

  parse_variable(input)
}

fn parse_variable<'i>(
  input: &mut Parser<'i, '_>,
) -> CondResult<'i, ConditionExpr> {
  let tok = input.next()?.clone();
  match tok {
    Token::Ident(ref name) if name.starts_with("--") => {
      Ok(ConditionExpr::var(name.to_string()))
    }
    other => Err(
      input.new_error(cssparser::BasicParseErrorKind::UnexpectedToken(other)),
    ),
  }
}

fn parse_value<'i>(
  input: &mut Parser<'i, '_>,
) -> CondResult<'i, ConditionValue> {
  let tok = input.next()?.clone();
  match tok {
    Token::QuotedString(s) => Ok(ConditionValue::string(s.to_string())),
    Token::Ident(s) => match s.as_ref() {
      "true" => Ok(ConditionValue::boolean(true)),
      "false" => Ok(ConditionValue::boolean(false)),
      other => Ok(ConditionValue::ident(other.to_string())),
    },
    other => Err(
      input.new_error(cssparser::BasicParseErrorKind::UnexpectedToken(other)),
    ),
  }
}

/// Try to consume two consecutive `Delim` tokens. Returns true on success,
/// false if not matched (caller must reset the parser state).
fn try_consume_double_delim(
  input: &mut Parser<'_, '_>,
  a: char,
  b: char,
) -> bool {
  let checkpoint = input.state();
  let ok = input
    .try_parse(|i| -> CondResult<'_, ()> {
      let first = i.next()?.clone();
      match &first {
        Token::Delim(c) if *c == a => {}
        other => {
          return Err(i.new_error(
            cssparser::BasicParseErrorKind::UnexpectedToken(other.clone()),
          ));
        }
      }
      let second = i.next()?.clone();
      match &second {
        Token::Delim(c) if *c == b => {}
        other => {
          return Err(i.new_error(
            cssparser::BasicParseErrorKind::UnexpectedToken(other.clone()),
          ));
        }
      }
      Ok(())
    })
    .is_ok();

  if !ok {
    input.reset(&checkpoint);
  }
  ok
}
