mod helpers;

use frostlang_parse::ast::*;
use helpers::*;

fn assert_lambda(expr: &Spanned<Expr>) -> LambdaParts<'_> {
    match &expr.node {
        Expr::Lambda {
            params,
            variadic_param,
            self_name,
            body,
            return_expr,
        } => LambdaParts {
            params,
            variadic_param: variadic_param.as_ref(),
            self_name: self_name.as_ref().map(|s| s.node.as_str()),
            body,
            return_expr,
        },
        other => panic!("expected Lambda, got {other:?}"),
    }
}

struct LambdaParts<'a> {
    params: &'a [Spanned<Binding>],
    variadic_param: Option<&'a Spanned<Binding>>,
    self_name: Option<&'a str>,
    body: &'a [Spanned<Statement>],
    return_expr: &'a Spanned<Expr>,
}

fn is_named(binding: &Spanned<Binding>, expected: &str) -> bool {
    matches!(&binding.node, Binding::Named(n) if n == expected)
}

// ============================================================
// No parameters
// ============================================================

mod no_params {
    use super::*;

    #[test]
    fn bare_arrow() {
        let expr = parse_expr("fn -> 42");
        let lam = assert_lambda(&expr);
        assert!(lam.params.is_empty());
        assert!(lam.variadic_param.is_none());
        assert!(lam.self_name.is_none());
        assert!(lam.body.is_empty());
        assert!(is_int(lam.return_expr, 42));
    }

    #[test]
    fn named_no_params() {
        let expr = parse_expr("fn f() -> 42");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.self_name, Some("f"));
        assert!(lam.params.is_empty());
        assert!(lam.variadic_param.is_none());
        assert!(is_int(lam.return_expr, 42));
    }
}

// ============================================================
// Bare (unparenthesized) parameters
// ============================================================

mod bare_params {
    use super::*;

    #[test]
    fn single_param() {
        let expr = parse_expr("fn x -> x + 1");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 1);
        assert!(is_named(&lam.params[0], "x"));
        assert!(lam.self_name.is_none());
    }

    #[test]
    fn multiple_params() {
        let expr = parse_expr("fn x, y -> x + y");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 2);
        assert!(is_named(&lam.params[0], "x"));
        assert!(is_named(&lam.params[1], "y"));
    }

    #[test]
    fn three_params() {
        let expr = parse_expr("fn a, b, c -> a");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 3);
    }

    #[test]
    fn with_variadic() {
        let expr = parse_expr("fn x, ...rest -> rest");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 1);
        assert!(is_named(&lam.params[0], "x"));
        assert!(is_named(lam.variadic_param.expect("variadic"), "rest"));
    }

    #[test]
    fn variadic_only() {
        let expr = parse_expr("fn ...args -> args");
        let lam = assert_lambda(&expr);
        assert!(lam.params.is_empty());
        assert!(is_named(lam.variadic_param.expect("variadic"), "args"));
        assert!(lam.self_name.is_none());
    }

    #[test]
    fn discard_param() {
        let expr = parse_expr("fn _ -> 42");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 1);
        assert_eq!(lam.params[0].node, Binding::Discarded);
    }
}

// ============================================================
// Parenthesized parameters
// ============================================================

mod parenthesized_params {
    use super::*;

    #[test]
    fn single_param() {
        let expr = parse_expr("fn(x) -> x");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 1);
        assert!(is_named(&lam.params[0], "x"));
        assert!(lam.self_name.is_none());
    }

    #[test]
    fn multiple_params() {
        let expr = parse_expr("fn(x, y) -> x + y");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 2);
    }

    #[test]
    fn empty_parens() {
        let expr = parse_expr("fn() -> 42");
        let lam = assert_lambda(&expr);
        assert!(lam.params.is_empty());
        assert!(lam.variadic_param.is_none());
        assert!(lam.self_name.is_none());
    }

    #[test]
    fn with_variadic() {
        let expr = parse_expr("fn(a, ...rest) -> rest");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 1);
        assert!(is_named(lam.variadic_param.expect("variadic"), "rest"));
    }

    #[test]
    fn variadic_only() {
        let expr = parse_expr("fn(...args) -> args");
        let lam = assert_lambda(&expr);
        assert!(lam.params.is_empty());
        assert!(is_named(lam.variadic_param.expect("variadic"), "args"));
    }

    #[test]
    fn trailing_comma() {
        let expr = parse_expr("fn(x, y,) -> x + y");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 2);
    }

    #[test]
    fn discard_params() {
        let expr = parse_expr("fn(_, y) -> y");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params[0].node, Binding::Discarded);
        assert!(is_named(&lam.params[1], "y"));
    }

    #[test]
    fn multiline() {
        let source = r"
            fn(
                x,
                y
            ) -> x + y
        ";
        let expr = parse_expr(source);
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 2);
    }
}

// ============================================================
// Named (recursive) lambdas
// ============================================================

mod named {
    use super::*;

    #[test]
    fn named_with_params() {
        let expr = parse_expr("fn add(x, y) -> x + y");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.self_name, Some("add"));
        assert_eq!(lam.params.len(), 2);
    }

    #[test]
    fn named_with_variadic() {
        let expr = parse_expr("fn f(a, ...rest) -> rest");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.self_name, Some("f"));
        assert_eq!(lam.params.len(), 1);
        assert!(lam.variadic_param.is_some());
    }

    #[test]
    fn named_variadic_only() {
        let expr = parse_expr("fn f(...args) -> args");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.self_name, Some("f"));
        assert!(lam.params.is_empty());
        assert!(is_named(lam.variadic_param.expect("variadic"), "args"));
    }

    #[test]
    fn recursive_usage() {
        let expr = parse_expr("fn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.self_name, Some("fact"));
        assert_eq!(lam.params.len(), 1);
        assert!(matches!(&lam.return_expr.node, Expr::If { .. }));
    }
}

// ============================================================
// Block body
// ============================================================

mod block_body {
    use super::*;

    #[test]
    fn single_def_then_expr() {
        let expr = parse_expr("fn x -> { def y = 1; y }");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.body.len(), 1);
        assert!(matches!(&lam.body[0].node, Statement::Def { .. }));
        assert!(matches!(&lam.return_expr.node, Expr::NameLookup(n) if n == "y"));
    }

    #[test]
    fn multiple_defs() {
        let expr = parse_expr("fn x -> { def a = 1; def b = 2; a + b + x }");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.body.len(), 2);
        assert!(is_binop(lam.return_expr).is_some());
    }

    #[test]
    fn no_params_block() {
        let expr = parse_expr("fn -> { def x = 1; x }");
        let lam = assert_lambda(&expr);
        assert!(lam.params.is_empty());
        assert_eq!(lam.body.len(), 1);
    }

    #[test]
    fn single_expression_in_block() {
        let expr = parse_expr("fn -> { 42 }");
        let lam = assert_lambda(&expr);
        assert!(lam.body.is_empty());
        assert!(is_int(lam.return_expr, 42));
    }

    #[test]
    fn multiline_block() {
        let source = r"
            fn x -> {
                def a = 1
                def b = 2
                a + b + x
            }
        ";
        let expr = parse_expr(source);
        let lam = assert_lambda(&expr);
        assert_eq!(lam.body.len(), 2);
    }

    #[test]
    fn named_with_block() {
        let expr = parse_expr("fn f(x) -> { def y = x + 1; y }");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.self_name, Some("f"));
        assert_eq!(lam.body.len(), 1);
    }
}

// ============================================================
// Map body (not a block)
// ============================================================

mod map_body {
    use super::*;

    #[test]
    fn simple_map() {
        let expr = parse_expr("fn -> {foo: 1}");
        let lam = assert_lambda(&expr);
        assert!(lam.body.is_empty());
        assert!(matches!(&lam.return_expr.node, Expr::Map(_)));
    }

    #[test]
    fn map_with_param() {
        let expr = parse_expr("fn x -> {name: x}");
        let lam = assert_lambda(&expr);
        assert!(lam.body.is_empty());
        assert!(matches!(&lam.return_expr.node, Expr::Map(_)));
    }

    #[test]
    fn map_with_multiple_entries() {
        let expr = parse_expr("fn x -> {name: x, age: 30}");
        let lam = assert_lambda(&expr);
        assert!(matches!(&lam.return_expr.node, Expr::Map(entries) if entries.len() == 2));
    }

    // `fn -> {}` is a thunk returning an empty Map, not an empty block.
    #[test]
    fn empty_map_thunk() {
        let expr = parse_expr("fn -> {}");
        let lam = assert_lambda(&expr);
        assert!(lam.params.is_empty());
        assert!(lam.variadic_param.is_none());
        assert!(lam.self_name.is_none());
        assert!(lam.body.is_empty());
        assert!(matches!(&lam.return_expr.node, Expr::Map(entries) if entries.is_empty()));
    }

    // -- A multiline Map body --
    // A Map literal ignores newlines inside its braces, so a body whose first
    // entry starts on the line after `{` is still a Map, not a block.

    /// The entry count of `lam`'s body, which must be a bare Map.
    fn map_body_len(lam: &LambdaParts) -> usize {
        assert!(lam.body.is_empty(), "a Map body has no statements");
        match &lam.return_expr.node {
            Expr::Map(entries) => entries.len(),
            other => panic!("expected Map body, got {other:?}"),
        }
    }

    #[test]
    fn multiline_map() {
        let source = r"
            fn x -> {
                a: x,
                b: x
            }
        ";
        let expr = parse_expr(source);
        assert_eq!(map_body_len(&assert_lambda(&expr)), 2);
    }

    #[test]
    fn multiline_map_with_trailing_comma() {
        let source = r"
            fn x -> {
                a: x,
                b: x,
            }
        ";
        let expr = parse_expr(source);
        assert_eq!(map_body_len(&assert_lambda(&expr)), 2);
    }

    #[test]
    fn multiline_map_after_blank_lines_and_a_comment() {
        let source = r"
            fn x -> { # a note

                a: x
            }
        ";
        let expr = parse_expr(source);
        assert_eq!(map_body_len(&assert_lambda(&expr)), 1);
    }

    #[test]
    fn multiline_map_with_computed_key() {
        let source = r"
            fn x -> {
                [x]: 1,
                b: 2
            }
        ";
        let expr = parse_expr(source);
        assert_eq!(map_body_len(&assert_lambda(&expr)), 2);
    }

    #[test]
    fn multiline_empty_map() {
        let source = r"
            fn -> {
            }
        ";
        let expr = parse_expr(source);
        assert_eq!(map_body_len(&assert_lambda(&expr)), 0);
    }

    #[test]
    fn multiline_map_in_defn() {
        let source = r"
            defn f(x) -> {
                a: x,
                b: x
            }
        ";
        let program = parse(source);
        let [statement] = program.statements.as_slice() else {
            panic!("expected one statement, got {program:?}");
        };
        let Statement::Def { expr, .. } = &statement.node else {
            panic!("expected Def, got {statement:?}");
        };
        assert_eq!(map_body_len(&assert_lambda(expr)), 2);
    }

    // -- Shorthand entries --
    // A comma after a leading name cannot continue a block statement, so
    // `{a, ...` is a Map. A lone `{a}` is a Map too: as a block it would only
    // mean `fn -> a`.

    #[test]
    fn lone_shorthand() {
        let expr = parse_expr("fn a -> {a}");
        let lam = assert_lambda(&expr);
        assert_eq!(map_body_len(&lam), 1);
        let Expr::Map(entries) = &lam.return_expr.node else {
            unreachable!("map_body_len checked for a Map");
        };
        assert!(
            matches!(&entries[0].node.value.node, Expr::NameLookup(n) if n == "a"),
            "{entries:?}"
        );
    }

    #[test]
    fn multiline_lone_shorthand() {
        let source = r"
            fn a -> {
                a
            }
        ";
        let expr = parse_expr(source);
        assert_eq!(map_body_len(&assert_lambda(&expr)), 1);
    }

    #[test]
    fn shorthand_entries() {
        let expr = parse_expr("fn a, b -> {a, b}");
        assert_eq!(map_body_len(&assert_lambda(&expr)), 2);
    }

    #[test]
    fn shorthand_then_explicit_entry() {
        let expr = parse_expr("fn a -> {a, b: 2}");
        assert_eq!(map_body_len(&assert_lambda(&expr)), 2);
    }

    #[test]
    fn multiline_shorthand_entries() {
        let source = r"
            fn a, b -> {
                a,
                b
            }
        ";
        let expr = parse_expr(source);
        assert_eq!(map_body_len(&assert_lambda(&expr)), 2);
    }

    // A block may still begin with a bare name, when another statement follows it.
    #[test]
    fn bare_name_then_another_statement_is_a_block() {
        let source = r"
            fn -> {
                a
                b
            }
        ";
        let expr = parse_expr(source);
        let lam = assert_lambda(&expr);
        assert_eq!(lam.body.len(), 1);
        assert!(matches!(&lam.return_expr.node, Expr::NameLookup(n) if n == "b"));
    }

    // Only a bare name has a shorthand entry; any other lone expression is a block.
    #[test]
    fn lone_non_name_expression_is_a_block() {
        for source in ["fn -> {a.b}", "fn -> {f(a)}", "fn -> {a + 1}"] {
            let expr = parse_expr(source);
            let lam = assert_lambda(&expr);
            assert!(lam.body.is_empty(), "{source}");
            assert!(
                !matches!(&lam.return_expr.node, Expr::Map(_)),
                "{source}: {:?}",
                lam.return_expr
            );
        }
    }

    // `{ [` first tries a Map with a computed key; an Array statement is not
    // one, so the body falls back to a block.
    #[test]
    fn multiline_block_starting_with_an_array_is_still_a_block() {
        let source = r"
            fn -> {
                [1, 2]
            }
        ";
        let expr = parse_expr(source);
        let lam = assert_lambda(&expr);
        assert!(lam.body.is_empty());
        assert!(matches!(&lam.return_expr.node, Expr::Array(elements) if elements.len() == 2));
    }
}

// ============================================================
// A body on the line after its arrow
// ============================================================

mod body_after_newline {
    use super::*;

    #[test]
    fn expression_body() {
        let source = r"
            fn x ->
                x + 1
        ";
        let expr = parse_expr(source);
        let lam = assert_lambda(&expr);
        assert!(lam.body.is_empty());
        assert!(is_binop(lam.return_expr).is_some());
    }

    #[test]
    fn block_body() {
        let source = r"
            fn x ->
            {
                def y = 1
                x + y
            }
        ";
        let expr = parse_expr(source);
        let lam = assert_lambda(&expr);
        assert_eq!(lam.body.len(), 1);
        assert!(is_binop(lam.return_expr).is_some());
    }

    #[test]
    fn map_body() {
        let source = r"
            fn ->
                {foo: 1}
        ";
        let expr = parse_expr(source);
        let lam = assert_lambda(&expr);
        assert!(lam.body.is_empty());
        assert!(matches!(&lam.return_expr.node, Expr::Map(_)));
    }

    #[test]
    fn after_blank_lines_and_a_comment() {
        let source = r"
            fn x -> # a note

                x
        ";
        let expr = parse_expr(source);
        let lam = assert_lambda(&expr);
        assert!(matches!(&lam.return_expr.node, Expr::NameLookup(n) if n == "x"));
    }

    #[test]
    fn curried() {
        let source = r"
            fn x -> fn y ->
                x + y
        ";
        let expr = parse_expr(source);
        let outer = assert_lambda(&expr);
        let inner = assert_lambda(outer.return_expr);
        assert!(is_binop(inner.return_expr).is_some());
    }

    #[test]
    fn defn_body() {
        let source = r"
            defn inc(x) ->
                x + 1
        ";
        let program = parse(source);
        let [statement] = program.statements.as_slice() else {
            panic!("expected one statement, got {program:?}");
        };
        assert!(
            matches!(&statement.node, Statement::Def { .. }),
            "{statement:?}"
        );
    }

    #[test]
    fn the_body_still_ends_at_its_newline() {
        let source = r"
            def f = fn x ->
                x
            f(1)
        ";
        let program = parse(source);
        assert_eq!(program.statements.len(), 2, "{program:?}");
    }
}

// ============================================================
// Lambda in expressions
// ============================================================

mod in_expressions {
    use super::*;

    #[test]
    fn in_def() {
        let program = parse("def f = fn x -> x");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].node {
            Statement::Def { expr, .. } => {
                assert_lambda(expr);
            }
            other => panic!("expected Def, got {other:?}"),
        }
    }

    #[test]
    fn in_call_position() {
        let expr = parse_expr("(fn a -> a)(42)");
        match &expr.node {
            Expr::Call { callee, args } => {
                assert_lambda(callee);
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn nested_lambdas() {
        let expr = parse_expr("fn x -> fn y -> x + y");
        let outer = assert_lambda(&expr);
        assert_eq!(outer.params.len(), 1);
        let inner = assert_lambda(outer.return_expr);
        assert_eq!(inner.params.len(), 1);
        assert!(is_binop(inner.return_expr).is_some());
    }

    #[test]
    fn in_call_arg() {
        let expr = parse_expr("f(fn x -> x)");
        match &expr.node {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 1);
                assert_lambda(&args[0]);
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn in_array() {
        let expr = parse_expr("[fn x -> x, fn y -> y]");
        match &expr.node {
            Expr::Array(elems) => {
                assert_eq!(elems.len(), 2);
                assert_lambda(&elems[0]);
                assert_lambda(&elems[1]);
            }
            other => panic!("expected Array, got {other:?}"),
        }
    }
}

// ============================================================
// Errors
// ============================================================

mod errors {
    use super::*;

    #[test]
    fn missing_arrow() {
        assert_eq!(parse_err_message("fn x x"), "expected `->`, but found `x`");
    }

    #[test]
    fn missing_body() {
        // Even with lines after the arrow, there must be a body.
        for source in ["fn x ->", "fn x ->\n", "fn x ->\n\n# only a comment\n"] {
            assert_eq!(
                parse_err_message(source),
                "expected an expression, but found the end of input",
                "{source:?}"
            );
        }
    }

    #[test]
    fn block_body_ending_with_def() {
        let err = parse_err("fn -> { def x = 1 }");
        assert!(err.contains("end with an expression"), "error was: {err}");
    }

    #[test]
    fn unclosed_parens() {
        assert_eq!(
            parse_err_message("fn(x, y -> x"),
            "expected `,` or `)`, but found `->`"
        );
    }

    #[test]
    fn number_as_param() {
        assert_eq!(
            parse_err_message("fn 42 -> 42"),
            "expected parameters or `->`, but found `42`"
        );
    }

    #[test]
    fn dollar_id_outside_abbreviated_lambda() {
        let source = r"
            def a = $($ + 2)
            def $3 = 2
        ";
        assert_eq!(
            parse_err_message(source),
            "expected a name, `[`, or `{`, but found `$3`"
        );
    }

    // A block-body lambda follows top-level grammar (minus `export`):
    // adjacent expressions require a separator.
    #[test]
    fn block_body_missing_separator() {
        assert_eq!(
            parse_err_message("fn -> { 1 2 }"),
            "expected a line break or `;`, but found `2`"
        );
    }
}

// ============================================================
// Abbreviated lambdas
// ============================================================

mod abbreviated {
    use super::*;

    // The parser preserves the body verbatim with placeholders left as name
    // lookups, and summarizes dollar usage on the wrapper for the compiler.
    fn assert_abbreviated(expr: &Spanned<Expr>) -> &Spanned<Expr> {
        match &expr.node {
            Expr::AbbreviatedLambda { body, .. } => body,
            other => panic!("expected AbbreviatedLambda, got {other:?}"),
        }
    }

    /// The `(used_params, uses_rest)` summary of an abbreviated lambda.
    fn usage(expr: &Spanned<Expr>) -> (&[bool], bool) {
        match &expr.node {
            Expr::AbbreviatedLambda {
                used_params,
                uses_rest,
                ..
            } => (used_params, *uses_rest),
            other => panic!("expected AbbreviatedLambda, got {other:?}"),
        }
    }

    fn is_dollar(expr: &Spanned<Expr>, name: &str) -> bool {
        matches!(&expr.node, Expr::NameLookup(n) if n == name)
    }

    #[test]
    fn single_dollar() {
        let expr = parse_expr("$($ * 2)");
        let body = assert_abbreviated(&expr);
        let (l, op, r) = is_binop(body).expect("binop body");
        assert!(is_dollar(l, "$"));
        assert!(matches!(op, BinOp::Mul));
        assert!(is_int(r, 2));
    }

    #[test]
    fn numbered_params() {
        let expr = parse_expr("$($1 + $2)");
        let body = assert_abbreviated(&expr);
        let (l, op, r) = is_binop(body).expect("binop body");
        assert!(is_dollar(l, "$1"));
        assert!(matches!(op, BinOp::Add));
        assert!(is_dollar(r, "$2"));
    }

    #[test]
    fn rest_only() {
        let expr = parse_expr("$($$)");
        let body = assert_abbreviated(&expr);
        assert!(is_dollar(body, "$$"));
    }

    #[test]
    fn numbered_with_rest() {
        let expr = parse_expr("$($ + $$)");
        let body = assert_abbreviated(&expr);
        let (l, _, r) = is_binop(body).expect("binop body");
        assert!(is_dollar(l, "$"));
        assert!(is_dollar(r, "$$"));
    }

    #[test]
    fn gap_filling() {
        let expr = parse_expr("$($9)");
        let body = assert_abbreviated(&expr);
        assert!(is_dollar(body, "$9"));
    }

    #[test]
    fn no_dollar_ids() {
        let expr = parse_expr("$(42)");
        let body = assert_abbreviated(&expr);
        assert!(is_int(body, 42));
    }

    #[test]
    fn dollar_in_call() {
        let expr = parse_expr("$(f($))");
        let body = assert_abbreviated(&expr);
        match &body.node {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 1);
                assert!(is_dollar(&args[0], "$"));
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn mixed_dollar_and_numbered() {
        let expr = parse_expr("$($ + $2)");
        let body = assert_abbreviated(&expr);
        let (l, _, r) = is_binop(body).expect("binop body");
        assert!(is_dollar(l, "$"));
        assert!(is_dollar(r, "$2"));
    }

    #[test]
    fn three_params() {
        // Left-associative: ($1 + $2) + $3.
        let expr = parse_expr("$($1 + $2 + $3)");
        let body = assert_abbreviated(&expr);
        let (l, _, r) = is_binop(body).expect("binop body");
        assert!(is_dollar(r, "$3"));
        let (ll, _, lr) = is_binop(l).expect("nested binop");
        assert!(is_dollar(ll, "$1"));
        assert!(is_dollar(lr, "$2"));
    }

    // A nested abbreviated lambda is its own scope: the inner `$($1)` parses to
    // its own node and does not consume the outer `$1` that follows it. The
    // depth counter must keep the outer `$1` legal after the inner one closes.
    #[test]
    fn nested_abbreviated() {
        let expr = parse_expr("$($($1) + $1)");
        let body = assert_abbreviated(&expr);
        let (l, op, r) = is_binop(body).expect("binop body");
        assert!(matches!(op, BinOp::Add));
        let inner = assert_abbreviated(l);
        assert!(is_dollar(inner, "$1"));
        assert!(is_dollar(r, "$1"));
    }

    // Placeholders are `$`, `$1`-`$9`, and `$$`. `$0` is not one of them, so it
    // is not a valid placeholder name in any position.
    #[test]
    fn dollar_zero_is_rejected() {
        let err = parse_err("$($0)");
        assert!(
            err.contains("Expected") || err.contains("unexpected"),
            "error was: {err}"
        );
        let err = parse_err("$0");
        assert!(err.contains("unexpected"), "error was: {err}");
    }

    // Placeholder indices are a single digit; `$10` is not `$1` followed by `0`.
    #[test]
    fn two_digit_placeholder_is_rejected() {
        let err = parse_err("$($10)");
        assert!(
            err.contains("unexpected") || err.contains("Expected"),
            "error was: {err}"
        );
    }

    // An abbreviated lambda must wrap an expression; `$()` is empty.
    #[test]
    fn empty_abbreviated_is_rejected() {
        assert_eq!(
            parse_err_message("$()"),
            "expected an expression, but found `)`"
        );
    }

    // -- Usage summaries --
    // The wrapper reports which positional params the body references (length =
    // highest referenced) and whether `$$` appears, so the compiler can write
    // the lambda prelude and drop unused params without inspecting the body.

    #[test]
    fn summary_single_dollar_is_param_one() {
        let expr = parse_expr("$($ * 2)");
        assert_eq!(usage(&expr), (&[true][..], false));
    }

    #[test]
    fn summary_skipped_param_is_recorded_unused() {
        let expr = parse_expr("$($2)");
        assert_eq!(usage(&expr), (&[false, true][..], false));
    }

    #[test]
    fn summary_all_params_used() {
        let expr = parse_expr("$($1 + $2)");
        assert_eq!(usage(&expr), (&[true, true][..], false));
    }

    #[test]
    fn summary_highest_param_sets_the_length() {
        let expr = parse_expr("$($9)");
        let (used, rest) = usage(&expr);
        assert_eq!(used.len(), 9);
        assert!(used[8]);
        assert!(!rest);
    }

    #[test]
    fn summary_rest_only() {
        let expr = parse_expr("$($$)");
        assert_eq!(usage(&expr), (&[][..], true));
    }

    #[test]
    fn summary_positional_plus_rest() {
        let expr = parse_expr("$(f($1, $$))");
        assert_eq!(usage(&expr), (&[true][..], true));
    }

    #[test]
    fn summary_dollar_free_thunk() {
        // The uber-terse zero-arg thunk: intended behavior, not an accident.
        let expr = parse_expr("$(42)");
        assert_eq!(usage(&expr), (&[][..], false));
    }

    #[test]
    fn summary_dollar_aliases_dollar_one() {
        // `$` and `$1` mark the same parameter; the body keeps each spelling
        // verbatim (no parse-time normalization).
        let expr = parse_expr("$($ + $1)");
        assert_eq!(usage(&expr), (&[true][..], false));
        let body = assert_abbreviated(&expr);
        let (l, _, r) = is_binop(body).expect("binop body");
        assert!(is_dollar(l, "$"));
        assert!(is_dollar(r, "$1"));
    }

    #[test]
    fn summary_nested_attribution() {
        // A dollar identifier belongs to the innermost abbreviated lambda:
        // `$2` inside the nested lambda marks the inner summary, not the outer.
        let expr = parse_expr("$(g($1, $($2)))");
        assert_eq!(usage(&expr), (&[true][..], false));
        let body = assert_abbreviated(&expr);
        let Expr::Call { args, .. } = &body.node else {
            panic!("expected Call body, got {:?}", body.node)
        };
        assert_eq!(usage(&args[1]), (&[false, true][..], false));
    }

    // -- Dollar identifiers inside format-string interpolations --
    // An interpolation is lexed separately from the enclosing source, but
    // lexically it still sits inside the abbreviated lambda: `$n` is legal there
    // and counts toward the lambda's parameters.

    /// The interpolated expressions of a format-string body, in order.
    fn interpolations(body: &Spanned<Expr>) -> Vec<&Spanned<Expr>> {
        let Expr::FormatString(segments) = &body.node else {
            panic!("expected FormatString body, got {:?}", body.node)
        };
        segments
            .iter()
            .filter_map(|s| match s {
                FormatSegment::Interpolation(e) => Some(e),
                FormatSegment::Literal(_) => None,
            })
            .collect()
    }

    #[test]
    fn interpolation_dollar_is_accepted_and_counted() {
        let expr = parse_expr("$($'v=${$1}')");
        assert_eq!(usage(&expr), (&[true][..], false));
        let body = assert_abbreviated(&expr);
        assert!(is_dollar(interpolations(body)[0], "$1"));
    }

    #[test]
    fn interpolation_records_a_skipped_param() {
        let expr = parse_expr("$($'v=${$2}')");
        assert_eq!(usage(&expr), (&[false, true][..], false));
    }

    #[test]
    fn interpolation_marks_merge_with_the_outer_body() {
        // The marks made inside the interpolation must survive back into the
        // enclosing frame; losing them would under-report `used_params` and the
        // compiler would drop a parameter the body actually reads.
        let expr = parse_expr("$($'${$1}' + $2)");
        assert_eq!(usage(&expr), (&[true, true][..], false));
    }

    #[test]
    fn interpolation_records_bare_dollar_and_rest() {
        assert_eq!(usage(&parse_expr("$($'${$}')")), (&[true][..], false));
        assert_eq!(usage(&parse_expr("$($'${$$}')")), (&[][..], true));
    }

    #[test]
    fn interpolation_dollar_outside_a_lambda_is_rejected() {
        // The gate stays closed: no enclosing abbreviated lambda, no `$n`.
        parse_err("$'v=${$1}'");
    }

    #[test]
    fn nested_lambda_in_interpolation_attributes_to_the_inner_lambda() {
        // `$1` belongs to the innermost lambda, even across the interpolation
        // boundary: the outer lambda takes no parameters.
        let expr = parse_expr("$($'${$($1)}')");
        assert_eq!(usage(&expr), (&[][..], false));
        let inner = interpolations(assert_abbreviated(&expr))[0];
        assert_eq!(usage(inner), (&[true][..], false));
    }
}
