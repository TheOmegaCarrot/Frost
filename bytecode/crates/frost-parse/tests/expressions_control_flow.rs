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
