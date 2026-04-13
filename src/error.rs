//! Parse error types for ECSS.

use std::fmt;

/// A parse error with source location.
#[derive(Debug, Clone)]
pub struct ParseError {
  pub message: String,
  pub line: u32,
  pub column: u32,
}

impl ParseError {
  pub fn new(message: impl Into<String>, line: u32, column: u32) -> Self {
    Self {
      message: message.into(),
      line,
      column,
    }
  }
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "[{}:{}] {}", self.line, self.column, self.message)
  }
}

impl std::error::Error for ParseError {}

/// Convert a `ParseError` into a `napi::Error` so it becomes a JS Error.
impl From<ParseError> for napi::Error {
  fn from(e: ParseError) -> Self {
    napi::Error::from_reason(e.to_string())
  }
}

/// Our custom error kind used with cssparser.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum EcssError {
  UnexpectedToken(String),
  ExpectedIdent,
  ExpectedBlock,
  ExpectedValues,
  InvalidArgument(String),
  InvalidCondition(String),
  UnexpectedElse,
  Custom(String),
}

impl fmt::Display for EcssError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::UnexpectedToken(t) => write!(f, "Unexpected token: {t}"),
      Self::ExpectedIdent => write!(f, "Expected identifier"),
      Self::ExpectedBlock => write!(f, "Expected '{{' block"),
      Self::ExpectedValues => {
        write!(f, "Expected 'values:' inside @state-variant")
      }
      Self::InvalidArgument(s) => write!(f, "Invalid argument: {s}"),
      Self::InvalidCondition(s) => write!(f, "Invalid condition: {s}"),
      Self::UnexpectedElse => write!(f, "@else without preceding @if"),
      Self::Custom(s) => write!(f, "{s}"),
    }
  }
}
