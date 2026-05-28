mod helpers;

use frost_parse::ast::*;
use frost_parse::parse_program;
use helpers::*;

fn assert_if(expr: &Expr) -> (&Expr, &Expr, Option<&Expr>) {
    match &expr.kind {
        ExprKind::If {
            condition,
            consequent,
            alternate,
        } => (condition, consequent, alternate.as_deref()),
        other => panic!("expected If, got {other:?}"),
    }
}

mod if_basic {
    use super::*;

    #[test]
    fn if_then() {
        let expr = parse_expr("if true: 1");
        let (cond, then, alt) = assert_if(&expr);
        assert!(matches!(&cond.kind, ExprKind::Literal(Literal::Bool(true))));
        assert!(is_int(then, 1));
        assert!(alt.is_none());
    }

    #[test]
    fn if_then_else() {
        let expr = parse_expr("if true: 1 else: 2");
        let (cond, then, alt) = assert_if(&expr);
        assert!(matches!(&cond.kind, ExprKind::Literal(Literal::Bool(true))));
        assert!(is_int(then, 1));
        assert!(is_int(alt.unwrap(), 2));
    }

    #[test]
    fn if_elif_else() {
        let expr = parse_expr("if true: 1 elif false: 2 else: 3");
        let (_, then, alt) = assert_if(&expr);
        assert!(is_int(then, 1));
        let alt = alt.unwrap();
        let (cond2, then2, alt2) = assert_if(alt);
        assert!(matches!(
            &cond2.kind,
            ExprKind::Literal(Literal::Bool(false))
        ));
        assert!(is_int(then2, 2));
        assert!(is_int(alt2.unwrap(), 3));
    }

    #[test]
    fn multiple_elif() {
        let expr = parse_expr("if a: 1 elif b: 2 elif c: 3 else: 4");
        let (_, _, alt1) = assert_if(&expr);
        let (_, _, alt2) = assert_if(alt1.unwrap());
        let (_, then3, alt3) = assert_if(alt2.unwrap());
        assert!(is_int(then3, 3));
        assert!(is_int(alt3.unwrap(), 4));
    }

    #[test]
    fn if_without_else_is_none() {
        let expr = parse_expr("if x: 42");
        let (_, _, alt) = assert_if(&expr);
        assert!(alt.is_none());
    }

    #[test]
    fn elif_without_else() {
        let expr = parse_expr("if a: 1 elif b: 2");
        let (_, _, alt) = assert_if(&expr);
        let (_, _, alt2) = assert_if(alt.unwrap());
        assert!(alt2.is_none());
    }
}

mod if_expression_branches {
    use super::*;

    #[test]
    fn arithmetic_condition() {
        let expr = parse_expr("if x > 0: 1 else: 2");
        let (cond, _, _) = assert_if(&expr);
        assert!(is_binop(cond).is_some());
    }

    #[test]
    fn arithmetic_consequent() {
        let expr = parse_expr("if true: 1 + 2");
        let (_, then, _) = assert_if(&expr);
        assert!(is_binop(then).is_some());
    }

    #[test]
    fn call_in_branch() {
        let expr = parse_expr("if true: f(1) else: g(2)");
        let (_, then, alt) = assert_if(&expr);
        assert!(matches!(&then.kind, ExprKind::Call { .. }));
        assert!(matches!(&alt.unwrap().kind, ExprKind::Call { .. }));
    }

    #[test]
    fn if_as_value_in_def() {
        let program = parse("def x = if true: 1 else: 2");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StatementKind::Def { expr, .. } => {
                assert!(matches!(&expr.kind, ExprKind::If { .. }));
            }
            other => panic!("expected Def, got {other:?}"),
        }
    }

    #[test]
    fn if_in_call_arg() {
        let expr = parse_expr("f(if true: 1 else: 2)");
        match &expr.kind {
            ExprKind::Call { args, .. } => {
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0].kind, ExprKind::If { .. }));
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn nested_if() {
        let expr = parse_expr("if true: if false: 1 else: 2 else: 3");
        let (_, then, alt) = assert_if(&expr);
        assert!(matches!(&then.kind, ExprKind::If { .. }));
        assert!(is_int(alt.unwrap(), 3));
    }
}

mod if_newlines {
    use super::*;

    #[test]
    fn newline_before_else() {
        let expr = parse_expr("if true: 1\nelse: 2");
        let (_, _, alt) = assert_if(&expr);
        assert!(is_int(alt.unwrap(), 2));
    }

    #[test]
    fn newline_before_elif() {
        let expr = parse_expr("if true: 1\nelif false: 2\nelse: 3");
        let (_, _, alt) = assert_if(&expr);
        let (_, _, alt2) = assert_if(alt.unwrap());
        assert!(is_int(alt2.unwrap(), 3));
    }

    #[test]
    fn multiple_newlines_before_else() {
        let expr = parse_expr("if true: 1\n\n\nelse: 2");
        let (_, _, alt) = assert_if(&expr);
        assert!(is_int(alt.unwrap(), 2));
    }

    #[test]
    fn all_clauses_on_separate_lines() {
        let expr = parse_expr("if a: 1\nelif b: 2\nelif c: 3\nelse: 4");
        let (_, _, alt1) = assert_if(&expr);
        let (_, _, alt2) = assert_if(alt1.unwrap());
        let (_, _, alt3) = assert_if(alt2.unwrap());
        assert!(is_int(alt3.unwrap(), 4));
    }

    #[test]
    fn if_in_parens_with_newlines() {
        let expr = parse_expr("(\nif true:\n1\nelse:\n2\n)");
        let (_, then, alt) = assert_if(&expr);
        assert!(is_int(then, 1));
        assert!(is_int(alt.unwrap(), 2));
    }

    #[test]
    fn if_without_else_then_newline_statement() {
        let program = parse_program("test.frst", "if true: 1\n2").expect("failed to parse");
        assert_eq!(program.statements.len(), 2);
    }
}

mod if_errors {
    use super::*;

    #[test]
    fn missing_colon_after_condition() {
        let err = parse_err("if true 1");
        assert!(
            err.contains("Expected :") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    #[test]
    fn missing_consequent() {
        let err = parse_err("if true:");
        assert!(
            err.contains("end of input") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    #[test]
    fn missing_colon_after_else() {
        let err = parse_err("if true: 1 else 2");
        assert!(
            err.contains("Expected :") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    #[test]
    fn missing_alternate_after_else_colon() {
        let err = parse_err("if true: 1 else:");
        assert!(
            err.contains("end of input") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    #[test]
    fn missing_colon_after_elif_condition() {
        let err = parse_err("if true: 1 elif false 2");
        assert!(
            err.contains("Expected :") || err.contains("unexpected"),
            "error was: {err}"
        );
    }
}

// ============================================================
// Do blocks
// ============================================================

fn assert_do(expr: &Expr) -> (&[Statement], &Expr) {
    match &expr.kind {
        ExprKind::Do { body, value } => (body, value),
        other => panic!("expected Do, got {other:?}"),
    }
}

mod do_basic {
    use super::*;

    #[test]
    fn single_expression() {
        let expr = parse_expr("do { 42 }");
        let (body, value) = assert_do(&expr);
        assert!(body.is_empty());
        assert!(is_int(value, 42));
    }

    #[test]
    fn def_then_expression() {
        let expr = parse_expr("do { def x = 5; x }");
        let (body, value) = assert_do(&expr);
        assert_eq!(body.len(), 1);
        assert!(matches!(&body[0].kind, StatementKind::Def { .. }));
        assert!(matches!(&value.kind, ExprKind::NameLookup(n) if n == "x"));
    }

    #[test]
    fn multiple_defs() {
        let expr = parse_expr("do { def x = 1; def y = 2; x + y }");
        let (body, value) = assert_do(&expr);
        assert_eq!(body.len(), 2);
        assert!(is_binop(value).is_some());
    }

    #[test]
    fn expression_value_with_arithmetic() {
        let expr = parse_expr("do { def x = 5; x + 1 }");
        let (body, value) = assert_do(&expr);
        assert_eq!(body.len(), 1);
        assert!(is_binop(value).is_some());
    }

    #[test]
    fn side_effect_expressions_in_body() {
        let expr = parse_expr("do { f(1); g(2); 42 }");
        let (body, value) = assert_do(&expr);
        assert_eq!(body.len(), 2);
        assert!(matches!(&body[0].kind, StatementKind::Expr(_)));
        assert!(matches!(&body[1].kind, StatementKind::Expr(_)));
        assert!(is_int(value, 42));
    }
}

mod do_newlines {
    use super::*;

    #[test]
    fn multiline() {
        let expr = parse_expr("do {\n    def x = 5\n    x\n}");
        let (body, value) = assert_do(&expr);
        assert_eq!(body.len(), 1);
        assert!(matches!(&value.kind, ExprKind::NameLookup(n) if n == "x"));
    }

    #[test]
    fn multiline_multiple_defs() {
        let expr = parse_expr("do {\n    def a = 1\n    def b = 2\n    a + b\n}");
        let (body, _) = assert_do(&expr);
        assert_eq!(body.len(), 2);
    }

    #[test]
    fn blank_lines() {
        let expr = parse_expr("do {\n    def x = 1\n\n    x\n}");
        let (body, _) = assert_do(&expr);
        assert_eq!(body.len(), 1);
    }

    #[test]
    fn semicolons_and_newlines_mixed() {
        let expr = parse_expr("do {\n    def x = 1; def y = 2\n    x + y\n}");
        let (body, _) = assert_do(&expr);
        assert_eq!(body.len(), 2);
    }
}

mod do_in_expressions {
    use super::*;

    #[test]
    fn do_in_def() {
        let program = parse("def x = do { 42 }");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StatementKind::Def { expr, .. } => {
                assert!(matches!(&expr.kind, ExprKind::Do { .. }));
            }
            other => panic!("expected Def, got {other:?}"),
        }
    }

    #[test]
    fn do_in_call() {
        let expr = parse_expr("f(do { 42 })");
        assert!(matches!(&expr.kind, ExprKind::Call { .. }));
    }

    #[test]
    fn do_in_if_branch() {
        let expr = parse_expr("if true: do { def x = 1; x } else: 0");
        let (_, then, _) = assert_if(&expr);
        assert!(matches!(&then.kind, ExprKind::Do { .. }));
    }

    #[test]
    fn do_in_arithmetic() {
        let expr = parse_expr("do { 1 } + do { 2 }");
        let (left, op, right) = is_binop(&expr).unwrap();
        assert!(matches!(op, BinOp::Add));
        assert!(matches!(&left.kind, ExprKind::Do { .. }));
        assert!(matches!(&right.kind, ExprKind::Do { .. }));
    }

    #[test]
    fn nested_do() {
        let expr = parse_expr("do { def x = do { 5 }; x }");
        let (body, _) = assert_do(&expr);
        assert_eq!(body.len(), 1);
        match &body[0].kind {
            StatementKind::Def { expr, .. } => {
                assert!(matches!(&expr.kind, ExprKind::Do { .. }));
            }
            other => panic!("expected Def, got {other:?}"),
        }
    }
}

mod do_errors {
    use super::*;

    #[test]
    fn empty_do_block() {
        let err = parse_err("do {}");
        assert!(err.contains("at least one expression"), "error was: {err}");
    }

    #[test]
    fn do_ending_with_def() {
        let err = parse_err("do { def x = 5 }");
        assert!(err.contains("end with an expression"), "error was: {err}");
    }

    #[test]
    fn export_in_do() {
        let err = parse_err("do { export def x = 5; x }");
        assert!(err.contains("unexpected"), "error was: {err}");
    }

    #[test]
    fn missing_closing_brace() {
        let err = parse_err("do { 42");
        assert!(
            err.contains("end of input") || err.contains("Expected }"),
            "error was: {err}"
        );
    }

    #[test]
    fn missing_opening_brace() {
        let err = parse_err("do 42");
        assert!(
            err.contains("Expected {") || err.contains("unexpected"),
            "error was: {err}"
        );
    }
}
