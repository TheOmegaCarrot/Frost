use crate::ast::{Binding, Destructure, Expr, Literal, MapDestructureEntry, SourceSpan, Spanned};
use crate::lex::Token;
use crate::parse::{ParseResult, ctx::ParseCtx};

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub(crate) fn parse_destructure(&mut self) -> ParseResult<Spanned<Destructure>> {
        let peek = self.must_peek("destructuring")?;

        match peek.token {
            Token::Identifier(_) => {
                let span = peek.span.clone();
                let binding = self.parse_binding("destructuring")?;
                Ok(Spanned::new(Destructure::Binding(binding), span.into()))
            }
            Token::OpenBracket => self.parse_destructure_array(),
            Token::OpenBrace => self.parse_destructure_map(),
            _ => Err(self.unexpected_token(peek, "destructuring")),
        }
    }

    fn parse_destructure_array(&mut self) -> ParseResult<Spanned<Destructure>> {
        let start = self.expect(Token::OpenBracket)?.span.start;
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
                rest = Some(self.parse_binding("rest binding")?);
                break;
            }

            elements.push(self.parse_destructure()?);
            self.maybe_skip_nl();

            let peek = self.must_peek("Array destructuring")?;

            match peek.token {
                Token::Comma => {
                    self.advance(1);
                    self.maybe_skip_nl();
                }
                Token::CloseBracket => break,
                _ => return Err(self.unexpected_token(peek, "Array destructuring")),
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
        let start = self.expect(Token::OpenBrace)?.span.start;
        self.enter_nl_context().maybe_skip_nl();

        let mut entries = Vec::new();

        if !matches!(self.peek().map(|t| &t.token), Some(Token::CloseBrace)) {
            loop {
                self.maybe_skip_nl();
                entries.push(self.parse_destructure_map_entry()?);
                self.maybe_skip_nl();

                let peek = self.must_peek("Map destructuring")?;
                match peek.token {
                    Token::Comma => {
                        self.advance(1);
                        self.maybe_skip_nl();
                    }
                    Token::CloseBrace => break,
                    _ => return Err(self.unexpected_token(peek, "Map destructuring")),
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
            let binding = self.parse_binding("as binding")?;
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
        let peek = self.must_peek("Map destructuring entry")?;
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

                let peek = self.must_peek("Map destructuring entry")?;
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
            _ => Err(self.unexpected_token(peek, "Map destructuring entry")),
        }
    }
}

fn string_key_expr(name: String, span: SourceSpan) -> Spanned<Expr> {
    Spanned::new(Expr::Literal(Literal::String(name)), span)
}
