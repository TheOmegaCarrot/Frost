use crate::ast::{Binding, Destructure, Expr, Literal, MapDestructureEntry, SourceSpan, Spanned};
use crate::lex::Token;
use crate::parse::ParseResult;
use crate::parse::ctx::{Bracket, ParseCtx};

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub(crate) fn parse_destructure(&mut self) -> ParseResult<Spanned<Destructure>> {
        const EXPECTED: &str = "a name, `[`, or `{`";
        let peek = self.must_peek(EXPECTED)?;

        match peek.token {
            Token::Identifier(_) => {
                let span = peek.span.clone();
                let binding = self.parse_binding(EXPECTED)?;
                Ok(Spanned::new(Destructure::Binding(binding), span.into()))
            }
            Token::OpenBracket => self.parse_destructure_array(),
            Token::OpenBrace => self.parse_destructure_map(),
            _ => Err(self.expected(EXPECTED, peek)),
        }
    }

    fn parse_destructure_array(&mut self) -> ParseResult<Spanned<Destructure>> {
        let ((elements, rest), span) = self.delimited(Bracket::ArrayPattern, |ctx| {
            let mut elements = Vec::new();

            if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
                return Ok((elements, None));
            }
            loop {
                ctx.maybe_skip_nl();

                if matches!(ctx.peek().map(|t| &t.token), Some(Token::DotDotDot)) {
                    ctx.advance(1);
                    let rest = ctx.parse_binding("a name after `...`")?;
                    return Ok((elements, Some(rest)));
                }

                elements.push(ctx.parse_destructure()?);
                ctx.maybe_skip_nl();

                let peek = ctx.must_peek("`,` or `]`")?;
                match peek.token {
                    Token::Comma => {
                        ctx.advance(1);
                        ctx.maybe_skip_nl();
                    }
                    Token::CloseBracket => return Ok((elements, None)),
                    _ => return Err(ctx.expected_in_list("`,` or `]`", peek)),
                }

                if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
                    return Ok((elements, None));
                }
            }
        })?;

        Ok(Spanned::new(Destructure::Array { elements, rest }, span))
    }

    fn parse_destructure_map(&mut self) -> ParseResult<Spanned<Destructure>> {
        let (entries, span) =
            self.parse_comma_separated(Bracket::MapPattern, Self::parse_destructure_map_entry)?;
        let start = span.start;
        let mut end = span.end;

        let bind_whole = if matches!(self.peek().map(|t| &t.token), Some(Token::KwAs)) {
            self.advance(1);
            let binding = self.parse_binding("a name after `as`")?;
            if let Some(t) = self.get(self.here() - 1) {
                end = t.span.end;
            }
            Some(binding)
        } else {
            None
        };

        Ok(Spanned::new(
            Destructure::Map {
                entries,
                bind_whole,
            },
            (start..end).into(),
        ))
    }

    fn parse_destructure_map_entry(&mut self) -> ParseResult<Spanned<MapDestructureEntry>> {
        const EXPECTED: &str = "a name or `[`";
        let peek = self.must_peek(EXPECTED)?;
        let start = peek.span.start;

        match peek.token {
            Token::OpenBracket => {
                let (key, _) = self.delimited(Bracket::ComputedKey, Self::parse_expression)?;
                self.expect(Token::Colon)?;
                self.maybe_skip_nl();
                let destructure = self.parse_destructure()?;
                let span = (start..destructure.span.end).into();
                Ok(Spanned::new(MapDestructureEntry { key, destructure }, span))
            }
            Token::Identifier(name) => {
                let name = name.to_owned();
                let name_span: SourceSpan = peek.span.clone().into();
                self.advance(1);

                let peek = self.must_peek("`:`, `,`, or `}`")?;
                if peek.token == Token::Colon {
                    self.advance(1);
                    self.maybe_skip_nl();
                    let destructure = self.parse_destructure()?;
                    let span = (start..destructure.span.end).into();
                    Ok(Spanned::new(
                        MapDestructureEntry {
                            key: string_key_expr(name, name_span),
                            destructure,
                        },
                        span,
                    ))
                } else {
                    let binding = match name.as_str() {
                        "_" => Binding::Discarded,
                        _ => Binding::Named(name.clone()),
                    };
                    Ok(Spanned::new(
                        MapDestructureEntry {
                            key: string_key_expr(name, name_span),
                            destructure: Spanned::new(
                                Destructure::Binding(Spanned::new(binding, name_span)),
                                name_span,
                            ),
                        },
                        name_span,
                    ))
                }
            }
            _ => Err(self.expected_in_list(EXPECTED, peek)),
        }
    }
}

fn string_key_expr(name: String, span: SourceSpan) -> Spanned<Expr> {
    Spanned::new(Expr::Literal(Literal::String(name)), span)
}
