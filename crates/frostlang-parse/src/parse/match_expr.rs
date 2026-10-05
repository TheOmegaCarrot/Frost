use crate::ast::{
    Binding, Expr, Literal, MapPatternEntry, MatchArm, MatchPattern, SourceSpan, Spanned,
    TypeConstraint,
};
use crate::lex::Token;
use crate::parse::ParseResult;
use crate::parse::ctx::{Bracket, ParseCtx, int_literal};
use crate::parse::strings::QuoteStyle;

impl<'src, 'f> ParseCtx<'src, 'f> {
    pub(crate) fn parse_match(&mut self) -> ParseResult<Spanned<Expr>> {
        let start = self.expect(Token::KwMatch)?.span.start;

        let target = self.parse_expression()?;

        let (arms, span) = self.parse_comma_separated(Bracket::MatchArms, Self::parse_arm)?;

        Ok(Spanned::new(
            Expr::Match {
                target: Box::new(target),
                arms,
            },
            (start..span.end).into(),
        ))
    }

    fn parse_arm(&mut self) -> ParseResult<Spanned<MatchArm>> {
        let pattern = self.parse_pattern_alternatives()?;

        self.maybe_skip_nl();
        let guard = if matches!(self.peek().map(|t| &t.token), Some(Token::KwIf)) {
            self.advance(1);
            self.maybe_skip_nl();
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
        self.maybe_skip_nl();

        if !matches!(self.peek().map(|t| &t.token), Some(Token::Pipe)) {
            return Ok(first);
        }

        let start = first.span.start;
        let mut alternatives = vec![first];

        while matches!(self.peek().map(|t| &t.token), Some(Token::Pipe)) {
            self.advance(1);
            alternatives.push(self.parse_single_pattern()?);
            self.maybe_skip_nl();
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
        let peek = self.must_peek("a pattern")?;

        let (peek_start, peek_end) = (peek.span.start, peek.span.end);
        match peek.token {
            Token::OpenBracket => self.parse_array_pattern(),
            Token::OpenBrace => self.parse_map_pattern(),

            Token::OpenParen => {
                let (expr, span) = self.delimited(Bracket::Group, Self::parse_expression)?;
                Ok(Spanned::new(MatchPattern::Value(expr), span))
            }

            Token::IntLiteral(magnitude) => {
                let n = int_literal(magnitude, false, peek_start..peek_end)?;
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
                self.advance(1).maybe_skip_nl();
                let next = self.must_peek("a number after `-`")?;
                match next.token {
                    Token::IntLiteral(magnitude) => {
                        let end = next.span.end;
                        let n = int_literal(magnitude, true, peek_start..end)?;
                        self.advance(1);
                        Ok(literal_pattern(peek_start, end, Literal::Int(n)))
                    }
                    Token::FloatLiteral(n) => {
                        let end = next.span.end;
                        self.advance(1);
                        Ok(literal_pattern(peek_start, end, Literal::Float(-n)))
                    }
                    _ => Err(self.expected("a number after `-`", next)),
                }
            }

            Token::Identifier(name) => {
                let name = name.to_owned();
                self.advance(1);
                self.parse_binding_pattern(name, peek_start, peek_end)
            }

            _ => Err(self.expected("a pattern", peek)),
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

        self.maybe_skip_nl();
        let type_constraint = if matches!(self.peek().map(|t| &t.token), Some(Token::KwIs)) {
            self.advance(1).maybe_skip_nl();
            Some(self.parse_type_constraint()?)
        } else {
            None
        };

        let end = if type_constraint.is_some() {
            self.get(self.here() - 1).map_or(end, |t| t.span.end)
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
        let peek = self.must_peek("a type name")?;
        let span: SourceSpan = peek.span.clone().into();

        let named = match peek.token {
            Token::Identifier(name) => TYPE_CONSTRAINTS.iter().find(|(n, _)| *n == name),
            _ => None,
        };
        let Some(&(_, constraint)) = named else {
            return Err(self.expected("a type name", peek));
        };

        self.advance(1);
        Ok(Spanned::new(constraint, span))
    }

    fn parse_array_pattern(&mut self) -> ParseResult<Spanned<MatchPattern>> {
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

                elements.push(ctx.parse_pattern_alternatives()?);
                ctx.maybe_skip_nl();

                let peek = ctx.must_peek("`,` or `]`")?;
                match peek.token {
                    Token::Comma => {
                        ctx.advance(1);
                        ctx.maybe_skip_nl();
                        if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
                            return Ok((elements, None));
                        }
                    }
                    Token::CloseBracket => return Ok((elements, None)),
                    _ => return Err(ctx.expected_in_list("`,` or `]`", peek)),
                }
            }
        })?;

        Ok(Spanned::new(MatchPattern::Array { elements, rest }, span))
    }

    fn parse_map_pattern(&mut self) -> ParseResult<Spanned<MatchPattern>> {
        let (entries, span) =
            self.parse_comma_separated(Bracket::MapPattern, Self::parse_map_pattern_entry)?;
        let start = span.start;
        let mut end = span.end;

        self.maybe_skip_nl();
        let bind_whole = if matches!(self.peek().map(|t| &t.token), Some(Token::KwAs)) {
            self.advance(1).maybe_skip_nl();
            let binding = self.parse_binding("a name after `as`")?;
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
        const EXPECTED: &str = "a name or `[`";
        let peek = self.must_peek(EXPECTED)?;
        let entry_start = peek.span.start;

        match peek.token {
            Token::OpenBracket => {
                let (key, _) = self.delimited(Bracket::ComputedKey, Self::parse_expression)?;
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

                let peek2 = self.must_peek("`:`, `,`, or `}`")?;

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

            _ => Err(self.expected_in_list(EXPECTED, peek)),
        }
    }
}

/// Each type a constraint can name after `is`, by its name.
pub(crate) const TYPE_CONSTRAINTS: &[(&str, TypeConstraint)] = &[
    ("Null", TypeConstraint::Null),
    ("Int", TypeConstraint::Int),
    ("Float", TypeConstraint::Float),
    ("Bool", TypeConstraint::Bool),
    ("String", TypeConstraint::String),
    ("Bytes", TypeConstraint::Bytes),
    ("Array", TypeConstraint::Array),
    ("Map", TypeConstraint::Map),
    ("Function", TypeConstraint::Function),
    ("Opaque", TypeConstraint::Opaque),
    ("Primitive", TypeConstraint::Primitive),
    ("Numeric", TypeConstraint::Numeric),
    ("Structured", TypeConstraint::Structured),
    ("Flat", TypeConstraint::Flat),
    ("Nonnull", TypeConstraint::Nonnull),
];

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
