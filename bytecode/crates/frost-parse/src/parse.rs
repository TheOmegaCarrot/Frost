use std::error::Error;
use std::fmt::Display;

use crate::ast;
use crate::lex::Token;
use crate::parse::ctx::ParseCtx;
use crate::parse::statements::{StatementContext, parse_statements};

use logos::Logos;

mod destructure;
mod expression;
mod statements;

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

impl From<miette::Report> for ParseError {
    fn from(value: miette::Report) -> Self {
        Self(format!("{:?}", value))
    }
}

pub fn parse_program(filename: &str, input: &str) -> ParseResult<ast::Program> {
    let mut ctx = ParseCtx::new(filename, input)?;

    parse_statements(&mut ctx, StatementContext::TopLevel)
        .map(|statements| ast::Program { statements })
}

type ParseResult<T> = Result<T, ParseError>;
