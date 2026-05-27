use crate::ast::{Expr, ExprKind};
use crate::lex::Token;
use crate::parse::expression::parse_expression;
use crate::parse::{ParseResult, ctx::ParseCtx};

pub fn parse_array_literal(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let start = ctx.expect(Token::OpenBracket)?.span.start;
    ctx.enter_nl_context().maybe_skip_nl();

    let mut elements = Vec::new();

    if !matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
        loop {
            ctx.maybe_skip_nl();
            elements.push(parse_expression(ctx)?);
            ctx.maybe_skip_nl();

            let peek = ctx.must_peek("Array literal")?;
            match peek.token {
                Token::Comma => { ctx.expect(Token::Comma)?; }
                Token::CloseBracket => break,
                _ => return Err(ctx.unexpected_token(peek, "Array literal")),
            }

            ctx.maybe_skip_nl();
            if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseBracket)) {
                break;
            }
        }
    }

    ctx.maybe_skip_nl().exit_nl_context();
    let close = ctx.expect(Token::CloseBracket)?;

    Ok(Expr {
        span: (start..close.span.end).into(),
        kind: ExprKind::Array(elements),
    })
}
