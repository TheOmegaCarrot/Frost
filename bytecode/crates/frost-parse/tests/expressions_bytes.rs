mod helpers;

use frost_parse::ast::*;
use helpers::*;

/// The literal's bytes.
fn bytes(expr: &Spanned<Expr>) -> &[u8] {
    match &expr.node {
        Expr::Literal(Literal::Bytes(b)) => b,
        other => panic!("expected Bytes literal, got {other:?}"),
    }
}

#[test]
fn single_quote_hex_pairs() {
    let expr = parse_expr("x'6869'");
    assert_eq!(bytes(&expr), &[0x68, 0x69]);
}

#[test]
fn double_quote_hex_pairs() {
    let expr = parse_expr(r#"x"6869""#);
    assert_eq!(bytes(&expr), &[0x68, 0x69]);
}

#[test]
fn empty_bytes_literal() {
    let expr = parse_expr("x''");
    assert!(bytes(&expr).is_empty());
}

#[test]
fn high_bytes_decode() {
    // Bytes carries arbitrary octets, including those no String could hold.
    let expr = parse_expr("x'80ff'");
    assert_eq!(bytes(&expr), &[0x80, 0xff]);
}

#[test]
fn hex_is_case_insensitive() {
    // Upper- and lowercase name the same octets.
    let upper = parse_expr("x'FF00'");
    let lower = parse_expr("x'ff00'");
    assert_eq!(bytes(&upper), &[0xff, 0x00]);
    assert_eq!(bytes(&upper), bytes(&lower));
}
