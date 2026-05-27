mod helpers;

use frost_parse::ast::*;
use helpers::*;

fn array_elements(expr: &Expr) -> &[Expr] {
    match &expr.kind {
        ExprKind::Array(elems) => elems,
        other => panic!("expected Array, got {other:?}"),
    }
}

// -- Array literals --

#[test]
fn empty_array() {
    let expr = parse_expr("[]");
    assert!(array_elements(&expr).is_empty());
}

#[test]
fn single_element() {
    let expr = parse_expr("[1]");
    let elems = array_elements(&expr);
    assert_eq!(elems.len(), 1);
    assert!(is_int(&elems[0], 1));
}

#[test]
fn multiple_elements() {
    let expr = parse_expr("[1, 2, 3]");
    let elems = array_elements(&expr);
    assert_eq!(elems.len(), 3);
    assert!(is_int(&elems[0], 1));
    assert!(is_int(&elems[1], 2));
    assert!(is_int(&elems[2], 3));
}

#[test]
fn trailing_comma() {
    let expr = parse_expr("[1, 2,]");
    let elems = array_elements(&expr);
    assert_eq!(elems.len(), 2);
}

#[test]
fn expression_elements() {
    let expr = parse_expr("[1 + 2, 3 * 4]");
    let elems = array_elements(&expr);
    assert_eq!(elems.len(), 2);
    assert!(is_binop(&elems[0]).is_some());
    assert!(is_binop(&elems[1]).is_some());
}

#[test]
fn nested_array() {
    let expr = parse_expr("[[1, 2], [3, 4]]");
    let elems = array_elements(&expr);
    assert_eq!(elems.len(), 2);
    assert_eq!(array_elements(&elems[0]).len(), 2);
    assert_eq!(array_elements(&elems[1]).len(), 2);
}

#[test]
fn deeply_nested_array() {
    let expr = parse_expr("[[[1]]]");
    let outer = array_elements(&expr);
    assert_eq!(outer.len(), 1);
    let mid = array_elements(&outer[0]);
    assert_eq!(mid.len(), 1);
    let inner = array_elements(&mid[0]);
    assert_eq!(inner.len(), 1);
    assert!(is_int(&inner[0], 1));
}

// -- Newlines inside arrays --

#[test]
fn newlines_between_elements() {
    let expr = parse_expr("[\n1,\n2,\n3\n]");
    let elems = array_elements(&expr);
    assert_eq!(elems.len(), 3);
}

#[test]
fn newlines_after_trailing_comma() {
    let expr = parse_expr("[\n1,\n2,\n]");
    let elems = array_elements(&expr);
    assert_eq!(elems.len(), 2);
}

#[test]
fn newlines_in_empty_array() {
    let expr = parse_expr("[\n]");
    assert!(array_elements(&expr).is_empty());
}

// -- Mixed content --

#[test]
fn heterogeneous_elements() {
    let expr = parse_expr("[1, true, null, foo]");
    let elems = array_elements(&expr);
    assert_eq!(elems.len(), 4);
    assert!(is_int(&elems[0], 1));
    assert!(matches!(&elems[1].kind, ExprKind::Literal(Literal::Bool(true))));
    assert!(matches!(&elems[2].kind, ExprKind::Literal(Literal::Null)));
    assert!(matches!(&elems[3].kind, ExprKind::NameLookup(n) if n == "foo"));
}

#[test]
fn string_elements() {
    let expr = parse_expr("['foo', 'bar']");
    let elems = array_elements(&expr);
    assert_eq!(elems.len(), 2);
}

#[test]
fn call_in_array() {
    let expr = parse_expr("[f(1), g(2)]");
    let elems = array_elements(&expr);
    assert_eq!(elems.len(), 2);
    assert!(matches!(&elems[0].kind, ExprKind::Call { .. }));
    assert!(matches!(&elems[1].kind, ExprKind::Call { .. }));
}

// -- Arrays in expressions --

#[test]
fn array_in_def() {
    let program = parse("def xs = [1, 2, 3]");
    assert_eq!(program.statements.len(), 1);
    match &program.statements[0].kind {
        StatementKind::Def { expr, .. } => {
            assert_eq!(array_elements(expr).len(), 3);
        }
        other => panic!("expected Def, got {other:?}"),
    }
}

#[test]
fn array_indexing() {
    let expr = parse_expr("[1, 2, 3][0]");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert_eq!(array_elements(target).len(), 3);
            assert!(is_int(key, 0));
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn array_concatenation() {
    let expr = parse_expr("[1, 2] + [3, 4]");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Add));
    assert_eq!(array_elements(left).len(), 2);
    assert_eq!(array_elements(right).len(), 2);
}

// -- Multiline arrays (stress tests) --

#[test]
fn multiline_simple() {
    let expr = parse_expr("[\n  1,\n  2,\n  3\n]");
    assert_eq!(array_elements(&expr).len(), 3);
}

#[test]
fn multiline_trailing_comma_then_newline() {
    let expr = parse_expr("[\n  1,\n  2,\n]");
    assert_eq!(array_elements(&expr).len(), 2);
}

#[test]
fn multiline_nested() {
    let expr = parse_expr("[\n  [1, 2],\n  [3, 4],\n]");
    let elems = array_elements(&expr);
    assert_eq!(elems.len(), 2);
    assert_eq!(array_elements(&elems[0]).len(), 2);
    assert_eq!(array_elements(&elems[1]).len(), 2);
}

#[test]
fn multiline_with_expressions() {
    let expr = parse_expr("[\n  1 + 2,\n  3 * 4,\n]");
    let elems = array_elements(&expr);
    assert_eq!(elems.len(), 2);
    assert!(is_binop(&elems[0]).is_some());
    assert!(is_binop(&elems[1]).is_some());
}

#[test]
fn multiline_with_calls() {
    let expr = parse_expr("[\n  f(1),\n  g(2),\n]");
    let elems = array_elements(&expr);
    assert_eq!(elems.len(), 2);
    assert!(matches!(&elems[0].kind, ExprKind::Call { .. }));
    assert!(matches!(&elems[1].kind, ExprKind::Call { .. }));
}

#[test]
fn multiline_blank_lines_between_elements() {
    let expr = parse_expr("[\n  1,\n\n  2,\n\n  3\n]");
    assert_eq!(array_elements(&expr).len(), 3);
}

// -- Call trailing comma and newline tests --

#[test]
fn call_trailing_comma_single_line() {
    let expr = parse_expr("f(1, 2,)");
    match &expr.kind {
        ExprKind::Call { args, .. } => assert_eq!(args.len(), 2),
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_trailing_comma_multiline() {
    let expr = parse_expr("f(\n  1,\n  2,\n)");
    match &expr.kind {
        ExprKind::Call { args, .. } => assert_eq!(args.len(), 2),
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_multiline_no_trailing_comma() {
    let expr = parse_expr("f(\n  1,\n  2\n)");
    match &expr.kind {
        ExprKind::Call { args, .. } => assert_eq!(args.len(), 2),
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_single_arg_trailing_comma() {
    let expr = parse_expr("f(1,)");
    match &expr.kind {
        ExprKind::Call { args, .. } => assert_eq!(args.len(), 1),
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_newline_after_open_paren() {
    let expr = parse_expr("f(\n1\n)");
    match &expr.kind {
        ExprKind::Call { args, .. } => assert_eq!(args.len(), 1),
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn thread_trailing_comma() {
    // a @ f(1, 2,) == f(a, 1, 2)
    let expr = parse_expr("a @ f(1, 2,)");
    match &expr.kind {
        ExprKind::Call { args, .. } => assert_eq!(args.len(), 3),
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn thread_multiline_args() {
    let expr = parse_expr("a @ f(\n  1,\n  2,\n)");
    match &expr.kind {
        ExprKind::Call { args, .. } => assert_eq!(args.len(), 3),
        other => panic!("expected Call, got {other:?}"),
    }
}

// -- Array error cases --

#[test]
fn error_unclosed_array() {
    let err = parse_err("[1, 2");
    assert!(err.contains("end of input") || err.contains("Expected ]"), "error was: {err}");
}

#[test]
fn error_missing_comma() {
    let err = parse_err("[1 2]");
    assert!(err.contains("unexpected") || err.contains("Expected"), "error was: {err}");
}

#[test]
fn error_double_comma() {
    let err = parse_err("[1,, 2]");
    assert!(err.contains("unexpected"), "error was: {err}");
}

#[test]
fn error_leading_comma() {
    let err = parse_err("[, 1]");
    assert!(err.contains("unexpected"), "error was: {err}");
}

#[test]
fn error_only_comma() {
    let err = parse_err("[,]");
    assert!(err.contains("unexpected"), "error was: {err}");
}
