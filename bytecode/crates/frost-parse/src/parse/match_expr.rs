use crate::ast::{
    Binding, Expr, ExprKind, Literal, MapPatternEntry, MatchArm, MatchPattern, MatchPatternKind,
    TypeConstraint,
};
use crate::lex::Token;
use crate::parse::expression::parse_expression;
use crate::parse::{ParseResult, ctx::ParseCtx, parse_binding};

pub fn parse_match(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let start = ctx.expect(Token::KwMatch)?.span.start;

    let target = parse_expression(ctx)?;

    ctx.expect(Token::OpenBrace)?;
    ctx.enter_nl_context();

    let (arms, close) =
        ctx.parse_comma_separated(Token::CloseBrace, "match expression", parse_arm)?;

    Ok(Expr {
        span: (start..close.span.end).into(),
        kind: ExprKind::Match {
            target: Box::new(target),
            arms,
        },
    })
}

fn parse_arm(ctx: &mut ParseCtx) -> ParseResult<MatchArm> {
    let pattern = parse_pattern_alternatives(ctx)?;

    let guard = if matches!(ctx.peek().map(|t| &t.token), Some(Token::KwIf)) {
        ctx.advance(1);
        ctx.expect(Token::Colon)?;
        Some(parse_expression(ctx)?)
    } else {
        None
    };

    ctx.maybe_skip_nl();
    ctx.expect(Token::FatArrow)?;
    ctx.maybe_skip_nl();

    let result = parse_expression(ctx)?;

    Ok(MatchArm {
        pattern,
        guard,
        result,
    })
}

fn parse_pattern_alternatives(ctx: &mut ParseCtx) -> ParseResult<MatchPattern> {
    let first = parse_single_pattern(ctx)?;

    if !matches!(ctx.peek().map(|t| &t.token), Some(Token::Pipe)) {
        return Ok(first);
    }

    let start = first.span.start;
    let mut alternatives = vec![first];

    while matches!(ctx.peek().map(|t| &t.token), Some(Token::Pipe)) {
        ctx.advance(1);
        ctx.maybe_skip_nl();
        alternatives.push(parse_single_pattern(ctx)?);
    }

    let end = alternatives
        .last()
        .expect("alternatives has at least two elements")
        .span
        .end;

    Ok(MatchPattern {
        span: (start..end).into(),
        kind: MatchPatternKind::Alternative(alternatives),
    })
}

fn parse_single_pattern(ctx: &mut ParseCtx) -> ParseResult<MatchPattern> {
    ctx.maybe_skip_nl();
    let peek = ctx.must_peek("match pattern")?;

    let (peek_start, peek_end) = (peek.span.start, peek.span.end);
    match peek.token {
        Token::OpenBracket => parse_array_pattern(ctx),
        Token::OpenBrace => parse_map_pattern(ctx),

        Token::OpenParen => {
            ctx.advance(1);
            ctx.enter_nl_context().maybe_skip_nl();
            let expr = parse_expression(ctx)?;
            ctx.maybe_skip_nl().exit_nl_context();
            let close = ctx.expect(Token::CloseParen)?;
            Ok(MatchPattern {
                span: (peek_start..close.span.end).into(),
                kind: MatchPatternKind::Value(expr),
            })
        }

        Token::IntLiteral(n) => {
            ctx.advance(1);
            Ok(literal_pattern(peek_start, peek_end, Literal::Int(n)))
        }

        Token::FloatLiteral(n) => {
            ctx.advance(1);
            Ok(literal_pattern(peek_start, peek_end, Literal::Float(n)))
        }

        Token::KwTrue => {
            ctx.advance(1);
            Ok(literal_pattern(peek_start, peek_end, Literal::Bool(true)))
        }

        Token::KwFalse => {
            ctx.advance(1);
            Ok(literal_pattern(peek_start, peek_end, Literal::Bool(false)))
        }

        Token::KwNull => {
            ctx.advance(1);
            Ok(literal_pattern(peek_start, peek_end, Literal::Null))
        }

        Token::SingleQuoteStringLiteral(_)
        | Token::DoubleQuoteStringLiteral(_)
        | Token::RawStringLiteral(_)
        | Token::MultilineStringLiteral(_)
        | Token::SingleQuoteFormatStringLiteral(_)
        | Token::DoubleQuoteFormatStringLiteral(_) => {
            let expr = parse_expression(ctx)?;
            Ok(MatchPattern {
                span: expr.span,
                kind: MatchPatternKind::Value(expr),
            })
        }

        Token::OpMinus => {
            ctx.advance(1);
            let next = ctx.must_peek("negative literal in match pattern")?;
            match next.token {
                Token::IntLiteral(n) => {
                    let end = next.span.end;
                    ctx.advance(1);
                    Ok(literal_pattern(peek_start, end, Literal::Int(-n)))
                }
                Token::FloatLiteral(n) => {
                    let end = next.span.end;
                    ctx.advance(1);
                    Ok(literal_pattern(peek_start, end, Literal::Float(-n)))
                }
                _ => Err(ctx.unexpected_token(next, "negative literal in match pattern")),
            }
        }

        Token::Identifier(name) => {
            let name = name.to_owned();
            ctx.advance(1);
            parse_binding_pattern(ctx, name, peek_start, peek_end)
        }

        _ => Err(ctx.unexpected_token(peek, "match pattern")),
    }
}

fn parse_binding_pattern(
    ctx: &mut ParseCtx,
    name: String,
    start: usize,
    end: usize,
) -> ParseResult<MatchPattern> {
    let binding = match name.as_str() {
        "_" => Binding::Discarded,
        _ => Binding::Named(name),
    };

    let type_constraint = if matches!(ctx.peek().map(|t| &t.token), Some(Token::KwIs)) {
        ctx.advance(1);
        Some(parse_type_constraint(ctx)?)
    } else {
        None
    };

    let end = if type_constraint.is_some() {
        ctx.get(ctx.here() - 1)
            .map(|t| t.span.end)
            .unwrap_or(end)
    } else {
        end
    };

    Ok(MatchPattern {
        span: (start..end).into(),
        kind: MatchPatternKind::Binding {
            name: binding,
            type_constraint,
        },
    })
}

fn parse_type_constraint(ctx: &mut ParseCtx) -> ParseResult<TypeConstraint> {
    let peek = ctx.must_peek("type constraint after 'is'")?;

    let constraint = match peek.token {
        Token::Identifier("Null") => TypeConstraint::Null,
        Token::Identifier("Int") => TypeConstraint::Int,
        Token::Identifier("Float") => TypeConstraint::Float,
        Token::Identifier("Bool") => TypeConstraint::Bool,
        Token::Identifier("String") => TypeConstraint::String,
        Token::Identifier("Array") => TypeConstraint::Array,
        Token::Identifier("Map") => TypeConstraint::Map,
        Token::Identifier("Function") => TypeConstraint::Function,
        Token::Identifier("Primitive") => TypeConstraint::Primitive,
        Token::Identifier("Numeric") => TypeConstraint::Numeric,
        Token::Identifier("Structured") => TypeConstraint::Structured,
        Token::Identifier("Nonnull") => TypeConstraint::Nonnull,
        _ => return Err(ctx.unexpected_token(peek, "type constraint")),
    };

    ctx.advance(1);
    Ok(constraint)
}

fn parse_array_pattern(ctx: &mut ParseCtx) -> ParseResult<MatchPattern> {
    let start = ctx.expect(Token::OpenBracket)?.span.start;
    ctx.enter_nl_context().maybe_skip_nl();

    let mut elements = Vec::new();
    let mut rest = None;

    if !matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
        loop {
            ctx.maybe_skip_nl();

            if matches!(ctx.peek().map(|t| &t.token), Some(Token::DotDotDot)) {
                ctx.advance(1);
                rest = Some(parse_binding(ctx, "rest binding after '...'")?);

                ctx.maybe_skip_nl();
                break;
            }

            elements.push(parse_pattern_alternatives(ctx)?);
            ctx.maybe_skip_nl();

            let peek = ctx.must_peek("array pattern")?;
            match peek.token {
                Token::Comma => {
                    ctx.advance(1);
                    ctx.maybe_skip_nl();
                    if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
                        break;
                    }
                }
                Token::CloseBracket => break,
                _ => return Err(ctx.unexpected_token(peek, "array pattern")),
            }
        }
    }

    ctx.maybe_skip_nl().exit_nl_context();
    let close = ctx.expect(Token::CloseBracket)?;

    Ok(MatchPattern {
        span: (start..close.span.end).into(),
        kind: MatchPatternKind::Array { elements, rest },
    })
}

fn parse_map_pattern(ctx: &mut ParseCtx) -> ParseResult<MatchPattern> {
    let start = ctx.expect(Token::OpenBrace)?.span.start;
    ctx.enter_nl_context().maybe_skip_nl();

    let mut entries = Vec::new();

    if !matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBrace)) {
        loop {
            ctx.maybe_skip_nl();
            entries.push(parse_map_pattern_entry(ctx)?);
            ctx.maybe_skip_nl();

            let peek = ctx.must_peek("map pattern")?;
            match peek.token {
                Token::Comma => {
                    ctx.advance(1);
                    ctx.maybe_skip_nl();
                    if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBrace)) {
                        break;
                    }
                }
                Token::CloseBrace => break,
                _ => return Err(ctx.unexpected_token(peek, "map pattern")),
            }
        }
    }

    ctx.maybe_skip_nl().exit_nl_context();
    let mut end = ctx.expect(Token::CloseBrace)?.span.end;

    let bind_whole = if matches!(ctx.peek().map(|t| &t.token), Some(Token::KwAs)) {
        ctx.advance(1);
        let binding = parse_binding(ctx, "as binding")?;
        if let Some(t) = ctx.get(ctx.here() - 1) {
            end = t.span.end;
        }
        Some(binding)
    } else {
        None
    };

    Ok(MatchPattern {
        span: (start..end).into(),
        kind: MatchPatternKind::Map {
            entries,
            bind_whole,
        },
    })
}

fn parse_map_pattern_entry(ctx: &mut ParseCtx) -> ParseResult<MapPatternEntry> {
    let peek = ctx.must_peek("map pattern entry")?;

    match peek.token {
        Token::OpenBracket => {
            ctx.advance(1);
            ctx.enter_nl_context().maybe_skip_nl();
            let key = parse_expression(ctx)?;
            ctx.maybe_skip_nl().exit_nl_context();
            ctx.expect(Token::CloseBracket)?;
            ctx.expect(Token::Colon)?;
            ctx.maybe_skip_nl();
            let pattern = parse_pattern_alternatives(ctx)?;
            Ok(MapPatternEntry { key, pattern })
        }

        Token::Identifier(name) => {
            let name = name.to_owned();
            let start = peek.span.start;
            let end = peek.span.end;
            ctx.advance(1);

            let peek2 = ctx.must_peek("map pattern entry")?;

            let pattern = if peek2.token == Token::Colon {
                ctx.advance(1);
                ctx.maybe_skip_nl();
                parse_pattern_alternatives(ctx)?
            } else {
                parse_binding_pattern(ctx, name.clone(), start, end)?
            };

            Ok(MapPatternEntry {
                key: string_key_expr(name, start, end),
                pattern,
            })
        }

        _ => Err(ctx.unexpected_token(peek, "map pattern entry")),
    }
}

fn literal_pattern(start: usize, end: usize, literal: Literal) -> MatchPattern {
    MatchPattern {
        span: (start..end).into(),
        kind: MatchPatternKind::Value(Expr {
            span: (start..end).into(),
            kind: ExprKind::Literal(literal),
        }),
    }
}

fn string_key_expr(name: String, start: usize, end: usize) -> Expr {
    Expr {
        span: (start..end).into(),
        kind: ExprKind::Literal(Literal::String(name.into_bytes())),
    }
}
