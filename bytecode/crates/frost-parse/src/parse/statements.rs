use crate::ParseError;
use crate::ast;
use crate::ast::SourceSpan;
use crate::ast::Statement;
use crate::ast::StatementKind;
use crate::lex::Token;
use crate::parse::destructure::parse_destructure;
use crate::parse::expression::parse_expression;
use crate::parse::{ParseResult, ctx::ParseCtx};

/// Statements are only allowed in a few contexts,
/// and the rules differ between contexts.
pub enum StatementContext {
    TopLevel,
    Scope,
}

pub fn parse_statements(ctx: &mut ParseCtx, kind: StatementContext) -> ParseResult<Vec<Statement>> {
    let mut stmts = Vec::new();

    while let Some(peek) = ctx.peek() {
        match peek.token {
            Token::KwExport => stmts.push(parse_def(ctx, true)?),
            Token::KwDef => stmts.push(parse_def(ctx, false)?),
            _ => {
                let expr = parse_expression(ctx)?;

                stmts.push(Statement {
                    span: expr.span,
                    kind: StatementKind::Expr(expr),
                })
            }
        }
    }

    Ok(stmts)
}

fn parse_def(ctx: &mut ParseCtx, exported: bool) -> ParseResult<Statement> {
    let first_token_pos = ctx.here();

    if exported {
        ctx.expect(Token::KwExport)?;
    }

    ctx.expect(Token::KwDef)?;

    let destructure = parse_destructure(ctx)?;

    ctx.expect(Token::Assign)?;

    let expr = parse_expression(ctx)?;

    Ok(Statement {
        span: SourceSpan {
            start: ctx.get(first_token_pos).expect("IMPOSSIBLE: OOB access after check").span.start,
            end: expr.span.end,
        },
        kind: StatementKind::Def {
            exported,
            destructure,
            expr,
        },
    })
}
