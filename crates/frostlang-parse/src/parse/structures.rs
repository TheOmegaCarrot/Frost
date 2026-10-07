use crate::ast::{Expr, Literal, MapEntry, SourceSpan, Spanned};
use crate::lex::Token;
use crate::parse::ParseResult;
use crate::parse::ctx::{Bracket, ParseCtx};

impl<'src> ParseCtx<'src> {
    pub(crate) fn parse_array_literal(&mut self) -> ParseResult<Spanned<Expr>> {
        let (elements, span) =
            self.parse_comma_separated(Bracket::ArrayLiteral, Self::parse_expression)?;
        Ok(Spanned::new(Expr::Array(elements), span))
    }

    pub(crate) fn parse_map_literal(&mut self) -> ParseResult<Spanned<Expr>> {
        let (entries, span) =
            self.parse_comma_separated(Bracket::MapLiteral, Self::parse_map_entry)?;
        Ok(Spanned::new(Expr::Map(entries), span))
    }

    fn parse_map_entry(&mut self) -> ParseResult<Spanned<MapEntry>> {
        const EXPECTED: &str = "a name or `[`";
        let peek = self.must_peek(EXPECTED)?;
        let start = peek.span.start;

        let key = match peek.token {
            Token::OpenBracket => {
                let (key, _) = self.delimited(Bracket::ComputedKey, Self::parse_expression)?;
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
            _ => return Err(self.expected_in_list(EXPECTED, peek)),
        };

        self.expect(Token::Colon)?;
        self.maybe_skip_nl();
        let value = self.parse_expression()?;

        let span = (start..value.span.end).into();
        Ok(Spanned::new(MapEntry { key, value }, span))
    }
}
