use crate::ast::{Expr, ExprKind, Literal, MapEntry};
use crate::lex::Token;
use crate::parse::expression::parse_expression;
use crate::parse::{ParseResult, ctx::ParseCtx};

pub fn parse_array_literal(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let start = ctx.expect(Token::OpenBracket)?.span.start;
    ctx.enter_nl_context();

    let (elements, close) =
        ctx.parse_comma_separated(Token::CloseBracket, "Array literal", |ctx| {
            parse_expression(ctx)
        })?;

    Ok(Expr {
        span: (start..close.span.end).into(),
        kind: ExprKind::Array(elements),
    })
}

pub fn parse_map_literal(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let start = ctx.expect(Token::OpenBrace)?.span.start;
    ctx.enter_nl_context();

    let (entries, close) =
        ctx.parse_comma_separated(Token::CloseBrace, "Map literal", parse_map_entry)?;

    Ok(Expr {
        span: (start..close.span.end).into(),
        kind: ExprKind::Map(entries),
    })
}

fn parse_map_entry(ctx: &mut ParseCtx) -> ParseResult<MapEntry> {
    let peek = ctx.must_peek("Map entry")?;

    let key = match peek.token {
        Token::OpenBracket => {
            ctx.expect(Token::OpenBracket)?;
            ctx.enter_nl_context().maybe_skip_nl();
            let key = parse_expression(ctx)?;
            ctx.maybe_skip_nl().exit_nl_context();
            ctx.expect(Token::CloseBracket)?;
            key
        }
        Token::Identifier(name) => {
            let name = name.to_owned();
            let span = peek.span.clone();
            ctx.advance(1);
            Expr {
                span: span.into(),
                kind: ExprKind::Literal(Literal::String(name.into_bytes())),
            }
        }
        _ => return Err(ctx.unexpected_token(peek, "Map entry key")),
    };

    ctx.expect(Token::Colon)?;
    ctx.maybe_skip_nl();
    let value = parse_expression(ctx)?;

    Ok(MapEntry { key, value })
}
