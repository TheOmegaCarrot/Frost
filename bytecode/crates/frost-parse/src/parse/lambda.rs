use crate::ast::{Binding, Expr, ExprKind, Statement, StatementKind};
use crate::lex::Token;
use crate::parse::expression::parse_expression;
use crate::parse::statements::{StatementContext, parse_statements};
use crate::parse::{ParseError, ParseResult, ctx::ParseCtx, parse_binding};

pub fn parse_lambda(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let start = ctx.expect(Token::KwFn)?.span.start;

    let peek = ctx.must_peek("lambda")?;

    let (self_name, params, variadic_param) = match peek.token {
        Token::SlimArrow => (None, Vec::new(), None),

        Token::OpenParen => {
            let (params, variadic) = parse_parenthesized_params(ctx)?;
            (None, params, variadic)
        }

        Token::DotDotDot => {
            ctx.advance(1);
            let variadic = parse_binding(ctx, "parameter name")?;
            (None, Vec::new(), Some(variadic))
        }

        Token::Identifier(name) => {
            let name = name.to_owned();
            ctx.advance(1);

            if matches!(ctx.peek().map(|t| &t.token), Some(Token::OpenParen)) {
                let (params, variadic) = parse_parenthesized_params(ctx)?;
                (Some(name), params, variadic)
            } else {
                let first = if name == "_" {
                    Binding::Discarded
                } else {
                    Binding::Named(name)
                };
                let (mut params, variadic) = parse_bare_params_tail(ctx)?;
                params.insert(0, first);
                (None, params, variadic)
            }
        }

        _ => return Err(ctx.unexpected_token(peek, "lambda")),
    };

    let (body, return_expr, end) = parse_fn_body(ctx)?;

    Ok(Expr {
        span: (start..end).into(),
        kind: ExprKind::Lambda {
            params,
            variadic_param,
            self_name,
            body,
            return_expr: Box::new(return_expr),
        },
    })
}

/// Parse `(a, b, ...rest)`. Caller has not consumed the `(`.
/// Reusable for `defn`.
pub fn parse_parenthesized_params(
    ctx: &mut ParseCtx,
) -> ParseResult<(Vec<Binding>, Option<Binding>)> {
    ctx.expect(Token::OpenParen)?;
    ctx.enter_nl_context().maybe_skip_nl();

    let mut params = Vec::new();
    let mut variadic = None;

    if !matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseParen)) {
        loop {
            ctx.maybe_skip_nl();

            if matches!(ctx.peek().map(|t| &t.token), Some(Token::DotDotDot)) {
                ctx.advance(1);
                variadic = Some(parse_binding(ctx, "parameter name")?);
                ctx.maybe_skip_nl();
                break;
            }

            params.push(parse_binding(ctx, "parameter name")?);
            ctx.maybe_skip_nl();

            let peek = ctx.must_peek("function parameters")?;
            match peek.token {
                Token::Comma => {
                    ctx.advance(1);
                    ctx.maybe_skip_nl();
                    if matches!(ctx.peek().map(|t| &t.token), Some(Token::CloseParen)) {
                        break;
                    }
                }
                Token::CloseParen => break,
                _ => return Err(ctx.unexpected_token(peek, "function parameters")),
            }
        }
    }

    ctx.maybe_skip_nl().exit_nl_context();
    ctx.expect(Token::CloseParen)?;

    Ok((params, variadic))
}

/// Parse `-> expr` or `-> { stmts; expr }`.
/// Returns `(body_stmts, return_expr, end_offset)`.
/// Reusable for `defn`.
pub fn parse_fn_body(ctx: &mut ParseCtx) -> ParseResult<(Vec<Statement>, Expr, usize)> {
    ctx.expect(Token::SlimArrow)?;
    ctx.maybe_skip_nl();

    match brace_disambiguation(ctx) {
        BraceKind::Block => parse_block_body(ctx),
        BraceKind::TryMap => {
            let checkpoint = ctx.checkpoint();
            match parse_expression(ctx) {
                Ok(expr) => {
                    let end = expr.span.end;
                    Ok((Vec::new(), expr, end))
                }
                Err(_) => {
                    ctx.restore(checkpoint);
                    parse_block_body(ctx)
                }
            }
        }
        BraceKind::Expression => {
            let expr = parse_expression(ctx)?;
            let end = expr.span.end;
            Ok((Vec::new(), expr, end))
        }
    }
}

enum BraceKind {
    Block,
    TryMap,
    Expression,
}

/// Disambiguate `{` after `->`:
/// - `{ identifier :` → map literal (expression)
/// - `{ [` → probably map, try expression first, backtrack to block on failure
/// - `{ <anything else>` → block body
/// - no `{` → plain expression
fn brace_disambiguation(ctx: &ParseCtx) -> BraceKind {
    let Some(peek) = ctx.peek() else {
        return BraceKind::Expression;
    };
    if peek.token != Token::OpenBrace {
        return BraceKind::Expression;
    }

    let pos = ctx.here();
    let second = ctx.get(pos + 1);
    let third = ctx.get(pos + 2);

    match second.map(|t| &t.token) {
        Some(Token::Identifier(_)) if matches!(third.map(|t| &t.token), Some(Token::Colon)) => {
            BraceKind::Expression
        }
        Some(Token::OpenBracket) => BraceKind::TryMap,
        _ => BraceKind::Block,
    }
}

fn parse_block_body(ctx: &mut ParseCtx) -> ParseResult<(Vec<Statement>, Expr, usize)> {
    ctx.expect(Token::OpenBrace)?;
    ctx.enter_nl_context().maybe_skip_nl();

    let mut body = parse_statements(ctx, StatementContext::Scope)?;

    ctx.maybe_skip_nl().exit_nl_context();
    let close = ctx.expect(Token::CloseBrace)?;

    let Some(last) = body.pop() else {
        return Err(ParseError::from(
            "lambda block body must contain at least one expression",
        ));
    };

    let return_expr = match last.kind {
        StatementKind::Expr(expr) => expr,
        StatementKind::Def { .. } => {
            return Err(ParseError::from(
                "lambda block body must end with an expression, not a definition",
            ));
        }
    };

    Ok((body, return_expr, close.span.end))
}

/// Parse the tail of a bare param list: `[, param]* [, ...rest]`.
/// Stops at `->` (which is not consumed).
fn parse_bare_params_tail(
    ctx: &mut ParseCtx,
) -> ParseResult<(Vec<Binding>, Option<Binding>)> {
    let mut params = Vec::new();
    let mut variadic = None;

    while matches!(ctx.peek().map(|t| &t.token), Some(Token::Comma)) {
        ctx.advance(1);

        if matches!(ctx.peek().map(|t| &t.token), Some(Token::DotDotDot)) {
            ctx.advance(1);
            variadic = Some(parse_binding(ctx, "parameter name")?);
            break;
        }

        params.push(parse_binding(ctx, "parameter name")?);
    }

    Ok((params, variadic))
}
