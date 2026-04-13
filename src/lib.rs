#![deny(clippy::all)]

mod ast;
mod condition;
mod error;
mod parser;

#[cfg(test)]
mod tests;

pub use ast::*;

use napi_derive::napi;

/// Parse an ECSS source string and return the AST.
///
/// # Errors
///
/// Returns an error if the source is not valid ECSS. The error message includes
/// the source location: `[line:column] description`.
#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn parse_ecss(source: String) -> napi::Result<EcssStylesheet> {
  parser::parse(&source).map_err(Into::into)
}
