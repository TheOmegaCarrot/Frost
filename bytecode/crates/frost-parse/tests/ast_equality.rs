//! Tests for AST equality: `==` compares structure and ignores spans
//! (see the `Spanned` docs). Two parses that differ only in whitespace or
//! other span-shifting trivia are equal; structural differences are not.

mod helpers;

use helpers::*;

#[test]
fn whitespace_does_not_affect_equality() {
    assert_eq!(parse_expr("1+2"), parse_expr("1 + 2"));
    assert_eq!(parse_expr("[1,2,3]"), parse_expr("[ 1, 2, 3 ]"));
    assert_eq!(
        parse_expr("f(a, b)"),
        parse_expr("f(\n  a,\n  b,\n)"),
        "newlines and a trailing comma shift spans but not structure"
    );
}

#[test]
fn comments_do_not_affect_equality() {
    assert_eq!(
        parse("def x = 1"),
        parse("def x = 1 # a comment"),
        "comments are span-invisible trivia"
    );
}

#[test]
fn structural_differences_are_unequal() {
    assert_ne!(parse_expr("1 + 2"), parse_expr("1 - 2"));
    assert_ne!(parse_expr("1 + 2"), parse_expr("2 + 1"));
    assert_ne!(parse_expr("a and b"), parse_expr("a or b"));
    assert_ne!(parse_expr("[1, 2]"), parse_expr("[1, 2, 3]"));
}

#[test]
fn logical_and_binop_are_never_equal() {
    // `Expr::Logical` and `Expr::BinOp` are distinct variants even where an
    // operator table might conflate them.
    assert_ne!(parse_expr("a and b"), parse_expr("a == b"));
}

#[test]
fn spans_remain_explicitly_comparable() {
    // Equality ignoring spans does not erase them: position is still there
    // for whoever asks.
    let tight = parse_expr("1+2");
    let spaced = parse_expr("1 + 2");
    assert_eq!(tight, spaced);
    assert_ne!(tight.span, spaced.span);
}

#[test]
fn full_programs_compare() {
    let a = parse("def x = 1\ndef y = x + 1");
    let b = parse("def x = 1\ndef y = x + 1");
    let c = parse("def x = 1\ndef y = x + 2");
    assert_eq!(a, b);
    assert_ne!(a, c);
}
