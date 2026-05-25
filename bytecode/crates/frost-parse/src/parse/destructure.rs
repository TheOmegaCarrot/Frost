use crate::ast::{Binding, Destructure, DestructureKind};
use crate::lex::Token;
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
    let open = ctx.expect(Token::OpenBracket)?;
    let start = open.span.start;

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
            _ => {
                let token = peek;
                return Err(ctx.unexpected_token(token, "Array destructuring"));
            }
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
    let token = ctx.must_peek(context)?;

    match token.token {
        Token::Identifier("_") => {
            ctx.advance(1);
            Ok(Binding::Discarded)
        }
        Token::Identifier(name) => {
            let name = name.to_owned();
            ctx.advance(1);
            Ok(Binding::Named(name))
        }
        _ => Err(ctx.unexpected_token(token, context)),
    }
}

fn parse_destructure_map(ctx: &mut ParseCtx) -> ParseResult<Destructure> {
    todo!()
}
