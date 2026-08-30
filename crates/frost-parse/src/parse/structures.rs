use crate::ast::{Expr, Literal, MapEntry, Spanned};
use crate::lex::Token;
use crate::parse::{ParseResult, ctx::ParseCtx};

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub(crate) fn parse_array_literal(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(Token::OpenBracket)?.span.start;
        self.enter_nl_context();

        let (elements, close) =
            self.parse_comma_separated(Token::CloseBracket, "Array literal", |ctx| {
                ctx.parse_expression()
            })?;

        Ok(Spanned::new(
            Expr::Array(elements),
            (start..close.span.end).into(),
        ))
    }

    pub(crate) fn parse_map_literal(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(Token::OpenBrace)?.span.start;
        self.enter_nl_context();

        let (entries, close) =
            self.parse_comma_separated(Token::CloseBrace, "Map literal", Self::parse_map_entry)?;

        Ok(Spanned::new(
            Expr::Map(entries),
            (start..close.span.end).into(),
        ))
    }

    fn parse_map_entry(&mut self) -> ParseResult<Spanned<MapEntry>> {
        let peek = self.must_peek("Map entry")?;
        let start = peek.span.start;

        let key = match peek.token {
            Token::OpenBracket => {
                self.expect(Token::OpenBracket)?;
                self.enter_nl_context().maybe_skip_nl();
                let key = self.parse_expression()?;
                self.maybe_skip_nl().exit_nl_context();
                self.expect(Token::CloseBracket)?;
                key
            }
            Token::Identifier(name) => {
                let name = name.to_owned();
                let span = peek.span.clone();
                self.advance(1);
                Spanned::new(Expr::Literal(Literal::String(name)), span.into())
            }
            _ => return Err(self.unexpected_token(peek, "Map entry key")),
        };

        self.expect(Token::Colon)?;
        self.maybe_skip_nl();
        let value = self.parse_expression()?;

        let span = (start..value.span.end).into();
        Ok(Spanned::new(MapEntry { key, value }, span))
    }
}
