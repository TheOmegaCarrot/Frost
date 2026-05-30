mod helpers;

use frost_parse::ast::*;
use frost_parse::parse_program;
use helpers::*;

// -- Basic arithmetic --

#[test]
fn addition() {
    let expr = parse_expr("1 + 2");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Add));
    assert!(is_int(left, 1));
    assert!(is_int(right, 2));
}

#[test]
fn subtraction() {
    let expr = parse_expr("5 - 3");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Sub));
    assert!(is_int(left, 5));
    assert!(is_int(right, 3));
}

#[test]
fn multiplication() {
    let expr = parse_expr("4 * 7");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Mul));
    assert!(is_int(left, 4));
    assert!(is_int(right, 7));
}

#[test]
fn division() {
    let expr = parse_expr("10 / 2");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Div));
    assert!(is_int(left, 10));
    assert!(is_int(right, 2));
}

#[test]
fn modulo() {
    let expr = parse_expr("7 % 3");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Mod));
    assert!(is_int(left, 7));
    assert!(is_int(right, 3));
}

// -- Precedence --

#[test]
fn mul_binds_tighter_than_add() {
    let expr = parse_expr("1 + 2 * 3");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Add));
    assert!(is_int(left, 1));
    let (rl, rop, rr) = is_binop(right).unwrap();
    assert!(matches!(rop, BinOp::Mul));
    assert!(is_int(rl, 2));
    assert!(is_int(rr, 3));
}

#[test]
fn add_is_left_associative() {
    let expr = parse_expr("1 - 2 - 3");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Sub));
    assert!(is_int(right, 3));
    let (ll, lop, lr) = is_binop(left).unwrap();
    assert!(matches!(lop, BinOp::Sub));
    assert!(is_int(ll, 1));
    assert!(is_int(lr, 2));
}

#[test]
fn mul_is_left_associative() {
    let expr = parse_expr("12 / 6 / 2");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Div));
    assert!(is_int(right, 2));
    let (ll, lop, lr) = is_binop(left).unwrap();
    assert!(matches!(lop, BinOp::Div));
    assert!(is_int(ll, 12));
    assert!(is_int(lr, 6));
}

#[test]
fn and_or_precedence() {
    let expr = parse_expr("true and false or true");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Or));
    assert!(matches!(
        &right.kind,
        ExprKind::Literal(Literal::Bool(true))
    ));
    let (ll, lop, lr) = is_binop(left).unwrap();
    assert!(matches!(lop, BinOp::And));
    assert!(matches!(&ll.kind, ExprKind::Literal(Literal::Bool(true))));
    assert!(matches!(&lr.kind, ExprKind::Literal(Literal::Bool(false))));
}

#[test]
fn comparison_lower_than_arithmetic() {
    let expr = parse_expr("1 + 2 == 3");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Eq));
    assert!(is_int(right, 3));
    let (ll, lop, lr) = is_binop(left).unwrap();
    assert!(matches!(lop, BinOp::Add));
    assert!(is_int(ll, 1));
    assert!(is_int(lr, 2));
}

#[test]
fn and_is_left_associative() {
    let expr = parse_expr("a and b and c");
    let (left, op, _) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::And));
    let (_, lop, _) = is_binop(left).unwrap();
    assert!(matches!(lop, BinOp::And));
}

#[test]
fn or_is_left_associative() {
    let expr = parse_expr("a or b or c");
    let (left, op, _) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Or));
    let (_, lop, _) = is_binop(left).unwrap();
    assert!(matches!(lop, BinOp::Or));
}

#[test]
fn mixed_precedence_left_assoc() {
    // 1 + 2 * 3 + 4 == (1 + (2 * 3)) + 4
    let expr = parse_expr("1 + 2 * 3 + 4");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Add));
    assert!(is_int(right, 4));
    let (ll, lop, lr) = is_binop(left).unwrap();
    assert!(matches!(lop, BinOp::Add));
    assert!(is_int(ll, 1));
    let (rl, rop, rr) = is_binop(lr).unwrap();
    assert!(matches!(rop, BinOp::Mul));
    assert!(is_int(rl, 2));
    assert!(is_int(rr, 3));
}

#[test]
fn and_with_comparison() {
    let expr = parse_expr("x > 0 and x < 100");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::And));
    let (_, lop, _) = is_binop(left).unwrap();
    assert!(matches!(lop, BinOp::Gt));
    let (_, rop, _) = is_binop(right).unwrap();
    assert!(matches!(rop, BinOp::Lt));
}

// -- Prefix operators --

#[test]
fn unary_negate() {
    let expr = parse_expr("-5");
    match &expr.kind {
        ExprKind::UnaryOp { op, operand } => {
            assert!(matches!(op, UnaryOp::Negate));
            assert!(is_int(operand, 5));
        }
        other => panic!("expected UnaryOp, got {other:?}"),
    }
}

#[test]
fn unary_not() {
    let expr = parse_expr("not true");
    match &expr.kind {
        ExprKind::UnaryOp { op, operand } => {
            assert!(matches!(op, UnaryOp::Not));
            assert!(matches!(
                &operand.kind,
                ExprKind::Literal(Literal::Bool(true))
            ));
        }
        other => panic!("expected UnaryOp, got {other:?}"),
    }
}

#[test]
fn negate_binds_tighter_than_add() {
    let expr = parse_expr("-2 + 3");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Add));
    assert!(is_int(right, 3));
    match &left.kind {
        ExprKind::UnaryOp { op, operand } => {
            assert!(matches!(op, UnaryOp::Negate));
            assert!(is_int(operand, 2));
        }
        other => panic!("expected UnaryOp, got {other:?}"),
    }
}

#[test]
fn not_precedence() {
    let expr = parse_expr("not true or false");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Or));
    assert!(matches!(
        &right.kind,
        ExprKind::Literal(Literal::Bool(false))
    ));
    match &left.kind {
        ExprKind::UnaryOp { op, operand } => {
            assert!(matches!(op, UnaryOp::Not));
            assert!(matches!(
                &operand.kind,
                ExprKind::Literal(Literal::Bool(true))
            ));
        }
        other => panic!("expected UnaryOp, got {other:?}"),
    }
}

#[test]
fn double_negate() {
    let expr = parse_expr("--1");
    match &expr.kind {
        ExprKind::UnaryOp {
            op: UnaryOp::Negate,
            operand,
        } => match &operand.kind {
            ExprKind::UnaryOp {
                op: UnaryOp::Negate,
                operand: inner,
            } => {
                assert!(is_int(inner, 1));
            }
            other => panic!("expected inner Negate, got {other:?}"),
        },
        other => panic!("expected Negate, got {other:?}"),
    }
}

#[test]
fn double_not() {
    let expr = parse_expr("not not true");
    match &expr.kind {
        ExprKind::UnaryOp {
            op: UnaryOp::Not,
            operand,
        } => match &operand.kind {
            ExprKind::UnaryOp {
                op: UnaryOp::Not,
                operand: inner,
            } => {
                assert!(matches!(
                    &inner.kind,
                    ExprKind::Literal(Literal::Bool(true))
                ));
            }
            other => panic!("expected inner Not, got {other:?}"),
        },
        other => panic!("expected Not, got {other:?}"),
    }
}

#[test]
fn negate_float() {
    let expr = parse_expr("-3.14");
    match &expr.kind {
        ExprKind::UnaryOp {
            op: UnaryOp::Negate,
            operand,
        } => {
            assert!(matches!(&operand.kind, ExprKind::Literal(Literal::Float(f)) if *f == 3.14));
        }
        other => panic!("expected Negate, got {other:?}"),
    }
}

#[test]
fn not_binds_tighter_than_equality() {
    // `not a == b` parses as `(not a) == b`, not `not (a == b)`.
    let expr = parse_expr("not a == b");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Eq));
    assert!(matches!(&right.kind, ExprKind::NameLookup(n) if n == "b"));
    match &left.kind {
        ExprKind::UnaryOp {
            op: UnaryOp::Not,
            operand,
        } => assert!(matches!(&operand.kind, ExprKind::NameLookup(n) if n == "a")),
        other => panic!("expected Not(a), got {other:?}"),
    }
}

#[test]
fn subtract_negative_literal() {
    // `3 - -2` is binary minus with a negated-literal RHS, not `3 -- 2`.
    let expr = parse_expr("3 - -2");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Sub));
    assert!(is_int(left, 3));
    match &right.kind {
        ExprKind::UnaryOp {
            op: UnaryOp::Negate,
            operand,
        } => assert!(is_int(operand, 2)),
        other => panic!("expected Negate(2), got {other:?}"),
    }
}

#[test]
fn negate_call() {
    // `-f()` is `-(f())`: the call binds tighter than the prefix negate.
    let expr = parse_expr("-f()");
    match &expr.kind {
        ExprKind::UnaryOp {
            op: UnaryOp::Negate,
            operand,
        } => assert!(matches!(&operand.kind, ExprKind::Call { .. })),
        other => panic!("expected Negate(Call), got {other:?}"),
    }
}

// -- Comparisons --

#[test]
fn less_than() {
    let expr = parse_expr("1 < 2");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Lt));
    assert!(is_int(left, 1));
    assert!(is_int(right, 2));
}

#[test]
fn greater_equal() {
    let expr = parse_expr("5 >= 3");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Gte));
    assert!(is_int(left, 5));
    assert!(is_int(right, 3));
}

#[test]
fn not_equal() {
    let expr = parse_expr("1 != 2");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Neq));
    assert!(is_int(left, 1));
    assert!(is_int(right, 2));
}

// -- Parenthesized grouping --

#[test]
fn parens_override_precedence() {
    let expr = parse_expr("(1 + 2) * 3");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Mul));
    assert!(is_int(right, 3));
    let (ll, lop, lr) = is_binop(left).unwrap();
    assert!(matches!(lop, BinOp::Add));
    assert!(is_int(ll, 1));
    assert!(is_int(lr, 2));
}

#[test]
fn nested_parens() {
    let expr = parse_expr("((1 + 2))");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Add));
    assert!(is_int(left, 1));
    assert!(is_int(right, 2));
}

#[test]
fn parens_with_newlines() {
    let expr = parse_expr("(\n1\n+\n2\n)");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Add));
    assert!(is_int(left, 1));
    assert!(is_int(right, 2));
}

// -- Newline sensitivity --

#[test]
fn newline_terminates_expression() {
    let program = parse_program("test.frst", "1 + 2\n3").expect("failed to parse");
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn newline_after_prefix_is_error() {
    let err = parse_err("not\ntrue");
    assert!(
        err.contains("unexpected") || err.contains("end of input"),
        "error was: {err}"
    );
}

#[test]
fn negate_newline_is_error() {
    let err = parse_err("-\n5");
    assert!(
        err.contains("unexpected") || err.contains("end of input"),
        "error was: {err}"
    );
}

#[test]
fn prefix_with_newline_in_parens() {
    let expr = parse_expr("(\nnot\ntrue\n)");
    match &expr.kind {
        ExprKind::UnaryOp {
            op: UnaryOp::Not,
            operand,
        } => {
            assert!(matches!(
                &operand.kind,
                ExprKind::Literal(Literal::Bool(true))
            ));
        }
        other => panic!("expected Not, got {other:?}"),
    }
}

#[test]
fn negate_with_newline_in_parens() {
    let expr = parse_expr("(-\n5)");
    match &expr.kind {
        ExprKind::UnaryOp {
            op: UnaryOp::Negate,
            operand,
        } => {
            assert!(is_int(operand, 5));
        }
        other => panic!("expected Negate, got {other:?}"),
    }
}

#[test]
fn infix_with_newline_in_parens() {
    let expr = parse_expr("(1 +\n2)");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Add));
    assert!(is_int(left, 1));
    assert!(is_int(right, 2));
}

// -- Non-chainable errors --

#[test]
fn error_chained_comparison() {
    let err = parse_err("1 < 2 < 3");
    assert!(err.contains("cannot chain"), "error was: {err}");
}

#[test]
fn error_chained_equality() {
    let err = parse_err("a == b == c");
    assert!(err.contains("cannot chain"), "error was: {err}");
}

#[test]
fn error_mixed_chain() {
    let err = parse_err("1 < 2 == 3");
    assert!(err.contains("cannot chain"), "error was: {err}");
}

// Relational and equality operators are mutually non-chainable: they cannot
// chain with each other OR mix in either direction. Parentheses are required.

#[test]
fn error_equality_then_comparison() {
    let err = parse_err("a == b < c");
    assert!(err.contains("cannot chain"), "error was: {err}");
}

#[test]
fn error_inequality_then_comparison() {
    let err = parse_err("a != b > c");
    assert!(err.contains("cannot chain"), "error was: {err}");
}

#[test]
fn error_comparison_then_inequality() {
    let err = parse_err("a > b != c");
    assert!(err.contains("cannot chain"), "error was: {err}");
}

#[test]
fn error_le_then_equality() {
    let err = parse_err("a <= b == c");
    assert!(err.contains("cannot chain"), "error was: {err}");
}

// Explicit parentheses are the escape hatch: a grouped relational result may be
// compared for equality. The chaining ban must NOT reject these -- a parsed
// Paren/group resets the "previous operator tier" tracking.
#[test]
fn parens_allow_relational_then_equality() {
    let expr = parse_expr("(a < b) == c");
    let (left, op, _right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Eq));
    assert!(matches!(is_binop(left), Some((_, BinOp::Lt, _))));
}

#[test]
fn parens_allow_equality_then_relational() {
    let expr = parse_expr("(a == b) < c");
    let (left, op, _right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Lt));
    assert!(matches!(is_binop(left), Some((_, BinOp::Eq, _))));
}

// -- General error cases --

#[test]
fn error_trailing_operator() {
    let err = parse_err("1 +");
    assert!(
        err.contains("end of input") || err.contains("unexpected"),
        "error was: {err}"
    );
}

#[test]
fn error_leading_infix_operator() {
    let err = parse_err("* 2");
    assert!(err.contains("unexpected"), "error was: {err}");
}

#[test]
fn error_unclosed_paren() {
    let err = parse_err("(1 + 2");
    assert!(
        err.contains("end of input") || err.contains("Expected )"),
        "error was: {err}"
    );
}

#[test]
fn error_mismatched_paren() {
    let err = parse_err("(1 + 2]");
    assert!(
        err.contains("Expected )") || err.contains("unexpected"),
        "error was: {err}"
    );
}
