use crate::ast::{
    Binding, Expr, Literal, MapPatternEntry, Pattern, SourceSpan, Spanned, TypeConstraint,
};
use crate::lex::Token;
use crate::parse::ParseResult;
use crate::parse::ctx::{Bracket, ParseCtx, int_literal};
use crate::parse::strings::QuoteStyle;

impl<'src> ParseCtx<'src> {
    /// A pattern, alternatives included: `p1 | p2 | ...`, as a child of the node being built.
    pub(crate) fn parse_pattern(&mut self) -> ParseResult<Spanned<Pattern>> {
        self.nested(Self::parse_alternatives)
    }

    fn parse_alternatives(&mut self) -> ParseResult<Spanned<Pattern>> {
        let first = self.parse_single_pattern()?;
        self.maybe_skip_nl();

        match self.peek() {
            Some(pipe) if pipe.token == Token::Pipe => self.parse_more_alternatives(first),
            _ => Ok(first),
        }
    }

    /// The alternatives after `first`, from the `|` the parse is at.
    // Apart from `parse_alternatives`, so that a single pattern, the usual case and
    // the path through nested patterns, has none of these locals in its frame.
    fn parse_more_alternatives(
        &mut self,
        first: Spanned<Pattern>,
    ) -> ParseResult<Spanned<Pattern>> {
        let pipe = self.peek().expect("a `|` is next").span.clone().into();
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

        // The alternatives were parsed in the place of the node that holds them.
        self.push_down(self.deepest(), pipe)?;

        Ok(Spanned::new(
            Pattern::Alternative(alternatives),
            (start..end).into(),
        ))
    }

    fn parse_single_pattern(&mut self) -> ParseResult<Spanned<Pattern>> {
        self.maybe_skip_nl();
        // Only dispatch is done here, so that this frame, on the path through nested
        // patterns, holds no pattern's locals. Each pattern holding others is parsed out of
        // line, as an optimized build would otherwise inline it here.
        match self.must_peek("a pattern")?.token {
            Token::OpenBracket => self.parse_array_pattern(),
            Token::OpenBrace => self.parse_map_pattern(),
            Token::OpenParen => self.parse_value_pattern(),
            _ => self.parse_leaf_pattern(),
        }
    }

    /// A parenthesized value pattern: `(expr)`.
    #[inline(never)] // See `parse_single_pattern`.
    fn parse_value_pattern(&mut self) -> ParseResult<Spanned<Pattern>> {
        let (expr, span) = self.delimited(Bracket::Group, Self::parse_expression)?;
        Ok(Spanned::new(Pattern::Value(expr), span))
    }

    /// A pattern holding no pattern or expression: a literal, or a binding.
    fn parse_leaf_pattern(&mut self) -> ParseResult<Spanned<Pattern>> {
        let peek = self.must_peek("a pattern")?;
        let (peek_start, peek_end) = (peek.span.start, peek.span.end);
        match peek.token {
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
                .map(value_pattern),

            Token::DoubleQuoteStringLiteral(_) => self
                .parse_simple_string(QuoteStyle::Double)
                .map(value_pattern),

            Token::RawStringLiteral(_) => self.parse_raw_string().map(value_pattern),

            Token::MultilineStringLiteral(_) => self.parse_multiline_string().map(value_pattern),

            Token::BytesLiteral(_) => self.parse_bytes_literal().map(value_pattern),

            Token::SingleQuoteFormatStringLiteral(_) | Token::DoubleQuoteFormatStringLiteral(_) => {
                self.parse_format_string(match peek.token {
                    Token::SingleQuoteFormatStringLiteral(_) => QuoteStyle::Single,
                    Token::DoubleQuoteFormatStringLiteral(_) => QuoteStyle::Double,
                    _ => unreachable!(),
                })
                .map(value_pattern)
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
    ) -> ParseResult<Spanned<Pattern>> {
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
            Pattern::Binding {
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
            Token::Identifier(name) => TypeConstraint::from_name(name),
            _ => None,
        };
        let Some(constraint) = named else {
            return Err(self.expected("a type name", peek));
        };

        self.advance(1);
        Ok(Spanned::new(constraint, span))
    }

    #[inline(never)] // See `parse_single_pattern`.
    fn parse_array_pattern(&mut self) -> ParseResult<Spanned<Pattern>> {
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

                elements.push(ctx.parse_pattern()?);
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

        Ok(Spanned::new(Pattern::Array { elements, rest }, span))
    }

    #[inline(never)] // See `parse_single_pattern`.
    fn parse_map_pattern(&mut self) -> ParseResult<Spanned<Pattern>> {
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
            Pattern::Map {
                entries,
                bind_whole,
            },
            (start..end).into(),
        ))
    }

    fn parse_map_pattern_entry(&mut self) -> ParseResult<Spanned<MapPatternEntry>> {
        let entry_start = self.must_peek("a name or `[`")?.span.start;
        // The key is parsed apart, so that this frame, on the path through nested Map
        // patterns, holds none of its locals.
        let (key, shorthand) = self.parse_map_pattern_key()?;
        let pattern = match shorthand {
            Some(binding) => binding,
            None => {
                self.expect_colon_on_the_line()?;
                self.maybe_skip_nl();
                self.parse_pattern()?
            }
        };
        let span = (entry_start..pattern.span.end).into();
        Ok(Spanned::new(MapPatternEntry { key, pattern }, span))
    }

    /// A Map pattern entry's key, and for the shorthand `name`, which binds the value at
    /// the key `name`, that binding. Otherwise a `:` and the entry's pattern follow.
    fn parse_map_pattern_key(&mut self) -> ParseResult<(Spanned<Expr>, Option<Spanned<Pattern>>)> {
        const EXPECTED: &str = "a name or `[`";
        let peek = self.must_peek(EXPECTED)?;

        match peek.token {
            Token::OpenBracket => {
                let (key, _) = self.delimited(Bracket::ComputedKey, Self::parse_expression)?;
                Ok((key, None))
            }

            Token::Identifier(name) => {
                let name = name.to_owned();
                let start = peek.span.start;
                let end = peek.span.end;
                self.advance(1);

                self.must_peek("`:`, `,`, or `}`")?;
                // A `:` on a later line is still this key's, misplaced.
                let colon_follows =
                    matches!(self.peek_past_nl().map(|t| &t.token), Some(Token::Colon));

                let shorthand = if colon_follows {
                    None
                } else {
                    Some(self.parse_binding_pattern(name.clone(), start, end)?)
                };
                Ok((string_key_expr(name, start, end), shorthand))
            }

            _ => Err(self.expected_in_list(EXPECTED, peek)),
        }
    }
}

fn value_pattern(expr: Spanned<Expr>) -> Spanned<Pattern> {
    let span = expr.span;
    Spanned::new(Pattern::Value(expr), span)
}

fn literal_pattern(start: usize, end: usize, literal: Literal) -> Spanned<Pattern> {
    Spanned::new(
        Pattern::Value(Spanned::new(Expr::Literal(literal), (start..end).into())),
        (start..end).into(),
    )
}

fn string_key_expr(name: String, start: usize, end: usize) -> Spanned<Expr> {
    Spanned::new(Expr::Literal(Literal::String(name)), (start..end).into())
}
