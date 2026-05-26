use frost_parse::ast::*;
use frost_parse::parse_program;

fn parse_expr(src: &str) -> Expr {
    let program = parse_program("test.frst", src).expect(&format!("failed to parse: {src}"));
    assert_eq!(program.statements.len(), 1);
    match program.statements.into_iter().next().unwrap().kind {
        StatementKind::Expr(expr) => expr,
        other => panic!("expected Expr statement, got {other:?}"),
    }
}

fn parse_err(src: &str) -> String {
    parse_program("test.frst", src)
        .expect_err(&format!("expected parse error for: {src}"))
        .to_string()
}

fn is_int(expr: &Expr, n: i64) -> bool {
    matches!(&expr.kind, ExprKind::Literal(Literal::Int(v)) if *v == n)
}

fn is_binop(expr: &Expr) -> Option<(&Expr, BinOp, &Expr)> {
    match &expr.kind {
        ExprKind::BinOp { left, op, right } => Some((left, *op, right)),
        _ => None,
    }
}

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
    // 1 + 2 * 3 == 1 + (2 * 3)
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
    // 1 - 2 - 3 == (1 - 2) - 3
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
    // 12 / 6 / 2 == (12 / 6) / 2
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
    // true and false or true == (true and false) or true
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
    // 1 + 2 == 3 parses as (1 + 2) == 3
    let expr = parse_expr("1 + 2 == 3");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Eq));
    assert!(is_int(right, 3));
    let (ll, lop, lr) = is_binop(left).unwrap();
    assert!(matches!(lop, BinOp::Add));
    assert!(is_int(ll, 1));
    assert!(is_int(lr, 2));
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
    // -2 + 3 == (-2) + 3
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
    // not true or false == (not true) or false
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
    // --1 is a double negation (not decrement)
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

// -- Parenthesized grouping --

#[test]
fn parens_override_precedence() {
    // (1 + 2) * 3
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

// -- Logical operators with arithmetic --

#[test]
fn and_with_comparison() {
    // x > 0 and x < 100
    let expr = parse_expr("x > 0 and x < 100");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::And));
    let (_, lop, _) = is_binop(left).unwrap();
    assert!(matches!(lop, BinOp::Gt));
    let (_, rop, _) = is_binop(right).unwrap();
    assert!(matches!(rop, BinOp::Lt));
}

#[test]
fn and_is_left_associative() {
    // a and b and c == (a and b) and c
    let expr = parse_expr("a and b and c");
    let (left, op, _) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::And));
    let (_, lop, _) = is_binop(left).unwrap();
    assert!(matches!(lop, BinOp::And));
}

#[test]
fn or_is_left_associative() {
    // a or b or c == (a or b) or c
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

// -- More prefix --

#[test]
fn double_not() {
    // not not true == not (not true)
    let expr = parse_expr("not not true");
    match &expr.kind {
        ExprKind::UnaryOp { op: UnaryOp::Not, operand } => {
            match &operand.kind {
                ExprKind::UnaryOp { op: UnaryOp::Not, operand: inner } => {
                    assert!(matches!(&inner.kind, ExprKind::Literal(Literal::Bool(true))));
                }
                other => panic!("expected inner Not, got {other:?}"),
            }
        }
        other => panic!("expected Not, got {other:?}"),
    }
}

#[test]
fn negate_float() {
    let expr = parse_expr("-3.14");
    match &expr.kind {
        ExprKind::UnaryOp { op: UnaryOp::Negate, operand } => {
            assert!(matches!(&operand.kind, ExprKind::Literal(Literal::Float(f)) if *f == 3.14));
        }
        other => panic!("expected Negate, got {other:?}"),
    }
}

// -- Float literals --

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

// -- Newline sensitivity --

#[test]
fn newline_terminates_expression() {
    // 1 + 2\n3 should be two statements
    let program = parse_program("test.frst", "1 + 2\n3")
        .expect("failed to parse");
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn newline_after_prefix_is_error() {
    // `not\ntrue` at top level: `not` alone is not a valid expression
    let err = parse_err("not\ntrue");
    assert!(err.contains("unexpected") || err.contains("end of input"), "error was: {err}");
}

#[test]
fn negate_newline_is_error() {
    // `-\n5` at top level: `-` alone is not a valid expression
    let err = parse_err("-\n5");
    assert!(err.contains("unexpected") || err.contains("end of input"), "error was: {err}");
}

#[test]
fn prefix_with_newline_in_parens() {
    // (\nnot\ntrue\n) is valid — newlines are insignificant inside parens
    let expr = parse_expr("(\nnot\ntrue\n)");
    match &expr.kind {
        ExprKind::UnaryOp { op: UnaryOp::Not, operand } => {
            assert!(matches!(&operand.kind, ExprKind::Literal(Literal::Bool(true))));
        }
        other => panic!("expected Not, got {other:?}"),
    }
}

#[test]
fn negate_with_newline_in_parens() {
    let expr = parse_expr("(-\n5)");
    match &expr.kind {
        ExprKind::UnaryOp { op: UnaryOp::Negate, operand } => {
            assert!(is_int(operand, 5));
        }
        other => panic!("expected Negate, got {other:?}"),
    }
}

#[test]
fn infix_with_newline_in_parens() {
    // (1 +\n2) is valid inside parens
    let expr = parse_expr("(1 +\n2)");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Add));
    assert!(is_int(left, 1));
    assert!(is_int(right, 2));
}

// -- Single atom (no operator) --

#[test]
fn bare_identifier_expression() {
    let expr = parse_expr("x");
    assert!(matches!(&expr.kind, ExprKind::NameLookup(n) if n == "x"));
}

// -- Error cases --

#[test]
fn empty_input_is_valid() {
    let program = parse_program("test.frst", "").expect("failed to parse");
    assert!(program.statements.is_empty());
}

#[test]
fn error_trailing_operator() {
    let err = parse_err("1 +");
    assert!(err.contains("end of input") || err.contains("unexpected"), "error was: {err}");
}

#[test]
fn error_leading_infix_operator() {
    let err = parse_err("* 2");
    assert!(err.contains("unexpected"), "error was: {err}");
}

#[test]
fn error_unclosed_paren() {
    let err = parse_err("(1 + 2");
    assert!(err.contains("end of input") || err.contains("Expected )"), "error was: {err}");
}

#[test]
fn error_mismatched_paren() {
    let err = parse_err("(1 + 2]");
    assert!(err.contains("Expected )") || err.contains("unexpected"), "error was: {err}");
}

// -- Function calls --

#[test]
fn call_no_args() {
    let expr = parse_expr("f()");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert!(matches!(&callee.kind, ExprKind::NameLookup(n) if n == "f"));
            assert!(args.is_empty());
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_one_arg() {
    let expr = parse_expr("f(1)");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert!(matches!(&callee.kind, ExprKind::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 1);
            assert!(is_int(&args[0], 1));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_multiple_args() {
    let expr = parse_expr("f(1, 2, 3)");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert!(matches!(&callee.kind, ExprKind::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 3);
            assert!(is_int(&args[0], 1));
            assert!(is_int(&args[1], 2));
            assert!(is_int(&args[2], 3));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_trailing_comma() {
    let expr = parse_expr("f(1, 2,)");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert!(matches!(&callee.kind, ExprKind::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 2);
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_with_expression_args() {
    let expr = parse_expr("f(1 + 2, 3 * 4)");
    match &expr.kind {
        ExprKind::Call { args, .. } => {
            assert_eq!(args.len(), 2);
            assert!(is_binop(&args[0]).is_some());
            assert!(is_binop(&args[1]).is_some());
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_with_newlines() {
    let expr = parse_expr("f(\n1,\n2\n)");
    match &expr.kind {
        ExprKind::Call { args, .. } => {
            assert_eq!(args.len(), 2);
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn chained_calls() {
    // f(1)(2) == Call(Call(f, [1]), [2])
    let expr = parse_expr("f(1)(2)");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert_eq!(args.len(), 1);
            assert!(is_int(&args[0], 2));
            match &callee.kind {
                ExprKind::Call { callee: inner, args: inner_args } => {
                    assert!(matches!(&inner.kind, ExprKind::NameLookup(n) if n == "f"));
                    assert_eq!(inner_args.len(), 1);
                    assert!(is_int(&inner_args[0], 1));
                }
                other => panic!("expected inner Call, got {other:?}"),
            }
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_newline_not_call() {
    // f\n(1) is two statements, not a call
    let program = parse_program("test.frst", "f\n(1)")
        .expect("failed to parse");
    assert_eq!(program.statements.len(), 2);
}

// -- Indexing --

#[test]
fn index_literal() {
    let expr = parse_expr("a[0]");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(matches!(&target.kind, ExprKind::NameLookup(n) if n == "a"));
            assert!(is_int(key, 0));
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn index_expression() {
    let expr = parse_expr("a[1 + 2]");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(matches!(&target.kind, ExprKind::NameLookup(n) if n == "a"));
            assert!(is_binop(key).is_some());
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn chained_index() {
    // a[0][1] == Index(Index(a, 0), 1)
    let expr = parse_expr("a[0][1]");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(is_int(key, 1));
            match &target.kind {
                ExprKind::Index { target: inner, key: inner_key } => {
                    assert!(matches!(&inner.kind, ExprKind::NameLookup(n) if n == "a"));
                    assert!(is_int(inner_key, 0));
                }
                other => panic!("expected inner Index, got {other:?}"),
            }
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn index_with_newlines() {
    let expr = parse_expr("a[\n0\n]");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(matches!(&target.kind, ExprKind::NameLookup(n) if n == "a"));
            assert!(is_int(key, 0));
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

// -- Dot access --

#[test]
fn dot_access() {
    let expr = parse_expr("a.foo");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(matches!(&target.kind, ExprKind::NameLookup(n) if n == "a"));
            assert!(matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"foo"));
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn chained_dot() {
    // a.b.c == Index(Index(a, "b"), "c")
    let expr = parse_expr("a.b.c");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"c"));
            match &target.kind {
                ExprKind::Index { target: inner, key: inner_key } => {
                    assert!(matches!(&inner.kind, ExprKind::NameLookup(n) if n == "a"));
                    assert!(matches!(&inner_key.kind, ExprKind::Literal(Literal::String(s)) if s == b"b"));
                }
                other => panic!("expected inner Index, got {other:?}"),
            }
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn dot_then_call() {
    // a.foo(1) == Call(Index(a, "foo"), [1])
    let expr = parse_expr("a.foo(1)");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert_eq!(args.len(), 1);
            assert!(is_int(&args[0], 1));
            match &callee.kind {
                ExprKind::Index { target, key } => {
                    assert!(matches!(&target.kind, ExprKind::NameLookup(n) if n == "a"));
                    assert!(matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"foo"));
                }
                other => panic!("expected Index inside Call, got {other:?}"),
            }
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

// -- Threading --

#[test]
fn thread_no_extra_args() {
    // a @ f() == f(a)
    let expr = parse_expr("a @ f()");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert!(matches!(&callee.kind, ExprKind::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 1);
            assert!(matches!(&args[0].kind, ExprKind::NameLookup(n) if n == "a"));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn thread_with_args() {
    // a @ f(1) == f(a, 1)
    let expr = parse_expr("a @ f(1)");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert!(matches!(&callee.kind, ExprKind::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0].kind, ExprKind::NameLookup(n) if n == "a"));
            assert!(is_int(&args[1], 1));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn thread_chained() {
    // a @ f(1) @ g(2) == g(f(a, 1), 2)
    let expr = parse_expr("a @ f(1) @ g(2)");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert!(matches!(&callee.kind, ExprKind::NameLookup(n) if n == "g"));
            assert_eq!(args.len(), 2);
            assert!(is_int(&args[1], 2));
            match &args[0].kind {
                ExprKind::Call { callee: inner_callee, args: inner_args } => {
                    assert!(matches!(&inner_callee.kind, ExprKind::NameLookup(n) if n == "f"));
                    assert_eq!(inner_args.len(), 2);
                    assert!(matches!(&inner_args[0].kind, ExprKind::NameLookup(n) if n == "a"));
                    assert!(is_int(&inner_args[1], 1));
                }
                other => panic!("expected inner Call, got {other:?}"),
            }
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn thread_dot_callee() {
    // a @ m.f() == m.f(a) == Index(m, "f") called with [a]
    let expr = parse_expr("a @ m.f()");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert_eq!(args.len(), 1);
            assert!(matches!(&args[0].kind, ExprKind::NameLookup(n) if n == "a"));
            match &callee.kind {
                ExprKind::Index { target, key } => {
                    assert!(matches!(&target.kind, ExprKind::NameLookup(n) if n == "m"));
                    assert!(matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"f"));
                }
                other => panic!("expected Index callee, got {other:?}"),
            }
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

// -- Postfix binds tighter than prefix --

#[test]
fn negate_index() {
    // -a[0] == -(a[0])
    let expr = parse_expr("-a[0]");
    match &expr.kind {
        ExprKind::UnaryOp { op: UnaryOp::Negate, operand } => {
            assert!(matches!(&operand.kind, ExprKind::Index { .. }));
        }
        other => panic!("expected Negate(Index), got {other:?}"),
    }
}

// -- Postfix error cases --

#[test]
fn error_dot_no_identifier() {
    let err = parse_err("a.42");
    assert!(err.contains("unexpected") || err.contains("expected identifier"), "error was: {err}");
}

#[test]
fn error_unclosed_index() {
    let err = parse_err("a[0");
    assert!(err.contains("end of input") || err.contains("Expected ]"), "error was: {err}");
}

#[test]
fn error_unclosed_call() {
    let err = parse_err("f(1, 2");
    assert!(err.contains("end of input") || err.contains("Expected )"), "error was: {err}");
}

#[test]
fn error_thread_no_parens() {
    let err = parse_err("a @ f");
    assert!(err.contains("Expected (") || err.contains("unexpected"), "error was: {err}");
}

// -- Mixed postfix chains --

#[test]
fn call_then_index() {
    // f()[0] == Index(Call(f, []), 0)
    let expr = parse_expr("f()[0]");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(is_int(key, 0));
            match &target.kind {
                ExprKind::Call { callee, args } => {
                    assert!(matches!(&callee.kind, ExprKind::NameLookup(n) if n == "f"));
                    assert!(args.is_empty());
                }
                other => panic!("expected Call, got {other:?}"),
            }
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn index_then_call() {
    // a[0](1) == Call(Index(a, 0), [1])
    let expr = parse_expr("a[0](1)");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert_eq!(args.len(), 1);
            assert!(is_int(&args[0], 1));
            match &callee.kind {
                ExprKind::Index { target, key } => {
                    assert!(matches!(&target.kind, ExprKind::NameLookup(n) if n == "a"));
                    assert!(is_int(key, 0));
                }
                other => panic!("expected Index, got {other:?}"),
            }
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_then_dot() {
    // f().bar == Index(Call(f, []), "bar")
    let expr = parse_expr("f().bar");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"bar"));
            match &target.kind {
                ExprKind::Call { callee, args } => {
                    assert!(matches!(&callee.kind, ExprKind::NameLookup(n) if n == "f"));
                    assert!(args.is_empty());
                }
                other => panic!("expected Call, got {other:?}"),
            }
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn dot_then_index() {
    // a.b[0] == Index(Index(a, "b"), 0)
    let expr = parse_expr("a.b[0]");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(is_int(key, 0));
            match &target.kind {
                ExprKind::Index { target: inner, key: inner_key } => {
                    assert!(matches!(&inner.kind, ExprKind::NameLookup(n) if n == "a"));
                    assert!(matches!(&inner_key.kind, ExprKind::Literal(Literal::String(s)) if s == b"b"));
                }
                other => panic!("expected inner Index, got {other:?}"),
            }
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn index_then_dot() {
    // a[0].bar == Index(Index(a, 0), "bar")
    let expr = parse_expr("a[0].bar");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"bar"));
            match &target.kind {
                ExprKind::Index { target: inner, key: inner_key } => {
                    assert!(matches!(&inner.kind, ExprKind::NameLookup(n) if n == "a"));
                    assert!(is_int(inner_key, 0));
                }
                other => panic!("expected inner Index, got {other:?}"),
            }
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn long_postfix_chain() {
    // a.b[0].c(1).d == Index(Call(Index(Index(Index(a, "b"), 0), "c"), [1]), "d")
    let expr = parse_expr("a.b[0].c(1).d");
    // outermost is .d
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"d"));
            // next is (1)
            match &target.kind {
                ExprKind::Call { callee, args } => {
                    assert_eq!(args.len(), 1);
                    assert!(is_int(&args[0], 1));
                    // next is .c
                    match &callee.kind {
                        ExprKind::Index { target, key } => {
                            assert!(matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"c"));
                            // next is [0]
                            match &target.kind {
                                ExprKind::Index { target, key } => {
                                    assert!(is_int(key, 0));
                                    // next is .b
                                    match &target.kind {
                                        ExprKind::Index { target, key } => {
                                            assert!(matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"b"));
                                            assert!(matches!(&target.kind, ExprKind::NameLookup(n) if n == "a"));
                                        }
                                        other => panic!("expected .b Index, got {other:?}"),
                                    }
                                }
                                other => panic!("expected [0] Index, got {other:?}"),
                            }
                        }
                        other => panic!("expected .c Index, got {other:?}"),
                    }
                }
                other => panic!("expected Call, got {other:?}"),
            }
        }
        other => panic!("expected .d Index, got {other:?}"),
    }
}

// -- Postfix in binary operands --

#[test]
fn calls_in_arithmetic() {
    // f(1) + g(2)
    let expr = parse_expr("f(1) + g(2)");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Add));
    assert!(matches!(&left.kind, ExprKind::Call { .. }));
    assert!(matches!(&right.kind, ExprKind::Call { .. }));
}

#[test]
fn index_in_arithmetic() {
    // a[0] * b[1]
    let expr = parse_expr("a[0] * b[1]");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Mul));
    assert!(matches!(&left.kind, ExprKind::Index { .. }));
    assert!(matches!(&right.kind, ExprKind::Index { .. }));
}

#[test]
fn dot_in_comparison() {
    // a.x == b.y
    let expr = parse_expr("a.x == b.y");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Eq));
    assert!(matches!(&left.kind, ExprKind::Index { .. }));
    assert!(matches!(&right.kind, ExprKind::Index { .. }));
}

// -- Threading extras --

#[test]
fn thread_multiple_extra_args() {
    // a @ f(1, 2) == f(a, 1, 2)
    let expr = parse_expr("a @ f(1, 2)");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert!(matches!(&callee.kind, ExprKind::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 3);
            assert!(matches!(&args[0].kind, ExprKind::NameLookup(n) if n == "a"));
            assert!(is_int(&args[1], 1));
            assert!(is_int(&args[2], 2));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn thread_after_postfix() {
    // a.b @ f() == f(a.b) == f(Index(a, "b"))
    let expr = parse_expr("a.b @ f()");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert!(matches!(&callee.kind, ExprKind::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 1);
            assert!(matches!(&args[0].kind, ExprKind::Index { .. }));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn thread_result_indexed() {
    // a @ f()[0] — thread result then index
    let expr = parse_expr("a @ f()[0]");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(is_int(key, 0));
            match &target.kind {
                ExprKind::Call { callee, args } => {
                    assert!(matches!(&callee.kind, ExprKind::NameLookup(n) if n == "f"));
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0].kind, ExprKind::NameLookup(n) if n == "a"));
                }
                other => panic!("expected Call, got {other:?}"),
            }
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

// -- Newline sensitivity with postfix --

#[test]
#[should_panic(expected = "Array literal")]
fn newline_before_bracket_is_two_statements() {
    // a\n[0] is two statements: `a` and `[0]` (array literal)
    // Hits todo!() because array literal parsing isn't implemented yet.
    // Remove should_panic once array literals are in place.
    let program = parse_program("test.frst", "a\n[0]")
        .expect("failed to parse");
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn newline_before_dot_is_error() {
    // a\n.foo — `.foo` is not a valid expression start
    let err = parse_err("a\n.foo");
    assert!(err.contains("unexpected"), "error was: {err}");
}

#[test]
fn newline_before_call_is_two_statements() {
    // f\n(1) is two statements: `f` and `(1)` (parenthesized 1)
    let program = parse_program("test.frst", "f\n(1)")
        .expect("failed to parse");
    assert_eq!(program.statements.len(), 2);
}
