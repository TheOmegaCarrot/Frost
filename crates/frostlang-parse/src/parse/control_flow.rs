use crate::ast::{Expr, Spanned, Statement};
use crate::lex::Token;
use crate::parse::ctx::ParseCtx;
use crate::parse::statements::StatementContext;
use crate::parse::{Diagnostic, ParseResult};

impl<'src> ParseCtx<'src> {
    pub(crate) fn parse_if(&mut self) -> ParseResult<Spanned<Expr>> {
        self.parse_if_or_elif(Token::KwIf)
    }

    fn parse_if_or_elif(&mut self, keyword: Token) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(keyword)?.span.start;

        let condition = self.parse_expression()?;
        self.expect(Token::Colon)?;
        // A branch may start on the line after its colon.
        self.skip_nl();
        let consequent = self.parse_expression()?;

        let alternate = self.parse_tail()?;

        let end = alternate.as_ref().unwrap_or(&consequent).span.end;

        Ok(Spanned::new(
            Expr::If {
                condition: Box::new(condition),
                consequent: Box::new(consequent),
                alternate: alternate.map(Box::new),
            },
            (start..end).into(),
        ))
    }

    fn parse_tail(&mut self) -> ParseResult<Option<Spanned<Expr>>> {
        let checkpoint = self.checkpoint();
        self.skip_nl();

        let Some(peek) = self.peek() else {
            self.restore(checkpoint);
            return Ok(None);
        };

        match peek.token {
            Token::KwElif => Ok(Some(self.parse_if_or_elif(Token::KwElif)?)),
            Token::KwElse => {
                self.expect(Token::KwElse)?;
                self.expect(Token::Colon)?;
                self.skip_nl();
                let alternate = self.parse_expression()?;
                Ok(Some(alternate))
            }
            _ => {
                self.restore(checkpoint);
                Ok(None)
            }
        }
    }

    pub(crate) fn parse_do(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(Token::KwDo)?.span.start;
        let (mut body, braces) = self.block(|ctx| ctx.parse_statements(StatementContext::Scope))?;
        let close_end = braces.end;

        let Some(last) = body.pop() else {
            return Err(Diagnostic::at(
                "`do` block must contain at least one expression",
                (start..close_end).into(),
                "empty block",
            ));
        };

        let value = match last.node {
            Statement::Expr(expr) => expr,
            Statement::Def { .. } => {
                return Err(Diagnostic::at(
                    "`do` block must end with an expression, not a definition",
                    last.span,
                    "definition here",
                ));
            }
        };

        Ok(Spanned::new(
            Expr::Do {
                body,
                value: Box::new(value),
            },
            (start..close_end).into(),
        ))
    }
}
