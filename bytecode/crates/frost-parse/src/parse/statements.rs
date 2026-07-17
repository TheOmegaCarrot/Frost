use crate::ast::{Binding, Destructure, Expr, SourceSpan, Spanned, Statement};
use crate::lex::Token;
use crate::parse::destructure::parse_destructure;
use crate::parse::expression::parse_expression;
use crate::parse::lambda::{parse_fn_body, parse_parenthesized_params};
use crate::parse::{Diagnostic, ParseResult, ctx::ParseCtx, parse_binding};

/// Statements are only allowed in a few contexts,
/// and the rules differ between contexts.
pub enum StatementContext {
    TopLevel,
    Scope,
}

pub fn parse_statements(
    ctx: &mut ParseCtx,
    kind: StatementContext,
) -> ParseResult<Vec<Spanned<Statement>>> {
    let mut stmts = Vec::new();

    let allow_export = matches!(kind, StatementContext::TopLevel);
    let in_a_scope = !allow_export;

    while let Some(peek) = ctx.peek() {
        match peek.token {
            // Newlines between statements should be skipped.
            // Doing it at the top of the loop skips blank/comment lines at the top of a file.
            Token::Newline | Token::Semicolon => {
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
                let span = expr.span;
                stmts.push(Spanned::new(Statement::Expr(expr), span));
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

fn parse_def(ctx: &mut ParseCtx, exported: bool) -> ParseResult<Spanned<Statement>> {
    let start = ctx.must_peek("definition")?.span.start;

    if exported {
        ctx.expect(Token::KwExport)?;
    }

    ctx.expect(Token::KwDef)?;

    let destructure = parse_destructure(ctx)?;

    ctx.expect(Token::Assign)?;

    let expr = parse_expression(ctx)?;
    let end = expr.span.end;

    Ok(Spanned::new(
        Statement::Def {
            exported,
            destructure,
            expr,
        },
        (start..end).into(),
    ))
}

fn parse_defn(ctx: &mut ParseCtx, exported: bool) -> ParseResult<Spanned<Statement>> {
    let defn_start = ctx.must_peek("function definition")?.span.start;

    if exported {
        ctx.expect(Token::KwExport)?;
    }

    ctx.expect(Token::KwDefn)?;

    // Not yet checked to be a name, but parse_binding below will error if this isn't the case.
    let name_span: SourceSpan = ctx.must_peek("defn function name")?.span.clone().into();

    let name = match parse_binding(ctx, "function name")?.node {
        Binding::Named(name) => name,
        Binding::Discarded => {
            return Err(Diagnostic::at(
                "defn requires a function name, not '_'",
                name_span,
                "expected a name",
            ));
        }
    };

    let (params, variadic_param) = parse_parenthesized_params(ctx)?;
    let (body, return_expr, end) = parse_fn_body(ctx)?;

    let expr = Spanned::new(
        Expr::Lambda {
            params,
            variadic_param,
            self_name: Some(Spanned::new(name.clone(), name_span)),
            body,
            return_expr: Box::new(return_expr),
        },
        (name_span.start..end).into(),
    );

    Ok(Spanned::new(
        Statement::Def {
            exported,
            destructure: Spanned::new(
                Destructure::Binding(Spanned::new(Binding::Named(name), name_span)),
                name_span,
            ),
            expr,
        },
        (defn_start..end).into(),
    ))
}
