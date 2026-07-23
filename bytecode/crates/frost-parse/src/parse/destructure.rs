use crate::ast::{Binding, Destructure, Expr, Literal, MapDestructureEntry, SourceSpan, Spanned};
use crate::lex::Token;
use crate::parse::expression::parse_expression;
use crate::parse::{ParseResult, ctx::ParseCtx, parse_binding};

pub fn parse_destructure(ctx: &mut ParseCtx) -> ParseResult<Spanned<Destructure>> {
    let peek = ctx.must_peek("destructuring")?;

    match peek.token {
        Token::Identifier(_) => {
            let span = peek.span.clone();
            let binding = parse_binding(ctx, "destructuring")?;
            Ok(Spanned::new(Destructure::Binding(binding), span.into()))
        }
        Token::OpenBracket => parse_destructure_array(ctx),
        Token::OpenBrace => parse_destructure_map(ctx),
        _ => Err(ctx.unexpected_token(peek, "destructuring")),
    }
}

fn parse_destructure_array(ctx: &mut ParseCtx) -> ParseResult<Spanned<Destructure>> {
    let start = ctx.expect(Token::OpenBracket)?.span.start;
    ctx.enter_nl_context().maybe_skip_nl();

    let mut elements = Vec::new();
    let mut rest = None;

    if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
        ctx.exit_nl_context();
        let close = ctx.expect(Token::CloseBracket)?;
        return Ok(Spanned::new(
            Destructure::Array { elements, rest },
            (start..close.span.end).into(),
        ));
    }

    loop {
        ctx.maybe_skip_nl();

        if matches!(ctx.peek().map(|t| &t.token), Some(Token::DotDotDot)) {
            ctx.advance(1);
            rest = Some(parse_binding(ctx, "rest binding")?);
            break;
        }

        elements.push(parse_destructure(ctx)?);
        ctx.maybe_skip_nl();

        let peek = ctx.must_peek("Array destructuring")?;

        match peek.token {
            Token::Comma => {
                ctx.advance(1);
                ctx.maybe_skip_nl();
            }
            Token::CloseBracket => break,
            _ => return Err(ctx.unexpected_token(peek, "Array destructuring")),
        }

        if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
            break;
        }
    }

    ctx.maybe_skip_nl().exit_nl_context();
    let close = ctx.expect(Token::CloseBracket)?;

    Ok(Spanned::new(
        Destructure::Array { elements, rest },
        (start..close.span.end).into(),
    ))
}

fn parse_destructure_map(ctx: &mut ParseCtx) -> ParseResult<Spanned<Destructure>> {
    let start = ctx.expect(Token::OpenBrace)?.span.start;
    ctx.enter_nl_context().maybe_skip_nl();

    let mut entries = Vec::new();

    if !matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBrace)) {
        loop {
            ctx.maybe_skip_nl();
            entries.push(parse_map_entry(ctx)?);
            ctx.maybe_skip_nl();

            let peek = ctx.must_peek("Map destructuring")?;
            match peek.token {
                Token::Comma => {
                    ctx.advance(1);
                    ctx.maybe_skip_nl();
                }
                Token::CloseBrace => break,
                _ => return Err(ctx.unexpected_token(peek, "Map destructuring")),
            }

            if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBrace)) {
                break;
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

    Ok(Spanned::new(
        Destructure::Map {
            entries,
            bind_whole,
        },
        (start..end).into(),
    ))
}

fn string_key_expr(name: String, span: SourceSpan) -> Spanned<Expr> {
    Spanned::new(Expr::Literal(Literal::String(name.into_bytes())), span)
}

fn parse_map_entry(ctx: &mut ParseCtx) -> ParseResult<Spanned<MapDestructureEntry>> {
    let peek = ctx.must_peek("Map destructuring entry")?;
    let start = peek.span.start;

    match peek.token {
        Token::OpenBracket => {
            ctx.advance(1);
            ctx.enter_nl_context().maybe_skip_nl();
            let key = parse_expression(ctx)?;
            ctx.maybe_skip_nl().exit_nl_context();
            ctx.expect(Token::CloseBracket)?;
            ctx.expect(Token::Colon)?;
            ctx.maybe_skip_nl();
            let destructure = parse_destructure(ctx)?;
            let span = (start..destructure.span.end).into();
            Ok(Spanned::new(MapDestructureEntry { key, destructure }, span))
        }
        Token::Identifier(name) => {
            let name = name.to_owned();
            let name_span: SourceSpan = peek.span.clone().into();
            ctx.advance(1);

            let peek = ctx.must_peek("Map destructuring entry")?;
            if peek.token == Token::Colon {
                ctx.advance(1);
                ctx.maybe_skip_nl();
                let destructure = parse_destructure(ctx)?;
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
        _ => Err(ctx.unexpected_token(peek, "Map destructuring entry")),
    }
}
