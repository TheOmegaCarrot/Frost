use crate::ast::{self, Binding, Spanned};
use crate::lex::Token;
use crate::parse::ctx::ParseCtx;
use crate::parse::statements::StatementContext;

mod control_flow;
mod destructure;
mod error;
mod expression;
mod format_string;
mod hints;
mod iterative;
mod lambda;
mod match_expr;
mod statements;
mod strings;
mod structures;

pub(crate) mod ctx;

pub(crate) use error::Diagnostic;
pub use error::{Label, ParseError};

/// Parses `input`, the source of a whole Frost program, into an [`ast::Program`].
///
/// `filename` names the source when a [`ParseError`] is rendered; it is used only for display.
pub fn parse_program(filename: &str, input: &str) -> Result<ast::Program, ParseError> {
    ParseCtx::new(input)
        .and_then(|mut ctx| ctx.parse_statements(StatementContext::TopLevel))
        .map(|statements| ast::Program { statements })
        .map_err(|diagnostic| ParseError::new(diagnostic, filename, input))
}

type ParseResult<T> = Result<T, Diagnostic>;

impl<'src> ParseCtx<'src> {
    /// Parse a name or `_`; `expected` describes it for an error, as in [`ParseCtx::expected`].
    fn parse_binding(&mut self, expected: &str) -> ParseResult<Spanned<Binding>> {
        let peek = self.must_peek(expected)?;
        let span = peek.span.clone().into();
        match peek.token {
            Token::Identifier("_") => {
                self.advance(1);
                Ok(Spanned::new(Binding::Discarded, span))
            }
            Token::Identifier(name) => {
                let name = name.to_owned();
                self.advance(1);
                Ok(Spanned::new(Binding::Named(name), span))
            }
            _ => Err(self.expected(expected, peek)),
        }
    }
}
