use crate::ast::{self, Binding, Spanned};
use crate::lex::Token;
use crate::parse::ctx::ParseCtx;
use crate::parse::statements::StatementContext;

mod control_flow;
mod destructure;
mod error;
mod expression;
mod format_string;
mod iterative;
mod lambda;
mod match_expr;
mod statements;
mod strings;
mod structures;

pub(crate) mod ctx;

pub(crate) use error::Diagnostic;
pub use error::{Label, ParseError};

pub fn parse_program(filename: &str, input: &str) -> Result<ast::Program, ParseError> {
    let mut ctx =
        ParseCtx::new(filename, input).map_err(|d| ParseError::from_diag(d, filename, input))?;

    ctx.parse_statements(StatementContext::TopLevel)
        .map(|statements| ast::Program { statements })
        .map_err(|d| ParseError::from_diag(d, filename, input))
}

/// The error carried while parsing is the lightweight [`Diagnostic`]; it is rendered
/// into a public [`ParseError`] only at the [`parse_program`] boundary.
type ParseResult<T> = Result<T, Diagnostic>;

impl<'src, 'f> ParseCtx<'src, 'f> {
    fn parse_binding(&mut self, context: &str) -> ParseResult<Spanned<Binding>> {
        let peek = self.must_peek(context)?;
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
            _ => Err(self.unexpected_token(peek, context)),
        }
    }
}
