use crate::helpers;

use frostlang_parse::ast::*;
use helpers::*;

// -- Literals --

#[test]
fn int_literal() {
    let expr = parse_expr("42");
    assert!(is_int(&expr, 42));
}

#[test]
fn greatest_int_literal() {
    let expr = parse_expr("9223372036854775807");
    assert!(is_int(&expr, i64::MAX));
}

#[test]
fn least_int_literal() {
    // Its magnitude is past the greatest Int, so it is a literal only negated,
    // parsed as one negative literal rather than a negation.
    let expr = parse_expr("-9223372036854775808");
    assert!(is_int(&expr, i64::MIN), "{expr:?}");
    assert_eq!((expr.span.start, expr.span.end), (0, 20));
}

#[test]
fn least_int_literal_in_an_expression() {
    let expr = parse_expr("1 + -9223372036854775808");
    let (_, _, rhs) = is_binop(&expr).expect("a binary operation");
    assert!(is_int(rhs, i64::MIN), "{rhs:?}");
}

#[test]
fn int_literal_out_of_range() {
    for source in [
        "9223372036854775808",
        "-9223372036854775809",
        "99999999999999999999999999",
        // The magnitude is the operand of the postfix operator, not of `-`.
        "-9223372036854775808.x",
        "-9223372036854775808[0]",
        "-9223372036854775808()",
        "- -9223372036854775808 @ f()",
    ] {
        let err = parse_err(source);
        assert!(
            err.contains("Int literal out of range"),
            "{source:?}: {err}"
        );
    }
}

// -- Int literals in other bases --
// `0b`, `0o`, and `0x` prefixes, each in either case; hex digits in either case.

#[test]
fn based_int_literals_denote_the_value_written() {
    for (source, n) in [
        ("0b0", 0),
        ("0b1", 1),
        ("0b101", 5),
        ("0B101", 5),
        ("0o0", 0),
        ("0o17", 15),
        ("0O644", 0o644),
        ("0x0", 0),
        ("0xff", 255),
        ("0xFF", 255),
        ("0XfF", 255),
        ("0xDeadBeef", 0xdead_beef),
        // Leading zeros change nothing.
        ("0x00ff", 255),
        ("0b0001", 1),
        ("0o007", 7),
    ] {
        let expr = parse_expr(source);
        assert!(is_int(&expr, n), "{source:?} is {n}: {expr:?}");
    }
}

#[test]
fn based_int_literals_at_the_ends_of_the_range() {
    for (source, n) in [
        ("0x7fffffffffffffff", i64::MAX),
        ("0o777777777777777777777", i64::MAX),
        (
            "0b111111111111111111111111111111111111111111111111111111111111111",
            i64::MAX,
        ),
        // As in decimal, the least Int's magnitude is a literal only when negated.
        ("-0x8000000000000000", i64::MIN),
        ("-0o1000000000000000000000", i64::MIN),
        (
            "-0b1000000000000000000000000000000000000000000000000000000000000000",
            i64::MIN,
        ),
    ] {
        let expr = parse_expr(source);
        assert!(is_int(&expr, n), "{source:?} is {n}: {expr:?}");
        assert_eq!(
            (expr.span.start, expr.span.end),
            (0, source.len()),
            "{source:?} is one literal"
        );
    }
}

#[test]
fn based_int_literals_out_of_range() {
    for source in [
        "0x8000000000000000",
        "-0x8000000000000001",
        "0xffffffffffffffff",
        // Past what 64 bits hold at all.
        "0x10000000000000000",
        "0o1000000000000000000000",
        "0b1000000000000000000000000000000000000000000000000000000000000000",
        "-0x8000000000000000.x",
    ] {
        let err = parse_err(source);
        assert!(
            err.contains("Int literal out of range"),
            "{source:?}: {err}"
        );
    }
}

#[test]
fn based_int_literals_are_operands_like_any_other() {
    let expr = parse_expr("0x10 + 0b11");
    let (lhs, _, rhs) = is_binop(&expr).expect("a binary operation");
    assert!(is_int(lhs, 16), "{lhs:?}");
    assert!(is_int(rhs, 3), "{rhs:?}");
}

#[test]
fn float_literal() {
    let expr = parse_expr("3.14");
    assert!(matches!(&expr.node, Expr::Literal(Literal::Float(f)) if *f == 3.14));
}

#[test]
fn bool_true() {
    let expr = parse_expr("true");
    assert!(matches!(&expr.node, Expr::Literal(Literal::Bool(true))));
}

#[test]
fn bool_false() {
    let expr = parse_expr("false");
    assert!(matches!(&expr.node, Expr::Literal(Literal::Bool(false))));
}

#[test]
fn null_literal() {
    let expr = parse_expr("null");
    assert!(matches!(&expr.node, Expr::Literal(Literal::Null)));
}

#[test]
fn identifier() {
    let expr = parse_expr("foo");
    assert!(matches!(&expr.node, Expr::NameLookup(n) if n == "foo"));
}

#[test]
fn bare_identifier_expression() {
    let expr = parse_expr("x");
    assert!(matches!(&expr.node, Expr::NameLookup(n) if n == "x"));
}

// -- Float edge cases --

#[test]
fn float_leading_dot() {
    let expr = parse_expr(".5");
    assert!(matches!(&expr.node, Expr::Literal(Literal::Float(f)) if *f == 0.5));
}

#[test]
fn float_scientific() {
    let expr = parse_expr("1e10");
    assert!(matches!(&expr.node, Expr::Literal(Literal::Float(f)) if *f == 1e10));
}

#[test]
fn float_scientific_negative_exponent() {
    let expr = parse_expr("3.14e-2");
    assert!(matches!(&expr.node, Expr::Literal(Literal::Float(f)) if (*f - 3.14e-2).abs() < 1e-15));
}

// -- Program-level --

#[test]
fn empty_input_is_valid() {
    let program = parse("");
    assert!(program.statements.is_empty());
}
