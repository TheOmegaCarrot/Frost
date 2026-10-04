use crate::ast::{Expr, Literal, MapEntry, SourceSpan, Spanned};
use crate::lex::Token;
use crate::parse::{ParseResult, ctx::ParseCtx};

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub(crate) fn parse_array_literal(&mut self) -> ParseResult<Spanned<Expr>> {
        let open: SourceSpan = self.expect(Token::OpenBracket)?.span.clone().into();
        let start = open.start;
        self.enter_nl_context();

        let (elements, close) = self.parse_comma_separated(
            open,
            "Array literal",
            Token::CloseBracket,
            Self::parse_expression,
        )?;

        Ok(Spanned::new(
            Expr::Array(elements),
            (start..close.span.end).into(),
        ))
    }

    pub(crate) fn parse_map_literal(&mut self) -> ParseResult<Spanned<Expr>> {
        let open: SourceSpan = self.expect(Token::OpenBrace)?.span.clone().into();
        let start = open.start;
        self.enter_nl_context();

        let (entries, close) = self.parse_comma_separated(
            open,
            "Map literal",
            Token::CloseBrace,
            Self::parse_map_entry,
        )?;

        Ok(Spanned::new(
            Expr::Map(entries),
            (start..close.span.end).into(),
        ))
    }

    fn parse_map_entry(&mut self) -> ParseResult<Spanned<MapEntry>> {
        const EXPECTED: &str = "a name or `[`";
        let peek = self.must_peek(EXPECTED)?;
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
                let span: SourceSpan = peek.span.clone().into();
                self.advance(1);

                if !matches!(self.peek().map(|t| &t.token), Some(Token::Colon)) {
                    // The shorthand `name`, for `name: name`.
                    let key = Spanned::new(Expr::Literal(Literal::String(name.clone())), span);
                    let value = Spanned::new(Expr::NameLookup(name), span);
                    return Ok(Spanned::new(MapEntry { key, value }, span));
                }

                Spanned::new(Expr::Literal(Literal::String(name)), span)
            }
            _ => return Err(self.expected(EXPECTED, peek)),
        };

        self.expect(Token::Colon)?;
        self.maybe_skip_nl();
        let value = self.parse_expression()?;

        let span = (start..value.span.end).into();
        Ok(Spanned::new(MapEntry { key, value }, span))
    }
}
