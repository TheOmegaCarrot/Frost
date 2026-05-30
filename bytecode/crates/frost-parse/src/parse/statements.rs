use crate::ast::{Binding, Destructure, DestructureKind, ExprKind, Statement, StatementKind};
use crate::lex::Token;
use crate::parse::destructure::parse_destructure;
use crate::parse::expression::parse_expression;
use crate::parse::lambda::{parse_fn_body, parse_parenthesized_params};
use crate::parse::{ParseResult, ctx::ParseCtx, parse_binding};

/// Statements are only allowed in a few contexts,
/// and the rules differ between contexts.
pub enum StatementContext {
    TopLevel,
    Scope,
}

pub fn parse_statements(ctx: &mut ParseCtx, kind: StatementContext) -> ParseResult<Vec<Statement>> {
    let mut stmts = Vec::new();

    let allow_export = matches!(kind, StatementContext::TopLevel);
    let in_a_scope = !allow_export;

    while let Some(peek) = ctx.peek() {
        match peek.token {
            // Newlines between statements should be skipped.
            // Doing it at the top of the loop skips blank/comment lines at the top of a file.
            Token::Newline => {
                ctx.advance(1);
                continue;
            }
            Token::CloseBrace if in_a_scope => break,
            _ => {}
        }

        match peek.token {
            Token::KwExport if allow_export => {
                let next = ctx.get(ctx.here() + 1).map(|t| &t.token);
                if matches!(next, Some(Token::KwDefn)) {
                    stmts.push(parse_defn(ctx, true)?);
                } else {
                    stmts.push(parse_def(ctx, true)?);
                }
            }
            Token::KwDef => stmts.push(parse_def(ctx, false)?),
            Token::KwDefn => stmts.push(parse_defn(ctx, false)?),
            _ => {
                let expr = parse_expression(ctx)?;

                stmts.push(Statement {
                    span: expr.span,
                    kind: StatementKind::Expr(expr),
                })
            }
        }

        if let Some(peek) = ctx.peek() {
            if in_a_scope && peek.token == Token::CloseBrace {
                break;
            }
            match peek.token {
                Token::Semicolon | Token::Newline => {
                    ctx.advance(1);
                    ctx.maybe_skip_nl();
                }
                _ => {
                    return Err(ctx.unexpected_token(
                        peek,
                        "expected line break or semicolon after complete statement",
                    ));
                }
            };
        }
    }

    Ok(stmts)
}

fn parse_def(ctx: &mut ParseCtx, exported: bool) -> ParseResult<Statement> {
    let start = ctx.must_peek("definition")?.span.start;

    if exported {
        ctx.expect(Token::KwExport)?;
    }

    ctx.expect(Token::KwDef)?;

    let destructure = parse_destructure(ctx)?;

    ctx.expect(Token::Assign)?;

    let expr = parse_expression(ctx)?;

    Ok(Statement {
        span: (start..expr.span.end).into(),
        kind: StatementKind::Def {
            exported,
            destructure,
            expr,
        },
    })
}

fn parse_defn(ctx: &mut ParseCtx, exported: bool) -> ParseResult<Statement> {
    let start = ctx.must_peek("function definition")?.span.start;

    if exported {
        ctx.expect(Token::KwExport)?;
    }

    ctx.expect(Token::KwDefn)?;

    let name = match parse_binding(ctx, "function name")? {
        Binding::Named(name) => name,
        Binding::Discarded => {
            return Err("defn requires a function name, not '_'".into());
        }
    };

    let (params, variadic_param) = parse_parenthesized_params(ctx)?;
    let (body, return_expr, end) = parse_fn_body(ctx)?;

    let expr = crate::ast::Expr {
        span: (start..end).into(),
        kind: ExprKind::Lambda {
            params,
            variadic_param,
            self_name: Some(name.clone()),
            body,
            return_expr: Box::new(return_expr),
        },
    };

    Ok(Statement {
        span: (start..end).into(),
        kind: StatementKind::Def {
            exported,
            destructure: Destructure {
                span: (start..end).into(),
                kind: DestructureKind::Binding(Binding::Named(name)),
            },
            expr,
        },
    })
}
