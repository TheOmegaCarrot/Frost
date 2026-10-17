use crate::ast::{Expr, Literal, MapEntry, SourceSpan, Spanned};
use crate::lex::Token;
use crate::parse::ParseResult;
use crate::parse::ctx::{Bracket, ParseCtx};

impl<'src> ParseCtx<'src> {
    #[inline(never)] // See `parse_atom`.
    pub(crate) fn parse_array_literal(&mut self) -> ParseResult<Spanned<Expr>> {
        let (elements, span) =
            self.parse_comma_separated(Bracket::ArrayLiteral, Self::parse_expression)?;
        Ok(Spanned::new(Expr::Array(elements), span))
    }

    #[inline(never)] // See `parse_atom`.
    pub(crate) fn parse_map_literal(&mut self) -> ParseResult<Spanned<Expr>> {
        let (entries, span) =
            self.parse_comma_separated(Bracket::MapLiteral, Self::parse_map_entry)?;
        Ok(Spanned::new(Expr::Map(entries), span))
    }

    fn parse_map_entry(&mut self) -> ParseResult<Spanned<MapEntry>> {
        let start = self.must_peek("a name or `[`")?.span.start;
        // The key is parsed apart, so that this frame, on the path through nested Map
        // literals, holds none of its locals.
        let key = match self.parse_map_key()? {
            MapKey::Key(key) => key,
            MapKey::Shorthand(entry) => return Ok(entry),
        };

        self.expect_colon_on_the_line()?;
        self.maybe_skip_nl();
        let value = self.parse_expression()?;

        let span = (start..value.span.end).into();
        Ok(Spanned::new(MapEntry { key, value }, span))
    }

    /// A Map literal entry's key, or for the shorthand `name`, the whole entry.
    fn parse_map_key(&mut self) -> ParseResult<MapKey> {
        const EXPECTED: &str = "a name or `[`";
        let peek = self.must_peek(EXPECTED)?;

        match peek.token {
            Token::OpenBracket => {
                let (key, _) = self.delimited(Bracket::ComputedKey, Self::parse_expression)?;
                Ok(MapKey::Key(key))
            }
            Token::Identifier(name) => {
                let name = name.to_owned();
                let span: SourceSpan = peek.span.clone().into();
                self.advance(1);

                // A `:` on a later line is still this key's, misplaced.
                if !matches!(self.peek_past_nl().map(|t| &t.token), Some(Token::Colon)) {
                    // The shorthand `name`, for `name: name`.
                    let key = Spanned::new(Expr::Literal(Literal::String(name.clone())), span);
                    let value = Spanned::new(Expr::NameLookup(name), span);
                    return Ok(MapKey::Shorthand(Spanned::new(
                        MapEntry { key, value },
                        span,
                    )));
                }

                Ok(MapKey::Key(Spanned::new(
                    Expr::Literal(Literal::String(name)),
                    span,
                )))
            }
            _ => Err(self.expected_in_list(EXPECTED, peek)),
        }
    }
}

/// A Map literal entry's key, as [`parse_map_key`](ParseCtx::parse_map_key) reads it.
enum MapKey {
    /// A key, which a `:` and the entry's value follow.
    Key(Spanned<Expr>),
    /// The shorthand `name`, a whole entry.
    Shorthand(Spanned<MapEntry>),
}
