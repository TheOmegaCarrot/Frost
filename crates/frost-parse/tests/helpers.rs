// Shared across test binaries via `mod helpers;`; no single binary uses every helper.
#![allow(dead_code)]

use frost_parse::ast::*;
use frost_parse::parse_program;

pub(crate) fn parse(src: &str) -> Program {
    parse_program("test.frst", src).unwrap_or_else(|_| panic!("failed to parse: {src}"))
}

pub(crate) fn parse_expr(src: &str) -> Spanned<Expr> {
    let program = parse(src);
    assert_eq!(program.statements.len(), 1);
    match program.statements.into_iter().next().unwrap().node {
        Statement::Expr(expr) => expr,
        other => panic!("expected Expr statement, got {other:?}"),
    }
}

pub(crate) fn parse_err(src: &str) -> String {
    parse_program("test.frst", src)
        .expect_err(&format!("expected parse error for: {src}"))
        .to_string()
}

pub(crate) fn is_int(expr: &Spanned<Expr>, n: i64) -> bool {
    matches!(&expr.node, Expr::Literal(Literal::Int(v)) if *v == n)
}

pub(crate) fn is_binop(expr: &Spanned<Expr>) -> Option<(&Spanned<Expr>, BinOp, &Spanned<Expr>)> {
    match &expr.node {
        Expr::BinOp { left, op, right } => Some((left, op.node, right)),
        _ => None,
    }
}

pub(crate) fn is_logical(expr: &Spanned<Expr>) -> Option<(&Spanned<Expr>, LogicalOp, &Spanned<Expr>)> {
    match &expr.node {
        Expr::Logical { left, op, right } => Some((left, op.node, right)),
        _ => None,
    }
}
