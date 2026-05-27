mod helpers;

use frost_parse::ast::*;
use helpers::*;

// -- Literals --

#[test]
fn int_literal() {
    let expr = parse_expr("42");
    assert!(is_int(&expr, 42));
}

#[test]
fn float_literal() {
    let expr = parse_expr("3.14");
    assert!(matches!(&expr.kind, ExprKind::Literal(Literal::Float(f)) if *f == 3.14));
}

#[test]
fn bool_true() {
    let expr = parse_expr("true");
    assert!(matches!(&expr.kind, ExprKind::Literal(Literal::Bool(true))));
}

#[test]
fn bool_false() {
    let expr = parse_expr("false");
    assert!(matches!(
        &expr.kind,
        ExprKind::Literal(Literal::Bool(false))
    ));
}

#[test]
fn null_literal() {
    let expr = parse_expr("null");
    assert!(matches!(&expr.kind, ExprKind::Literal(Literal::Null)));
}

#[test]
fn identifier() {
    let expr = parse_expr("foo");
    assert!(matches!(&expr.kind, ExprKind::NameLookup(n) if n == "foo"));
}

#[test]
fn bare_identifier_expression() {
    let expr = parse_expr("x");
    assert!(matches!(&expr.kind, ExprKind::NameLookup(n) if n == "x"));
}

// -- Float edge cases --

#[test]
fn float_leading_dot() {
    let expr = parse_expr(".5");
    assert!(matches!(&expr.kind, ExprKind::Literal(Literal::Float(f)) if *f == 0.5));
}

#[test]
fn float_scientific() {
    let expr = parse_expr("1e10");
    assert!(matches!(&expr.kind, ExprKind::Literal(Literal::Float(f)) if *f == 1e10));
}

#[test]
fn float_scientific_negative_exponent() {
    let expr = parse_expr("3.14e-2");
    assert!(matches!(&expr.kind, ExprKind::Literal(Literal::Float(f)) if (*f - 3.14e-2).abs() < 1e-15));
}

// -- Program-level --

#[test]
fn empty_input_is_valid() {
    let program = parse("");
    assert!(program.statements.is_empty());
}
