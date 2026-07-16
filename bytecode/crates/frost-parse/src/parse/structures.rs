use crate::ast::{Expr, Literal, MapEntry, Spanned};
use crate::lex::Token;
use crate::parse::expression::parse_expression;
use crate::parse::{ParseResult, ctx::ParseCtx};

pub fn parse_array_literal(ctx: &mut ParseCtx) -> ParseResult<Spanned<Expr>> {
    let start = ctx.expect(Token::OpenBracket)?.span.start;
    ctx.enter_nl_context();

    let (elements, close) =
        ctx.parse_comma_separated(Token::CloseBracket, "Array literal", |ctx| {
            parse_expression(ctx)
        })?;

    Ok(Spanned::new(
        Expr::Array(elements),
        (start..close.span.end).into(),
    ))
}

pub fn parse_map_literal(ctx: &mut ParseCtx) -> ParseResult<Spanned<Expr>> {
    let start = ctx.expect(Token::OpenBrace)?.span.start;
    ctx.enter_nl_context();

    let (entries, close) =
        ctx.parse_comma_separated(Token::CloseBrace, "Map literal", parse_map_entry)?;

    Ok(Spanned::new(
        Expr::Map(entries),
        (start..close.span.end).into(),
    ))
}

fn parse_map_entry(ctx: &mut ParseCtx) -> ParseResult<Spanned<MapEntry>> {
    let peek = ctx.must_peek("Map entry")?;
    let start = peek.span.start;

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
            Spanned::new(
                Expr::Literal(Literal::String(name.into_bytes())),
                span.into(),
            )
        }
        _ => return Err(ctx.unexpected_token(peek, "Map entry key")),
    };

    ctx.expect(Token::Colon)?;
    ctx.maybe_skip_nl();
    let value = parse_expression(ctx)?;

    let span = (start..value.span.end).into();
    Ok(Spanned::new(MapEntry { key, value }, span))
}
