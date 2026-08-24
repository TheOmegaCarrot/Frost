use crate::ast::{Expr, Spanned, Statement};
use crate::lex::Token;
use crate::parse::statements::StatementContext;
use crate::parse::{Diagnostic, ParseResult, ctx::ParseCtx};

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub fn parse_if(&mut self) -> ParseResult<Spanned<Expr>> {
        self.parse_if_or_elif(Token::KwIf)
    }

    fn parse_if_or_elif(&mut self, keyword: Token) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(keyword)?.span.start;

        let condition = self.parse_expression()?;
        self.expect(Token::Colon)?;
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
                let alternate = self.parse_expression()?;
                Ok(Some(alternate))
            }
            _ => {
                self.restore(checkpoint);
                Ok(None)
            }
        }
    }

    pub fn parse_do(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(Token::KwDo)?.span.start;
        self.expect(Token::OpenBrace)?;

        let mut body = self.parse_statements(StatementContext::Scope)?;

        let close_end = self.expect(Token::CloseBrace)?.span.end;

        let Some(last) = body.pop() else {
            return Err(Diagnostic::at(
                "do block must contain at least one expression",
                (start..close_end).into(),
                "empty block",
            ));
        };

        let value = match last.node {
            Statement::Expr(expr) => expr,
            Statement::Def { .. } => {
                return Err(Diagnostic::at(
                    "do block must end with an expression, not a definition",
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
