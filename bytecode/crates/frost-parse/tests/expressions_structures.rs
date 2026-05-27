mod helpers;

use frost_parse::ast::*;
use helpers::*;

fn array_elements(expr: &Expr) -> &[Expr] {
    match &expr.kind {
        ExprKind::Array(elems) => elems,
        other => panic!("expected Array, got {other:?}"),
    }
}

fn map_entries(expr: &Expr) -> &[MapEntry] {
    match &expr.kind {
        ExprKind::Map(entries) => entries,
        other => panic!("expected Map, got {other:?}"),
    }
}

fn str_key(entry: &MapEntry) -> &[u8] {
    match &entry.key.kind {
        ExprKind::Literal(Literal::String(s)) => s,
        other => panic!("expected String key, got {other:?}"),
    }
}

mod array_literals {
    use super::*;

    #[test]
    fn empty() {
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
        assert_eq!(array_elements(&expr).len(), 2);
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
    fn nested() {
        let expr = parse_expr("[[1, 2], [3, 4]]");
        let elems = array_elements(&expr);
        assert_eq!(elems.len(), 2);
        assert_eq!(array_elements(&elems[0]).len(), 2);
        assert_eq!(array_elements(&elems[1]).len(), 2);
    }

    #[test]
    fn deeply_nested() {
        let expr = parse_expr("[[[1]]]");
        let outer = array_elements(&expr);
        assert_eq!(outer.len(), 1);
        let mid = array_elements(&outer[0]);
        assert_eq!(mid.len(), 1);
        let inner = array_elements(&mid[0]);
        assert_eq!(inner.len(), 1);
        assert!(is_int(&inner[0], 1));
    }

    #[test]
    fn heterogeneous_elements() {
        let expr = parse_expr("[1, true, null, foo]");
        let elems = array_elements(&expr);
        assert_eq!(elems.len(), 4);
        assert!(is_int(&elems[0], 1));
        assert!(matches!(
            &elems[1].kind,
            ExprKind::Literal(Literal::Bool(true))
        ));
        assert!(matches!(&elems[2].kind, ExprKind::Literal(Literal::Null)));
        assert!(matches!(&elems[3].kind, ExprKind::NameLookup(n) if n == "foo"));
    }

    #[test]
    fn string_elements() {
        let expr = parse_expr("['foo', 'bar']");
        assert_eq!(array_elements(&expr).len(), 2);
    }

    #[test]
    fn call_in_array() {
        let expr = parse_expr("[f(1), g(2)]");
        let elems = array_elements(&expr);
        assert_eq!(elems.len(), 2);
        assert!(matches!(&elems[0].kind, ExprKind::Call { .. }));
        assert!(matches!(&elems[1].kind, ExprKind::Call { .. }));
    }

    #[test]
    fn in_def() {
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
    fn indexing() {
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
    fn concatenation() {
        let expr = parse_expr("[1, 2] + [3, 4]");
        let (left, op, right) = is_binop(&expr).unwrap();
        assert!(matches!(op, BinOp::Add));
        assert_eq!(array_elements(left).len(), 2);
        assert_eq!(array_elements(right).len(), 2);
    }
}

mod array_multiline {
    use super::*;

    #[test]
    fn simple() {
        let expr = parse_expr("[\n  1,\n  2,\n  3\n]");
        assert_eq!(array_elements(&expr).len(), 3);
    }

    #[test]
    fn trailing_comma_then_newline() {
        let expr = parse_expr("[\n  1,\n  2,\n]");
        assert_eq!(array_elements(&expr).len(), 2);
    }

    #[test]
    fn nested() {
        let expr = parse_expr("[\n  [1, 2],\n  [3, 4],\n]");
        let elems = array_elements(&expr);
        assert_eq!(elems.len(), 2);
        assert_eq!(array_elements(&elems[0]).len(), 2);
        assert_eq!(array_elements(&elems[1]).len(), 2);
    }

    #[test]
    fn with_expressions() {
        let expr = parse_expr("[\n  1 + 2,\n  3 * 4,\n]");
        let elems = array_elements(&expr);
        assert_eq!(elems.len(), 2);
        assert!(is_binop(&elems[0]).is_some());
        assert!(is_binop(&elems[1]).is_some());
    }

    #[test]
    fn with_calls() {
        let expr = parse_expr("[\n  f(1),\n  g(2),\n]");
        let elems = array_elements(&expr);
        assert_eq!(elems.len(), 2);
        assert!(matches!(&elems[0].kind, ExprKind::Call { .. }));
        assert!(matches!(&elems[1].kind, ExprKind::Call { .. }));
    }

    #[test]
    fn blank_lines_between_elements() {
        let expr = parse_expr("[\n  1,\n\n  2,\n\n  3\n]");
        assert_eq!(array_elements(&expr).len(), 3);
    }

    #[test]
    fn empty() {
        let expr = parse_expr("[\n]");
        assert!(array_elements(&expr).is_empty());
    }
}

mod array_errors {
    use super::*;

    #[test]
    fn unclosed() {
        let err = parse_err("[1, 2");
        assert!(
            err.contains("end of input") || err.contains("Expected ]"),
            "error was: {err}"
        );
    }

    #[test]
    fn missing_comma() {
        let err = parse_err("[1 2]");
        assert!(
            err.contains("unexpected") || err.contains("Expected"),
            "error was: {err}"
        );
    }

    #[test]
    fn double_comma() {
        let err = parse_err("[1,, 2]");
        assert!(err.contains("unexpected"), "error was: {err}");
    }

    #[test]
    fn leading_comma() {
        let err = parse_err("[, 1]");
        assert!(err.contains("unexpected"), "error was: {err}");
    }

    #[test]
    fn only_comma() {
        let err = parse_err("[,]");
        assert!(err.contains("unexpected"), "error was: {err}");
    }
}

mod call_trailing_comma_and_newlines {
    use super::*;

    #[test]
    fn trailing_comma_single_line() {
        let expr = parse_expr("f(1, 2,)");
        match &expr.kind {
            ExprKind::Call { args, .. } => assert_eq!(args.len(), 2),
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn trailing_comma_multiline() {
        let expr = parse_expr("f(\n  1,\n  2,\n)");
        match &expr.kind {
            ExprKind::Call { args, .. } => assert_eq!(args.len(), 2),
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn multiline_no_trailing_comma() {
        let expr = parse_expr("f(\n  1,\n  2\n)");
        match &expr.kind {
            ExprKind::Call { args, .. } => assert_eq!(args.len(), 2),
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn single_arg_trailing_comma() {
        let expr = parse_expr("f(1,)");
        match &expr.kind {
            ExprKind::Call { args, .. } => assert_eq!(args.len(), 1),
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn newline_after_open_paren() {
        let expr = parse_expr("f(\n1\n)");
        match &expr.kind {
            ExprKind::Call { args, .. } => assert_eq!(args.len(), 1),
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn thread_trailing_comma() {
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
}

mod map_literals {
    use super::*;

    #[test]
    fn empty() {
        let expr = parse_expr("{}");
        assert!(map_entries(&expr).is_empty());
    }

    #[test]
    fn single_identifier_key() {
        let expr = parse_expr("{foo: 42}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 1);
        assert_eq!(str_key(&entries[0]), b"foo");
        assert!(is_int(&entries[0].value, 42));
    }

    #[test]
    fn multiple_identifier_keys() {
        let expr = parse_expr("{foo: 1, bar: 2, baz: 3}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 3);
        assert_eq!(str_key(&entries[0]), b"foo");
        assert_eq!(str_key(&entries[1]), b"bar");
        assert_eq!(str_key(&entries[2]), b"baz");
    }

    #[test]
    fn computed_key() {
        let expr = parse_expr("{[42]: 'wow'}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 1);
        assert!(is_int(&entries[0].key, 42));
    }

    #[test]
    fn computed_key_expression() {
        let expr = parse_expr("{[1 + 2]: 'three'}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 1);
        assert!(is_binop(&entries[0].key).is_some());
    }

    #[test]
    fn mixed_key_styles() {
        let expr = parse_expr("{foo: 1, [42]: 2, bar: 3}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 3);
        assert_eq!(str_key(&entries[0]), b"foo");
        assert!(is_int(&entries[1].key, 42));
        assert_eq!(str_key(&entries[2]), b"bar");
    }

    #[test]
    fn trailing_comma() {
        let expr = parse_expr("{foo: 1, bar: 2,}");
        assert_eq!(map_entries(&expr).len(), 2);
    }

    #[test]
    fn expression_values() {
        let expr = parse_expr("{x: 1 + 2, y: f(3)}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 2);
        assert!(is_binop(&entries[0].value).is_some());
        assert!(matches!(&entries[1].value.kind, ExprKind::Call { .. }));
    }

    #[test]
    fn nested_map() {
        let expr = parse_expr("{outer: {inner: 42}}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 1);
        let inner = map_entries(&entries[0].value);
        assert_eq!(inner.len(), 1);
        assert_eq!(str_key(&inner[0]), b"inner");
        assert!(is_int(&inner[0].value, 42));
    }

    #[test]
    fn map_with_array_value() {
        let expr = parse_expr("{items: [1, 2, 3]}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0].value.kind, ExprKind::Array(_)));
    }

    #[test]
    fn array_of_maps() {
        let expr = parse_expr("[{a: 1}, {b: 2}]");
        let elems = array_elements(&expr);
        assert_eq!(elems.len(), 2);
        assert_eq!(map_entries(&elems[0]).len(), 1);
        assert_eq!(map_entries(&elems[1]).len(), 1);
    }

    #[test]
    fn in_def() {
        let program = parse("def m = {foo: 42}");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StatementKind::Def { expr, .. } => {
                assert_eq!(map_entries(expr).len(), 1);
            }
            other => panic!("expected Def, got {other:?}"),
        }
    }

    #[test]
    fn dot_access() {
        let expr = parse_expr("{foo: 42}.foo");
        assert!(matches!(&expr.kind, ExprKind::Index { .. }));
    }

    #[test]
    fn index_access() {
        let expr = parse_expr("{foo: 42}['foo']");
        assert!(matches!(&expr.kind, ExprKind::Index { .. }));
    }

    #[test]
    fn merge() {
        let expr = parse_expr("{a: 1} + {b: 2}");
        let (left, op, right) = is_binop(&expr).unwrap();
        assert!(matches!(op, BinOp::Add));
        assert_eq!(map_entries(left).len(), 1);
        assert_eq!(map_entries(right).len(), 1);
    }
}

mod map_multiline {
    use super::*;

    #[test]
    fn simple() {
        let expr = parse_expr("{\n  foo: 1,\n  bar: 2\n}");
        assert_eq!(map_entries(&expr).len(), 2);
    }

    #[test]
    fn trailing_comma() {
        let expr = parse_expr("{\n  foo: 1,\n  bar: 2,\n}");
        assert_eq!(map_entries(&expr).len(), 2);
    }

    #[test]
    fn computed_keys() {
        let expr = parse_expr("{\n  [1]: 'one',\n  [2]: 'two',\n}");
        assert_eq!(map_entries(&expr).len(), 2);
    }

    #[test]
    fn blank_lines() {
        let expr = parse_expr("{\n  foo: 1,\n\n  bar: 2,\n}");
        assert_eq!(map_entries(&expr).len(), 2);
    }

    #[test]
    fn empty() {
        let expr = parse_expr("{\n}");
        assert!(map_entries(&expr).is_empty());
    }
}

mod map_errors {
    use super::*;

    #[test]
    fn unclosed() {
        let err = parse_err("{foo: 1");
        assert!(
            err.contains("end of input") || err.contains("Expected }"),
            "error was: {err}"
        );
    }

    #[test]
    fn missing_colon() {
        let err = parse_err("{foo 1}");
        assert!(
            err.contains("Expected :") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    #[test]
    fn missing_value() {
        let err = parse_err("{foo:}");
        assert!(err.contains("unexpected"), "error was: {err}");
    }

    #[test]
    fn double_comma() {
        let err = parse_err("{foo: 1,, bar: 2}");
        assert!(err.contains("unexpected"), "error was: {err}");
    }

    #[test]
    fn bare_number_key() {
        let err = parse_err("{42: 'x'}");
        assert!(err.contains("unexpected"), "error was: {err}");
    }

    #[test]
    fn shorthand_not_allowed() {
        let err = parse_err("{foo}");
        assert!(
            err.contains("Expected :") || err.contains("unexpected"),
            "error was: {err}"
        );
    }
}
