use frost_parse::ast::*;
use frost_parse::parse_program;

pub fn parse(src: &str) -> Program {
    parse_program("test.frst", src).unwrap_or_else(|_| panic!("failed to parse: {src}"))
}

pub fn parse_expr(src: &str) -> Expr {
    let program = parse(src);
    assert_eq!(program.statements.len(), 1);
    match program.statements.into_iter().next().unwrap().kind {
        StatementKind::Expr(expr) => expr,
        other => panic!("expected Expr statement, got {other:?}"),
    }
}

pub fn parse_err(src: &str) -> String {
    parse_program("test.frst", src)
        .expect_err(&format!("expected parse error for: {src}"))
        .to_string()
}

pub fn is_int(expr: &Expr, n: i64) -> bool {
    matches!(&expr.kind, ExprKind::Literal(Literal::Int(v)) if *v == n)
}

pub fn is_binop(expr: &Expr) -> Option<(&Expr, BinOp, &Expr)> {
    match &expr.kind {
        ExprKind::BinOp { left, op, right } => Some((left, *op, right)),
        _ => None,
    }
}
