use crate::ast::{
    Binding, Expr, Literal, MapPatternEntry, MatchArm, MatchPattern, SourceSpan, Spanned,
    TypeConstraint,
};
use crate::lex::Token;
use crate::parse::strings::QuoteStyle;
use crate::parse::{ParseResult, ctx::ParseCtx};

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub fn parse_match(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(Token::KwMatch)?.span.start;

        let target = self.parse_expression()?;

        self.expect(Token::OpenBrace)?;
        self.enter_nl_context();

        let (arms, close) =
            self.parse_comma_separated(Token::CloseBrace, "match expression", Self::parse_arm)?;

        Ok(Spanned::new(
            Expr::Match {
                target: Box::new(target),
                arms,
            },
            (start..close.span.end).into(),
        ))
    }

    fn parse_arm(&mut self) -> ParseResult<Spanned<MatchArm>> {
        let pattern = self.parse_pattern_alternatives()?;

        let guard = if matches!(self.peek().map(|t| &t.token), Some(Token::KwIf)) {
            self.advance(1);
            self.expect(Token::Colon)?;
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.maybe_skip_nl();
        self.expect(Token::FatArrow)?;
        self.maybe_skip_nl();

        let result = self.parse_expression()?;

        let span = (pattern.span.start..result.span.end).into();
        Ok(Spanned::new(
            MatchArm {
                pattern,
                guard,
                result,
            },
            span,
        ))
    }

    fn parse_pattern_alternatives(&mut self) -> ParseResult<Spanned<MatchPattern>> {
        let first = self.parse_single_pattern()?;

        if !matches!(self.peek().map(|t| &t.token), Some(Token::Pipe)) {
            return Ok(first);
        }

        let start = first.span.start;
        let mut alternatives = vec![first];

        while matches!(self.peek().map(|t| &t.token), Some(Token::Pipe)) {
            self.advance(1);
            self.maybe_skip_nl();
            alternatives.push(self.parse_single_pattern()?);
        }

        let end = alternatives
            .last()
            .expect("alternatives has at least two elements")
            .span
            .end;

        Ok(Spanned::new(
            MatchPattern::Alternative(alternatives),
            (start..end).into(),
        ))
    }

    fn parse_single_pattern(&mut self) -> ParseResult<Spanned<MatchPattern>> {
        self.maybe_skip_nl();
        let peek = self.must_peek("match pattern")?;

        let (peek_start, peek_end) = (peek.span.start, peek.span.end);
        match peek.token {
            Token::OpenBracket => self.parse_array_pattern(),
            Token::OpenBrace => self.parse_map_pattern(),

            Token::OpenParen => {
                self.advance(1);
                self.enter_nl_context().maybe_skip_nl();
                let expr = self.parse_expression()?;
                self.maybe_skip_nl().exit_nl_context();
                let close = self.expect(Token::CloseParen)?;
                Ok(Spanned::new(
                    MatchPattern::Value(expr),
                    (peek_start..close.span.end).into(),
                ))
            }

            Token::IntLiteral(n) => {
                self.advance(1);
                Ok(literal_pattern(peek_start, peek_end, Literal::Int(n)))
            }

            Token::FloatLiteral(n) => {
                self.advance(1);
                Ok(literal_pattern(peek_start, peek_end, Literal::Float(n)))
            }

            Token::KwTrue => {
                self.advance(1);
                Ok(literal_pattern(peek_start, peek_end, Literal::Bool(true)))
            }

            Token::KwFalse => {
                self.advance(1);
                Ok(literal_pattern(peek_start, peek_end, Literal::Bool(false)))
            }

            Token::KwNull => {
                self.advance(1);
                Ok(literal_pattern(peek_start, peek_end, Literal::Null))
            }

            Token::SingleQuoteStringLiteral(_) => self
                .parse_simple_string(QuoteStyle::Single)
                .map(expr_match_pattern),

            Token::DoubleQuoteStringLiteral(_) => self
                .parse_simple_string(QuoteStyle::Double)
                .map(expr_match_pattern),

            Token::RawStringLiteral(_) => self.parse_raw_string().map(expr_match_pattern),

            Token::MultilineStringLiteral(_) => {
                self.parse_multiline_string().map(expr_match_pattern)
            }

            Token::BytesLiteral(_) => self.parse_bytes_literal().map(expr_match_pattern),

            Token::SingleQuoteFormatStringLiteral(_) | Token::DoubleQuoteFormatStringLiteral(_) => {
                self.parse_format_string(match peek.token {
                    Token::SingleQuoteFormatStringLiteral(_) => QuoteStyle::Single,
                    Token::DoubleQuoteFormatStringLiteral(_) => QuoteStyle::Double,
                    _ => unreachable!(),
                })
                .map(expr_match_pattern)
            }

            Token::OpMinus => {
                self.advance(1);
                let next = self.must_peek("negative literal in match pattern")?;
                match next.token {
                    Token::IntLiteral(n) => {
                        let end = next.span.end;
                        self.advance(1);
                        Ok(literal_pattern(peek_start, end, Literal::Int(-n)))
                    }
                    Token::FloatLiteral(n) => {
                        let end = next.span.end;
                        self.advance(1);
                        Ok(literal_pattern(peek_start, end, Literal::Float(-n)))
                    }
                    _ => Err(self.unexpected_token(next, "negative literal in match pattern")),
                }
            }

            Token::Identifier(name) => {
                let name = name.to_owned();
                self.advance(1);
                self.parse_binding_pattern(name, peek_start, peek_end)
            }

            _ => Err(self.unexpected_token(peek, "match pattern")),
        }
    }

    fn parse_binding_pattern(
        &mut self,
        name: String,
        start: usize,
        end: usize,
    ) -> ParseResult<Spanned<MatchPattern>> {
        let binding = match name.as_str() {
            "_" => Binding::Discarded,
            _ => Binding::Named(name),
        };
        let name = Spanned::new(binding, (start..end).into());

        let type_constraint = if matches!(self.peek().map(|t| &t.token), Some(Token::KwIs)) {
            self.advance(1);
            Some(self.parse_type_constraint()?)
        } else {
            None
        };

        let end = if type_constraint.is_some() {
            self.get(self.here() - 1).map(|t| t.span.end).unwrap_or(end)
        } else {
            end
        };

        Ok(Spanned::new(
            MatchPattern::Binding {
                name,
                type_constraint,
            },
            (start..end).into(),
        ))
    }

    fn parse_type_constraint(&mut self) -> ParseResult<Spanned<TypeConstraint>> {
        let peek = self.must_peek("type constraint after 'is'")?;
        let span: SourceSpan = peek.span.clone().into();

        let constraint = match peek.token {
            Token::Identifier("Null") => TypeConstraint::Null,
            Token::Identifier("Int") => TypeConstraint::Int,
            Token::Identifier("Float") => TypeConstraint::Float,
            Token::Identifier("Bool") => TypeConstraint::Bool,
            Token::Identifier("String") => TypeConstraint::String,
            Token::Identifier("Bytes") => TypeConstraint::Bytes,
            Token::Identifier("Array") => TypeConstraint::Array,
            Token::Identifier("Map") => TypeConstraint::Map,
            Token::Identifier("Function") => TypeConstraint::Function,
            Token::Identifier("Primitive") => TypeConstraint::Primitive,
            Token::Identifier("Numeric") => TypeConstraint::Numeric,
            Token::Identifier("Structured") => TypeConstraint::Structured,
            Token::Identifier("Flat") => TypeConstraint::Flat,
            Token::Identifier("Nonnull") => TypeConstraint::Nonnull,
            _ => return Err(self.unexpected_token(peek, "type constraint")),
        };

        self.advance(1);
        Ok(Spanned::new(constraint, span))
    }

    fn parse_array_pattern(&mut self) -> ParseResult<Spanned<MatchPattern>> {
        let start = self.expect(Token::OpenBracket)?.span.start;
        self.enter_nl_context().maybe_skip_nl();

        let mut elements = Vec::new();
        let mut rest = None;

        if !matches!(self.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
            loop {
                self.maybe_skip_nl();

                if matches!(self.peek().map(|t| &t.token), Some(Token::DotDotDot)) {
                    self.advance(1);
                    rest = Some(self.parse_binding("rest binding after '...'")?);

                    self.maybe_skip_nl();
                    break;
                }

                elements.push(self.parse_pattern_alternatives()?);
                self.maybe_skip_nl();

                let peek = self.must_peek("array pattern")?;
                match peek.token {
                    Token::Comma => {
                        self.advance(1);
                        self.maybe_skip_nl();
                        if matches!(self.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
                            break;
                        }
                    }
                    Token::CloseBracket => break,
                    _ => return Err(self.unexpected_token(peek, "array pattern")),
                }
            }
        }

        self.maybe_skip_nl().exit_nl_context();
        let close = self.expect(Token::CloseBracket)?;

        Ok(Spanned::new(
            MatchPattern::Array { elements, rest },
            (start..close.span.end).into(),
        ))
    }

    fn parse_map_pattern(&mut self) -> ParseResult<Spanned<MatchPattern>> {
        let start = self.expect(Token::OpenBrace)?.span.start;
        self.enter_nl_context().maybe_skip_nl();

        let mut entries = Vec::new();

        if !matches!(self.peek().map(|t| &t.token), Some(Token::CloseBrace)) {
            loop {
                self.maybe_skip_nl();
                entries.push(self.parse_map_pattern_entry()?);
                self.maybe_skip_nl();

                let peek = self.must_peek("map pattern")?;
                match peek.token {
                    Token::Comma => {
                        self.advance(1);
                        self.maybe_skip_nl();
                        if matches!(self.peek().map(|t| &t.token), Some(Token::CloseBrace)) {
                            break;
                        }
                    }
                    Token::CloseBrace => break,
                    _ => return Err(self.unexpected_token(peek, "map pattern")),
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
            MatchPattern::Map {
                entries,
                bind_whole,
            },
            (start..end).into(),
        ))
    }

    fn parse_map_pattern_entry(&mut self) -> ParseResult<Spanned<MapPatternEntry>> {
        let peek = self.must_peek("map pattern entry")?;
        let entry_start = peek.span.start;

        match peek.token {
            Token::OpenBracket => {
                self.advance(1);
                self.enter_nl_context().maybe_skip_nl();
                let key = self.parse_expression()?;
                self.maybe_skip_nl().exit_nl_context();
                self.expect(Token::CloseBracket)?;
                self.expect(Token::Colon)?;
                self.maybe_skip_nl();
                let pattern = self.parse_pattern_alternatives()?;
                let span = (entry_start..pattern.span.end).into();
                Ok(Spanned::new(MapPatternEntry { key, pattern }, span))
            }

            Token::Identifier(name) => {
                let name = name.to_owned();
                let start = peek.span.start;
                let end = peek.span.end;
                self.advance(1);

                let peek2 = self.must_peek("map pattern entry")?;

                let pattern = if peek2.token == Token::Colon {
                    self.advance(1);
                    self.maybe_skip_nl();
                    self.parse_pattern_alternatives()?
                } else {
                    self.parse_binding_pattern(name.clone(), start, end)?
                };

                let span = (entry_start..pattern.span.end).into();
                Ok(Spanned::new(
                    MapPatternEntry {
                        key: string_key_expr(name, start, end),
                        pattern,
                    },
                    span,
                ))
            }

            _ => Err(self.unexpected_token(peek, "map pattern entry")),
        }
    }
}

fn expr_match_pattern(expr: Spanned<Expr>) -> Spanned<MatchPattern> {
    let span = expr.span;
    Spanned::new(MatchPattern::Value(expr), span)
}

fn literal_pattern(start: usize, end: usize, literal: Literal) -> Spanned<MatchPattern> {
    Spanned::new(
        MatchPattern::Value(Spanned::new(Expr::Literal(literal), (start..end).into())),
        (start..end).into(),
    )
}

fn string_key_expr(name: String, start: usize, end: usize) -> Spanned<Expr> {
    Spanned::new(Expr::Literal(Literal::String(name)), (start..end).into())
}
