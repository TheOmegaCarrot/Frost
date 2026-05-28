use crate::ast::{Expr, ExprKind, StatementKind};
use crate::lex::Token;
use crate::parse::expression::parse_expression;
use crate::parse::statements::{StatementContext, parse_statements};
use crate::parse::{ParseError, ParseResult, ctx::ParseCtx};

pub fn parse_if(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    parse_if_or_elif(ctx, Token::KwIf)
}

fn parse_if_or_elif(ctx: &mut ParseCtx, keyword: Token) -> ParseResult<Expr> {
    let start = ctx.expect(keyword)?.span.start;

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

    let Some(peek) = ctx.peek() else {
        ctx.restore(checkpoint);
        return Ok(None);
    };

    match peek.token {
        Token::KwElif => Ok(Some(parse_if_or_elif(ctx, Token::KwElif)?)),
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

pub fn parse_do(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let start = ctx.expect(Token::KwDo)?.span.start;
    ctx.expect(Token::OpenBrace)?;
    ctx.enter_nl_context().maybe_skip_nl();

    let mut body = parse_statements(ctx, StatementContext::Scope)?;

    ctx.maybe_skip_nl().exit_nl_context();
    let close = ctx.expect(Token::CloseBrace)?;

    let Some(last) = body.pop() else {
        return Err(ParseError::from("do block must contain at least one expression"));
    };

    let value = match last.kind {
        StatementKind::Expr(expr) => expr,
        StatementKind::Def { .. } => {
            return Err(ParseError::from("do block must end with an expression, not a definition"));
        }
    };

    Ok(Expr {
        span: (start..close.span.end).into(),
        kind: ExprKind::Do {
            body,
            value: Box::new(value),
        },
    })
}
