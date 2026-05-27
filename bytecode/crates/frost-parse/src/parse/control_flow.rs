use crate::ast::{Expr, ExprKind};
use crate::lex::Token;
use crate::parse::expression::parse_expression;
use crate::parse::{ParseResult, ctx::ParseCtx};

pub fn parse_if(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let start = ctx.expect(Token::KwIf)?.span.start;

    let condition = parse_expression(ctx)?;
    ctx.expect(Token::Colon)?;
    let consequent = parse_expression(ctx)?;

    let alternate = parse_tail(ctx)?;

    let end = alternate.as_ref().unwrap_or(&consequent).span.end;

    Ok(Expr {
        span: (start..end).into(),
        kind: ExprKind::If {
            condition: Box::new(condition),
            consequent: Box::new(consequent),
            alternate: alternate.map(Box::new),
        },
    })
}

fn parse_tail(ctx: &mut ParseCtx) -> ParseResult<Option<Expr>> {
    let checkpoint = ctx.checkpoint();
    ctx.enter_nl_context().maybe_skip_nl().exit_nl_context();

    // An `if` without an `else` or `elif` is permitted to be the end of input.
    let Some(peek) = ctx.peek() else {
        ctx.restore(checkpoint);
        return Ok(None);
    };

    match peek.token {
        Token::KwElif => {
            let elif = parse_if_from_elif(ctx)?;
            Ok(Some(elif))
        }
        Token::KwElse => {
            ctx.expect(Token::KwElse)?;
            ctx.expect(Token::Colon)?;
            let alternate = parse_expression(ctx)?;
            Ok(Some(alternate))
        }
        _ => {
            ctx.restore(checkpoint);
            Ok(None)
        }
    }
}

fn parse_if_from_elif(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let start = ctx.expect(Token::KwElif)?.span.start;

    let condition = parse_expression(ctx)?;
    ctx.expect(Token::Colon)?;
    let consequent = parse_expression(ctx)?;

    let alternate = parse_tail(ctx)?;

    let end = alternate.as_ref().unwrap_or(&consequent).span.end;

    Ok(Expr {
        span: (start..end).into(),
        kind: ExprKind::If {
            condition: Box::new(condition),
            consequent: Box::new(consequent),
            alternate: alternate.map(Box::new),
        },
    })
}
