use crate::ast::{
    Binding, Destructure, DestructureKind, Expr, ExprKind, Literal, MapDestructureEntry,
};
use crate::lex::Token;
use crate::parse::expression::parse_expression;
use crate::parse::{ParseResult, ctx::ParseCtx};

pub fn parse_destructure(ctx: &mut ParseCtx) -> ParseResult<Destructure> {
    let peek = ctx.must_peek("destructuring")?;

    match peek.token {
        Token::Identifier(_) => {
            let span = peek.span.clone();
            let binding = parse_binding(ctx, "destructuring")?;
            Ok(Destructure {
                span: span.into(),
                kind: DestructureKind::Binding(binding),
            })
        }
        Token::OpenBracket => parse_destructure_array(ctx),
        Token::OpenBrace => parse_destructure_map(ctx),
        _ => Err(ctx.unexpected_token(peek, "destructuring")),
    }
}

fn parse_destructure_array(ctx: &mut ParseCtx) -> ParseResult<Destructure> {
    let start = ctx.expect(Token::OpenBracket)?.span.start;

    let mut elements = Vec::new();
    let mut rest = None;

    if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
        let close = ctx.expect(Token::CloseBracket)?;
        return Ok(Destructure {
            span: (start..close.span.end).into(),
            kind: DestructureKind::Array { elements, rest },
        });
    }

    loop {
        if matches!(ctx.peek().map(|t| &t.token), Some(Token::DotDotDot)) {
            ctx.advance(1);
            rest = Some(parse_binding(ctx, "rest binding")?);
            break;
        }

        elements.push(parse_destructure(ctx)?);

        let peek = ctx.must_peek("Array destructuring")?;

        match peek.token {
            Token::Comma => {
                ctx.advance(1);
            }
            Token::CloseBracket => break,
            _ => return Err(ctx.unexpected_token(peek, "Array destructuring")),
        }

        if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
            // unexpected eof handled by next loop
            break;
        }
    }

    let close = ctx.expect(Token::CloseBracket)?;

    Ok(Destructure {
        span: (start..close.span.end).into(),
        kind: DestructureKind::Array { elements, rest },
    })
}

fn parse_binding(ctx: &mut ParseCtx, context: &str) -> ParseResult<Binding> {
    let peek = ctx.must_peek(context)?;

    match peek.token {
        Token::Identifier("_") => { ctx.next(); Ok(Binding::Discarded) }
        Token::Identifier(name) => { let name = name.to_owned(); ctx.next(); Ok(Binding::Named(name)) }
        _ => Err(ctx.unexpected_token(peek, context)),
    }
}

fn parse_destructure_map(ctx: &mut ParseCtx) -> ParseResult<Destructure> {
    let start = ctx.expect(Token::OpenBrace)?.span.start;

    let mut entries = Vec::new();

    if !matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBrace)) {
        loop {
            entries.push(parse_map_entry(ctx)?);

            let peek = ctx.must_peek("Map destructuring")?;
            match peek.token {
                Token::Comma => {
                    ctx.advance(1);
                }
                Token::CloseBrace => break,
                _ => return Err(ctx.unexpected_token(peek, "Map destructuring")),
            }

            if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBrace)) {
                break;
            }
        }
    }

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

    Ok(Destructure {
        span: (start..end).into(),
        kind: DestructureKind::Map {
            entries,
            bind_whole,
        },
    })
}

fn string_key_expr(name: String, span: std::ops::Range<usize>) -> Expr {
    Expr {
        span: span.into(),
        kind: ExprKind::Literal(Literal::String(name.into_bytes())),
    }
}

fn parse_map_entry(ctx: &mut ParseCtx) -> ParseResult<MapDestructureEntry> {
    let peek = ctx.must_peek("Map destructuring entry")?;

    match peek.token {
        Token::OpenBracket => {
            ctx.advance(1);
            let key = parse_expression(ctx)?;
            ctx.expect(Token::CloseBracket)?;
            ctx.expect(Token::Colon)?;
            let destructure = parse_destructure(ctx)?;
            Ok(MapDestructureEntry { key, destructure })
        }
        Token::Identifier(name) => {
            let name = name.to_owned();
            let span = peek.span.clone();
            ctx.advance(1);

            let peek = ctx.must_peek("Map destructuring entry")?;
            if peek.token == Token::Colon {
                ctx.advance(1);
                let destructure = parse_destructure(ctx)?;
                Ok(MapDestructureEntry {
                    key: string_key_expr(name, span),
                    destructure,
                })
            } else {
                let binding = match name.as_str() {
                    "_" => Binding::Discarded,
                    _ => Binding::Named(name.clone()),
                };
                Ok(MapDestructureEntry {
                    key: string_key_expr(name, span.clone()),
                    destructure: Destructure {
                        span: span.into(),
                        kind: DestructureKind::Binding(binding),
                    },
                })
            }
        }
        _ => Err(ctx.unexpected_token(peek, "Map destructuring entry")),
    }
}
