use crate::ast::{Expr, ExprKind, Literal, SourceSpan};
use crate::lex::Token;
use crate::parse::{ParseResult, ctx::ParseCtx};

pub fn parse_expression(ctx: &mut ParseCtx) -> ParseResult<Expr> {
    let token = ctx.must_peek("expression")?;

    match token.token {
        Token::IntLiteral(n) => {
            let span = token.span.clone();
            ctx.advance(1);
            Ok(Expr {
                span: span.into(),
                kind: ExprKind::Literal(Literal::Int(n)),
            })
        }
        _ => todo!(),
    }
}
