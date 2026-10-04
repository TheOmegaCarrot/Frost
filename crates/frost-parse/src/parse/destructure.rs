use crate::ast::{Binding, Destructure, Expr, Literal, MapDestructureEntry, SourceSpan, Spanned};
use crate::lex::Token;
use crate::parse::{ParseResult, ctx::ParseCtx};

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
        let open: SourceSpan = self.expect(Token::OpenBracket)?.span.clone().into();
        let start = open.start;
        self.enter_nl_context().maybe_skip_nl();

        let mut elements = Vec::new();
        let mut rest = None;

        if matches!(self.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
            self.exit_nl_context();
            let close = self.expect(Token::CloseBracket)?;
            return Ok(Spanned::new(
                Destructure::Array { elements, rest },
                (start..close.span.end).into(),
            ));
        }

        loop {
            self.maybe_skip_nl();

            if matches!(self.peek().map(|t| &t.token), Some(Token::DotDotDot)) {
                self.advance(1);
                rest = Some(self.parse_binding("a name after `...`")?);
                break;
            }

            elements.push(self.parse_destructure()?);
            self.maybe_skip_nl();

            let peek = self.must_peek("`,` or `]`")?;

            match peek.token {
                Token::Comma => {
                    self.advance(1);
                    self.maybe_skip_nl();
                }
                Token::CloseBracket => break,
                _ => {
                    return Err(self.expected_in_list("`,` or `]`", peek, open, "Array pattern"));
                }
            }

            if matches!(self.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
                break;
            }
        }

        self.maybe_skip_nl().exit_nl_context();
        let close = self.expect(Token::CloseBracket)?;

        Ok(Spanned::new(
            Destructure::Array { elements, rest },
            (start..close.span.end).into(),
        ))
    }

    fn parse_destructure_map(&mut self) -> ParseResult<Spanned<Destructure>> {
        let open: SourceSpan = self.expect(Token::OpenBrace)?.span.clone().into();
        let start = open.start;
        self.enter_nl_context().maybe_skip_nl();

        let mut entries = Vec::new();

        if !matches!(self.peek().map(|t| &t.token), Some(Token::CloseBrace)) {
            loop {
                self.maybe_skip_nl();
                entries.push(self.parse_destructure_map_entry()?);
                self.maybe_skip_nl();

                let peek = self.must_peek("`,` or `}`")?;
                match peek.token {
                    Token::Comma => {
                        self.advance(1);
                        self.maybe_skip_nl();
                    }
                    Token::CloseBrace => break,
                    _ => {
                        return Err(self.expected_in_list("`,` or `}`", peek, open, "Map pattern"));
                    }
                }

                if matches!(self.peek().map(|t| &t.token), Some(Token::CloseBrace)) {
                    break;
                }
            }
        }

        self.maybe_skip_nl().exit_nl_context();
        let mut end = self.expect(Token::CloseBrace)?.span.end;

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
                self.advance(1);
                self.enter_nl_context().maybe_skip_nl();
                let key = self.parse_expression()?;
                self.maybe_skip_nl().exit_nl_context();
                self.expect(Token::CloseBracket)?;
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
            _ => Err(self.expected(EXPECTED, peek)),
        }
    }
}

fn string_key_expr(name: String, span: SourceSpan) -> Spanned<Expr> {
    Spanned::new(Expr::Literal(Literal::String(name)), span)
}
