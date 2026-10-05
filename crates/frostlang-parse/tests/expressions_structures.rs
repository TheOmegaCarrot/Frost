mod helpers;

use frostlang_parse::ast::*;
use helpers::*;

fn array_elements(expr: &Spanned<Expr>) -> &[Spanned<Expr>] {
    match &expr.node {
        Expr::Array(elems) => elems,
        other => panic!("expected Array, got {other:?}"),
    }
}

fn map_entries(expr: &Spanned<Expr>) -> &[Spanned<MapEntry>] {
    match &expr.node {
        Expr::Map(entries) => entries,
        other => panic!("expected Map, got {other:?}"),
    }
}

fn str_key(entry: &Spanned<MapEntry>) -> &str {
    match &entry.node.key.node {
        Expr::Literal(Literal::String(s)) => s,
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
        assert!(matches!(&elems[1].node, Expr::Literal(Literal::Bool(true))));
        assert!(matches!(&elems[2].node, Expr::Literal(Literal::Null)));
        assert!(matches!(&elems[3].node, Expr::NameLookup(n) if n == "foo"));
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
        assert!(matches!(&elems[0].node, Expr::Call { .. }));
        assert!(matches!(&elems[1].node, Expr::Call { .. }));
    }

    #[test]
    fn in_def() {
        let program = parse("def xs = [1, 2, 3]");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].node {
            Statement::Def { expr, .. } => {
                assert_eq!(array_elements(expr).len(), 3);
            }
            other => panic!("expected Def, got {other:?}"),
        }
    }

    #[test]
    fn indexing() {
        let expr = parse_expr("[1, 2, 3][0]");
        match &expr.node {
            Expr::SoftIndex { target, key } => {
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
        let source = r"
            [
                1,
                2,
                3
            ]
        ";
        let expr = parse_expr(source);
        assert_eq!(array_elements(&expr).len(), 3);
    }

    #[test]
    fn trailing_comma_then_newline() {
        let source = r"
            [
                1,
                2,
            ]
        ";
        let expr = parse_expr(source);
        assert_eq!(array_elements(&expr).len(), 2);
    }

    #[test]
    fn nested() {
        let source = r"
            [
                [1, 2],
                [3, 4],
            ]
        ";
        let expr = parse_expr(source);
        let elems = array_elements(&expr);
        assert_eq!(elems.len(), 2);
        assert_eq!(array_elements(&elems[0]).len(), 2);
        assert_eq!(array_elements(&elems[1]).len(), 2);
    }

    #[test]
    fn with_expressions() {
        let source = r"
            [
                1 + 2,
                3 * 4,
            ]
        ";
        let expr = parse_expr(source);
        let elems = array_elements(&expr);
        assert_eq!(elems.len(), 2);
        assert!(is_binop(&elems[0]).is_some());
        assert!(is_binop(&elems[1]).is_some());
    }

    #[test]
    fn with_calls() {
        let source = r"
            [
                f(1),
                g(2),
            ]
        ";
        let expr = parse_expr(source);
        let elems = array_elements(&expr);
        assert_eq!(elems.len(), 2);
        assert!(matches!(&elems[0].node, Expr::Call { .. }));
        assert!(matches!(&elems[1].node, Expr::Call { .. }));
    }

    #[test]
    fn blank_lines_between_elements() {
        let source = r"
            [
                1,

                2,

                3
            ]
        ";
        let expr = parse_expr(source);
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
        assert_eq!(
            parse_err_message("[1, 2"),
            "expected `,` or `]`, but found the end of input"
        );
    }

    #[test]
    fn missing_comma() {
        assert_eq!(
            parse_err_message("[1 2]"),
            "expected `,` or `]`, but found `2`"
        );
    }

    #[test]
    fn double_comma() {
        assert_eq!(
            parse_err_message("[1,, 2]"),
            "expected an expression, but found `,`"
        );
    }

    #[test]
    fn leading_comma() {
        assert_eq!(
            parse_err_message("[, 1]"),
            "expected an expression, but found `,`"
        );
    }

    #[test]
    fn only_comma() {
        assert_eq!(
            parse_err_message("[,]"),
            "expected an expression, but found `,`"
        );
    }
}

mod call_trailing_comma_and_newlines {
    use super::*;

    #[test]
    fn trailing_comma_single_line() {
        let expr = parse_expr("f(1, 2,)");
        match &expr.node {
            Expr::Call { args, .. } => assert_eq!(args.len(), 2),
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn trailing_comma_multiline() {
        let source = r"
            f(
                1,
                2,
            )
        ";
        let expr = parse_expr(source);
        match &expr.node {
            Expr::Call { args, .. } => assert_eq!(args.len(), 2),
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn multiline_no_trailing_comma() {
        let source = r"
            f(
                1,
                2
            )
        ";
        let expr = parse_expr(source);
        match &expr.node {
            Expr::Call { args, .. } => assert_eq!(args.len(), 2),
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn single_arg_trailing_comma() {
        let expr = parse_expr("f(1,)");
        match &expr.node {
            Expr::Call { args, .. } => assert_eq!(args.len(), 1),
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn newline_after_open_paren() {
        let expr = parse_expr("f(\n1\n)");
        match &expr.node {
            Expr::Call { args, .. } => assert_eq!(args.len(), 1),
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn thread_trailing_comma() {
        let expr = parse_expr("a @ f(1, 2,)");
        match &expr.node {
            Expr::Call { args, .. } => assert_eq!(args.len(), 3),
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn thread_multiline_args() {
        let source = r"
            a @ f(
                1,
                2,
            )
        ";
        let expr = parse_expr(source);
        match &expr.node {
            Expr::Call { args, .. } => assert_eq!(args.len(), 3),
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
        assert_eq!(str_key(&entries[0]), "foo");
        assert!(is_int(&entries[0].node.value, 42));
    }

    #[test]
    fn multiple_identifier_keys() {
        let expr = parse_expr("{foo: 1, bar: 2, baz: 3}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 3);
        assert_eq!(str_key(&entries[0]), "foo");
        assert_eq!(str_key(&entries[1]), "bar");
        assert_eq!(str_key(&entries[2]), "baz");
    }

    #[test]
    fn computed_key() {
        let expr = parse_expr("{[42]: 'wow'}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 1);
        assert!(is_int(&entries[0].node.key, 42));
    }

    #[test]
    fn computed_key_expression() {
        let expr = parse_expr("{[1 + 2]: 'three'}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 1);
        assert!(is_binop(&entries[0].node.key).is_some());
    }

    #[test]
    fn negative_computed_key() {
        let expr = parse_expr("{[-1]: v}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 1);
        assert!(matches!(
            &entries[0].node.key.node,
            Expr::UnaryOp {
                op: Spanned {
                    node: UnaryOp::Negate,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn reserved_keyword_key() {
        // A reserved word cannot be an identifier map key (shorthand or `key:`).
        assert_eq!(
            parse_err_message("{if: 1}"),
            "expected a name or `[`, but found `if`"
        );
    }

    #[test]
    fn mixed_key_styles() {
        let expr = parse_expr("{foo: 1, [42]: 2, bar: 3}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 3);
        assert_eq!(str_key(&entries[0]), "foo");
        assert!(is_int(&entries[1].node.key, 42));
        assert_eq!(str_key(&entries[2]), "bar");
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
        assert!(is_binop(&entries[0].node.value).is_some());
        assert!(matches!(&entries[1].node.value.node, Expr::Call { .. }));
    }

    #[test]
    fn nested_map() {
        let expr = parse_expr("{outer: {inner: 42}}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 1);
        let inner = map_entries(&entries[0].node.value);
        assert_eq!(inner.len(), 1);
        assert_eq!(str_key(&inner[0]), "inner");
        assert!(is_int(&inner[0].node.value, 42));
    }

    #[test]
    fn map_with_array_value() {
        let expr = parse_expr("{items: [1, 2, 3]}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0].node.value.node, Expr::Array(_)));
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
        match &program.statements[0].node {
            Statement::Def { expr, .. } => {
                assert_eq!(map_entries(expr).len(), 1);
            }
            other => panic!("expected Def, got {other:?}"),
        }
    }

    #[test]
    fn dot_access() {
        let expr = parse_expr("{foo: 42}.foo");
        assert!(matches!(&expr.node, Expr::HardIndex { .. }));
    }

    #[test]
    fn index_access() {
        let expr = parse_expr("{foo: 42}['foo']");
        assert!(matches!(&expr.node, Expr::SoftIndex { .. }));
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
        let source = r"
            {
                foo: 1,
                bar: 2
            }
        ";
        let expr = parse_expr(source);
        assert_eq!(map_entries(&expr).len(), 2);
    }

    #[test]
    fn trailing_comma() {
        let source = r"
            {
                foo: 1,
                bar: 2,
            }
        ";
        let expr = parse_expr(source);
        assert_eq!(map_entries(&expr).len(), 2);
    }

    #[test]
    fn computed_keys() {
        let source = r"
            {
                [1]: 'one',
                [2]: 'two',
            }
        ";
        let expr = parse_expr(source);
        assert_eq!(map_entries(&expr).len(), 2);
    }

    #[test]
    fn blank_lines() {
        let source = r"
            {
                foo: 1,

                bar: 2,
            }
        ";
        let expr = parse_expr(source);
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
        assert_eq!(
            parse_err_message("{foo: 1"),
            "expected `,` or `}`, but found the end of input"
        );
    }

    #[test]
    fn missing_colon() {
        assert_eq!(
            parse_err_message("{foo 1}"),
            "expected `,` or `}`, but found `1`"
        );
    }

    #[test]
    fn missing_value() {
        assert_eq!(
            parse_err_message("{foo:}"),
            "expected an expression, but found `}`"
        );
    }

    #[test]
    fn double_comma() {
        assert_eq!(
            parse_err_message("{foo: 1,, bar: 2}"),
            "expected a name or `[`, but found `,`"
        );
    }

    #[test]
    fn bare_number_key() {
        assert_eq!(
            parse_err_message("{42: 'x'}"),
            "expected a name or `[`, but found `42`"
        );
    }
}

// `{name}` is shorthand for `{name: name}`, as in Map destructuring and patterns.
mod map_shorthand {
    use super::*;

    /// The name an entry's value looks up, which must be a bare name.
    fn value_name(entry: &Spanned<MapEntry>) -> &str {
        match &entry.node.value.node {
            Expr::NameLookup(name) => name,
            other => panic!("expected NameLookup value, got {other:?}"),
        }
    }

    #[test]
    fn single() {
        let expr = parse_expr("{foo}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 1);
        assert_eq!(str_key(&entries[0]), "foo");
        assert_eq!(value_name(&entries[0]), "foo");
    }

    #[test]
    fn same_tree_as_explicit_form() {
        assert_eq!(parse_expr("{foo, bar}"), parse_expr("{foo: foo, bar: bar}"));
    }

    // The key, the value, and the entry all span just the name.
    #[test]
    fn spans_cover_the_name() {
        let expr = parse_expr("{ foo }");
        let entry = &map_entries(&expr)[0];
        let name = SourceSpan { start: 2, end: 5 };
        assert_eq!(entry.span, name, "entry span");
        assert_eq!(entry.node.key.span, name, "key span");
        assert_eq!(entry.node.value.span, name, "value span");
    }

    #[test]
    fn mixed_with_other_entries() {
        let expr = parse_expr("{sku, qty: 2, [3]: 'three', price}");
        let entries = map_entries(&expr);
        assert_eq!(entries.len(), 4);
        assert_eq!(value_name(&entries[0]), "sku");
        assert!(is_int(&entries[1].node.value, 2));
        assert!(is_int(&entries[2].node.key, 3));
        assert_eq!(value_name(&entries[3]), "price");
    }

    #[test]
    fn trailing_comma() {
        let expr = parse_expr("{foo, bar,}");
        assert_eq!(map_entries(&expr).len(), 2);
    }

    #[test]
    fn multiline() {
        let source = r"
            {
                foo,
                bar: 2,
                baz,
            }
        ";
        let expr = parse_expr(source);
        assert_eq!(map_entries(&expr).len(), 3);
    }

    #[test]
    fn reserved_word_is_rejected() {
        assert_eq!(
            parse_err_message("{if}"),
            "expected a name or `[`, but found `if`"
        );
    }

    // A dollar identifier is not a valid key name, so it has no shorthand.
    #[test]
    fn dollar_identifier_is_rejected() {
        assert_eq!(
            parse_err_message("$({$})"),
            "expected a name or `[`, but found `$`"
        );
    }
}
