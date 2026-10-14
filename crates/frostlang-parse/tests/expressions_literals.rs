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

// -- Digit separators --
// `_` may appear among the digits of any number literal, following Rust's rules,
// and never changes the value.

fn is_float(expr: &Spanned<Expr>, x: f64) -> bool {
    matches!(&expr.node, Expr::Literal(Literal::Float(f)) if *f == x)
}

fn assert_one_literal(source: &str, expr: &Spanned<Expr>) {
    assert_eq!(
        (expr.span.start, expr.span.end),
        (0, source.len()),
        "{source:?} is one literal"
    );
}

#[test]
fn decimal_int_separators_do_not_change_the_value() {
    for (source, n) in [
        ("1_000", 1000),
        ("1_000_000", 1_000_000),
        ("1_0", 10),
        ("12_34_5", 12345),
        ("1__2", 12),
        ("1___000", 1000),
        // A trailing separator is allowed.
        ("1_", 1),
        ("1__", 1),
        ("0_", 0),
        ("0_0", 0),
        ("100_", 100),
    ] {
        let expr = parse_expr(source);
        assert!(is_int(&expr, n), "{source:?} is {n}: {expr:?}");
        assert_one_literal(source, &expr);
    }
}

#[test]
fn based_int_separators_do_not_change_the_value() {
    for (source, n) in [
        ("0xff_ff", 0xffff),
        ("0xDEAD_BEEF", 0xdead_beef),
        ("0x_ff", 255),
        ("0x__ff", 255),
        ("0xf_", 15),
        ("0x_f_", 15),
        ("0X_1_0", 16),
        ("0b1111_0000", 240),
        ("0b_1", 1),
        ("0b1_", 1),
        ("0B1__0", 2),
        ("0b_0_", 0),
        ("0o_7_7", 63),
        ("0o7_7", 63),
        ("0o17_", 15),
        ("0O_6_4_4", 0o644),
    ] {
        let expr = parse_expr(source);
        assert!(is_int(&expr, n), "{source:?} is {n}: {expr:?}");
        assert_one_literal(source, &expr);
    }
}

#[test]
fn float_separators_do_not_change_the_value() {
    for (source, x) in [
        // In the integer part.
        ("1_000.5", 1000.5),
        ("1_.5", 1.5),
        ("1__0.5", 10.5),
        // In the fraction.
        ("1_000.000_1", 1000.0001),
        ("1.5_", 1.5),
        ("1.5_0", 1.50),
        ("1.0_5", 1.05),
        // Leading-dot Floats.
        (".1_0", 0.10),
        (".5_", 0.5),
        (".1_0_1", 0.101),
        // In and around the exponent.
        ("1e1_0", 1e10),
        ("1e_5", 1e5),
        ("1e+_5", 1e5),
        ("1E-_1", 1e-1),
        ("1e-1_0", 1e-10),
        ("1e5_", 1e5),
        ("1_e5", 1e5),
        ("1__e5", 1e5),
        ("1.5_e2", 150.0),
        ("1.5e_2_", 150.0),
        (".5e_2", 50.0),
        (".5_e2", 50.0),
        ("1_0e1_0", 10e10),
    ] {
        let expr = parse_expr(source);
        assert!(is_float(&expr, x), "{source:?} is {x}: {expr:?}");
        assert_one_literal(source, &expr);
    }
}

#[test]
fn negation_applies_to_a_literal_with_separators() {
    // Away from the least Int, `-` is the Negate operator on the literal.
    for (source, magnitude) in [("-1_000", 1000), ("-0xff_ff", 0xffff), ("-0b1_0", 2)] {
        let expr = parse_expr(source);
        let Expr::UnaryOp { op, operand } = &expr.node else {
            panic!("{source:?}: expected a negation, got {expr:?}");
        };
        assert_eq!(op.node, UnaryOp::Negate, "{source:?}");
        assert!(is_int(operand, magnitude), "{source:?}: {operand:?}");
    }
    let expr = parse_expr("-1_000.5");
    let Expr::UnaryOp { operand, .. } = &expr.node else {
        panic!("expected a negation, got {expr:?}");
    };
    assert!(is_float(operand, 1000.5), "{operand:?}");
}

#[test]
fn int_range_ends_with_separators() {
    for (source, n) in [
        ("9_223_372_036_854_775_807", i64::MAX),
        ("9_223_372_036_854_775_807_", i64::MAX),
        ("-9_223_372_036_854_775_808", i64::MIN),
        ("-9_2_2_3_372_036_854_775_808", i64::MIN),
        ("0x7fff_ffff_ffff_ffff", i64::MAX),
        ("0x_7fff_ffff_ffff_ffff_", i64::MAX),
        ("-0x8000_0000_0000_0000", i64::MIN),
        ("0o777_777_777_777_777_777_777", i64::MAX),
        ("-0o1_000_000_000_000_000_000_000", i64::MIN),
        (
            "0b111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111",
            i64::MAX,
        ),
        (
            "-0b1000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000",
            i64::MIN,
        ),
    ] {
        let expr = parse_expr(source);
        assert!(is_int(&expr, n), "{source:?} is {n}: {expr:?}");
        assert_one_literal(source, &expr);
    }
}

#[test]
fn least_int_with_separators_in_an_expression() {
    let expr = parse_expr("1 + -9_223_372_036_854_775_808");
    let (_, _, rhs) = is_binop(&expr).expect("a binary operation");
    assert!(is_int(rhs, i64::MIN), "{rhs:?}");
}

#[test]
fn int_literals_with_separators_out_of_range() {
    for source in [
        "9_223_372_036_854_775_808",
        "-9_223_372_036_854_775_809",
        "99_999_999_999_999_999_999_999_999",
        // The magnitude is the operand of the postfix operator, not of `-`.
        "-9_223_372_036_854_775_808.x",
        "-9_223_372_036_854_775_808[0]",
        "-9_223_372_036_854_775_808()",
        "- -9_223_372_036_854_775_808 @ f()",
        "0x8000_0000_0000_0000",
        "-0x8000_0000_0000_0001",
        "0xffff_ffff_ffff_ffff",
        "0x1_0000_0000_0000_0000",
        "0o1_000_000_000_000_000_000_000",
        "0b1000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000",
        "-0x8000_0000_0000_0000.x",
    ] {
        let err = parse_err(source);
        assert!(
            err.contains("Int literal out of range"),
            "{source:?}: {err}"
        );
    }
}

#[test]
fn separated_literals_are_operands_like_any_other() {
    let expr = parse_expr("1_000 + 0xff_ff");
    let (lhs, op, rhs) = is_binop(&expr).expect("a binary operation");
    assert!(is_int(lhs, 1000), "{lhs:?}");
    assert_eq!(op, BinOp::Add);
    assert!(is_int(rhs, 0xffff), "{rhs:?}");
    assert_eq!((lhs.span.start, lhs.span.end), (0, 5));
    assert_eq!((rhs.span.start, rhs.span.end), (8, 15));

    let expr = parse_expr("1_0 * 2_0 - 3_0");
    let (sum_lhs, _, sum_rhs) = is_binop(&expr).expect("a subtraction");
    assert!(is_int(sum_rhs, 30), "{sum_rhs:?}");
    let (a, _, b) = is_binop(sum_lhs).expect("a product");
    assert!(is_int(a, 10) && is_int(b, 20), "{a:?} {b:?}");

    let expr = parse_expr("1_.5 + 2_0");
    let (lhs, _, rhs) = is_binop(&expr).expect("a binary operation");
    assert!(is_float(lhs, 1.5), "{lhs:?}");
    assert!(is_int(rhs, 20), "{rhs:?}");
}

#[test]
fn separated_literals_in_array_literals() {
    let expr = parse_expr("[1_000, 0b1_1, 2_.5]");
    let Expr::Array(elements) = &expr.node else {
        panic!("expected an Array, got {expr:?}");
    };
    assert_eq!(elements.len(), 3, "{expr:?}");
    assert!(is_int(&elements[0], 1000), "{:?}", elements[0]);
    assert!(is_int(&elements[1], 3), "{:?}", elements[1]);
    assert!(is_float(&elements[2], 2.5), "{:?}", elements[2]);
    assert_eq!((elements[0].span.start, elements[0].span.end), (1, 6));
}

#[test]
fn separated_literals_in_map_literals() {
    let expr = parse_expr("{[1_0]: 2_0, [0x_f]: 3.0_1}");
    let Expr::Map(entries) = &expr.node else {
        panic!("expected a Map, got {expr:?}");
    };
    assert_eq!(entries.len(), 2, "{expr:?}");
    assert!(is_int(&entries[0].node.key, 10), "{:?}", entries[0]);
    assert!(is_int(&entries[0].node.value, 20), "{:?}", entries[0]);
    assert!(is_int(&entries[1].node.key, 15), "{:?}", entries[1]);
    assert!(is_float(&entries[1].node.value, 3.01), "{:?}", entries[1]);
}

#[test]
fn separated_literals_as_call_arguments() {
    let expr = parse_expr("f(1_000, 2_0, 0b1_0)");
    let Expr::Call { args, .. } = &expr.node else {
        panic!("expected a Call, got {expr:?}");
    };
    assert_eq!(args.len(), 3, "{expr:?}");
    assert!(is_int(&args[0], 1000), "{:?}", args[0]);
    assert!(is_int(&args[1], 20), "{:?}", args[1]);
    assert!(is_int(&args[2], 2), "{:?}", args[2]);
}

#[test]
fn separated_literals_as_index_expressions() {
    let expr = parse_expr("xs[1_0]");
    let Expr::SoftIndex { key, .. } = &expr.node else {
        panic!("expected an index, got {expr:?}");
    };
    assert!(is_int(key, 10), "{key:?}");
}

#[test]
fn separated_literals_in_format_string_interpolations() {
    let expr = parse_expr(r"$'a${1_000}b${0x_ff + 1_}'");
    let Expr::FormatString(segments) = &expr.node else {
        panic!("expected a FormatString, got {expr:?}");
    };
    let interpolations: Vec<_> = segments
        .iter()
        .filter_map(|segment| match segment {
            FormatSegment::Interpolation(e) => Some(e),
            FormatSegment::Literal(_) => None,
        })
        .collect();
    assert_eq!(interpolations.len(), 2, "{segments:?}");
    assert!(is_int(interpolations[0], 1000), "{:?}", interpolations[0]);
    let (lhs, _, rhs) = is_binop(interpolations[1]).expect("a binary operation");
    assert!(is_int(lhs, 255) && is_int(rhs, 1), "{lhs:?} {rhs:?}");

    let err = parse_err(r"$'${9_223_372_036_854_775_808}'");
    assert!(err.contains("Int literal out of range"), "{err}");
}

#[test]
fn underscore_after_a_dot_is_a_field_not_a_digit_separator() {
    // `1._5` is the field `_5` of the Int 1, never a Float.
    let expr = parse_expr("1._5");
    let Expr::HardIndex { target, key } = &expr.node else {
        panic!("expected a field access, got {expr:?}");
    };
    assert!(is_int(target, 1), "{target:?}");
    assert_eq!(key.node, "_5");

    let expr = parse_expr("x._5");
    let Expr::HardIndex { target, key } = &expr.node else {
        panic!("expected a field access, got {expr:?}");
    };
    assert!(
        matches!(&target.node, Expr::NameLookup(n) if n == "x"),
        "{target:?}"
    );
    assert_eq!(key.node, "_5");

    // With the separator before the point, the same digits are a Float.
    let expr = parse_expr("1_.5");
    assert!(is_float(&expr, 1.5), "{expr:?}");
    assert_one_literal("1_.5", &expr);
}

#[test]
fn separated_digits_after_a_dot_do_not_index_like_a_field() {
    // `x.1_0` fails as `x.1` does: Frost has no tuple-style indexing.
    for source in ["x.1_0", "x.1"] {
        parse_err(source);
    }
}

#[test]
fn leading_underscore_before_a_dot_is_not_a_float() {
    // `._5` has no digit before the underscore, so it is not a Float.
    let err = parse_err("._5");
    assert!(!err.is_empty());
}
