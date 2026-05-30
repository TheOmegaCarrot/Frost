mod helpers;

use frost_parse::ast::*;
use helpers::*;

fn assert_lambda(expr: &Expr) -> LambdaParts<'_> {
    match &expr.kind {
        ExprKind::Lambda {
            params,
            variadic_param,
            self_name,
            body,
            return_expr,
        } => LambdaParts {
            params,
            variadic_param: variadic_param.as_ref(),
            self_name: self_name.as_deref(),
            body,
            return_expr,
        },
        other => panic!("expected Lambda, got {other:?}"),
    }
}

struct LambdaParts<'a> {
    params: &'a [Binding],
    variadic_param: Option<&'a Binding>,
    self_name: Option<&'a str>,
    body: &'a [Statement],
    return_expr: &'a Expr,
}

fn is_named(binding: &Binding, expected: &str) -> bool {
    matches!(binding, Binding::Named(n) if n == expected)
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
        assert_eq!(lam.params[0], Binding::Discarded);
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
        assert_eq!(lam.params[0], Binding::Discarded);
        assert!(is_named(&lam.params[1], "y"));
    }

    #[test]
    fn multiline() {
        let expr = parse_expr("fn(\n    x,\n    y\n) -> x + y");
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
        assert!(matches!(&lam.return_expr.kind, ExprKind::If { .. }));
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
        assert!(matches!(&lam.body[0].kind, StatementKind::Def { .. }));
        assert!(matches!(&lam.return_expr.kind, ExprKind::NameLookup(n) if n == "y"));
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
        let expr = parse_expr("fn x -> {\n    def a = 1\n    def b = 2\n    a + b + x\n}");
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
        assert!(matches!(&lam.return_expr.kind, ExprKind::Map(_)));
    }

    #[test]
    fn map_with_param() {
        let expr = parse_expr("fn x -> {name: x}");
        let lam = assert_lambda(&expr);
        assert!(lam.body.is_empty());
        assert!(matches!(&lam.return_expr.kind, ExprKind::Map(_)));
    }

    #[test]
    fn map_with_multiple_entries() {
        let expr = parse_expr("fn x -> {name: x, age: 30}");
        let lam = assert_lambda(&expr);
        assert!(matches!(&lam.return_expr.kind, ExprKind::Map(entries) if entries.len() == 2));
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
        match &program.statements[0].kind {
            StatementKind::Def { expr, .. } => {
                assert_lambda(expr);
            }
            other => panic!("expected Def, got {other:?}"),
        }
    }

    #[test]
    fn in_call_position() {
        let expr = parse_expr("(fn a -> a)(42)");
        match &expr.kind {
            ExprKind::Call { callee, args } => {
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
        match &expr.kind {
            ExprKind::Call { args, .. } => {
                assert_eq!(args.len(), 1);
                assert_lambda(&args[0]);
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn in_array() {
        let expr = parse_expr("[fn x -> x, fn y -> y]");
        match &expr.kind {
            ExprKind::Array(elems) => {
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
        let err = parse_err("fn x x");
        assert!(
            err.contains("Expected ->") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    #[test]
    fn missing_body() {
        let err = parse_err("fn x ->");
        assert!(
            err.contains("end of input") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    #[test]
    fn empty_block_body() {
        let err = parse_err("fn -> {}");
        assert!(err.contains("at least one expression"), "error was: {err}");
    }

    #[test]
    fn block_body_ending_with_def() {
        let err = parse_err("fn -> { def x = 1 }");
        assert!(err.contains("end with an expression"), "error was: {err}");
    }

    #[test]
    fn unclosed_parens() {
        let err = parse_err("fn(x, y -> x");
        assert!(
            err.contains("Expected )") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    #[test]
    fn number_as_param() {
        let err = parse_err("fn 42 -> 42");
        assert!(err.contains("unexpected"), "error was: {err}");
    }

    #[test]
    fn dollar_id_outside_abbreviated_lambda() {
        let err = parse_err("def a = $($ + 2) ; def $3 = 2");
        assert!(err.contains("unexpected"), "error was: {err}");
    }
}

// ============================================================
// Abbreviated lambdas
// ============================================================

mod abbreviated {
    use super::*;

    #[test]
    fn single_dollar() {
        let expr = parse_expr("$($ * 2)");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 1);
        assert!(is_named(&lam.params[0], "$1"));
        assert!(lam.variadic_param.is_none());
    }

    #[test]
    fn numbered_params() {
        let expr = parse_expr("$($1 + $2)");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 2);
        assert!(is_named(&lam.params[0], "$1"));
        assert!(is_named(&lam.params[1], "$2"));
    }

    #[test]
    fn rest_only() {
        let expr = parse_expr("$($$)");
        let lam = assert_lambda(&expr);
        assert!(lam.params.is_empty());
        assert!(is_named(lam.variadic_param.expect("variadic"), "$$"));
    }

    #[test]
    fn numbered_with_rest() {
        let expr = parse_expr("$($ + $$)");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 1);
        assert!(is_named(lam.variadic_param.expect("variadic"), "$$"));
    }

    #[test]
    fn gap_filling() {
        let expr = parse_expr("$($9)");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 9);
        assert!(is_named(&lam.params[8], "$9"));
    }

    #[test]
    fn no_dollar_ids() {
        let expr = parse_expr("$(42)");
        let lam = assert_lambda(&expr);
        assert!(lam.params.is_empty());
        assert!(lam.variadic_param.is_none());
    }

    #[test]
    fn dollar_in_call() {
        let expr = parse_expr("$(f($))");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 1);
        assert!(matches!(&lam.return_expr.kind, ExprKind::Call { .. }));
    }

    #[test]
    fn mixed_dollar_and_numbered() {
        let expr = parse_expr("$($ + $2)");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 2);
    }

    #[test]
    fn three_params() {
        let expr = parse_expr("$($1 + $2 + $3)");
        let lam = assert_lambda(&expr);
        assert_eq!(lam.params.len(), 3);
    }
}
