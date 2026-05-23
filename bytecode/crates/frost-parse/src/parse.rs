use std::error::Error;
use std::fmt::Display;

use crate::ast;
use crate::lex::Token;
use crate::parse::ctx::ParseCtx;

use logos::Logos;

mod program;

pub(crate) mod ctx;

/// A parser error.
#[derive(Clone, Debug)]
pub struct ParseError(String);

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for ParseError {}

impl From<&str> for ParseError {
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
    }
}

impl From<String> for ParseError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

pub fn parse_program(filename: &str, input: &str) -> Result<ast::Program, ParseError> {
    let ctx = ParseCtx::new(filename, input)?;

    // TODO: call into the "real" parser entry point (NYI)
    todo!()
}
