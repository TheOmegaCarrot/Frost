mod helpers;

use frostlang_parse::ast::*;
use frostlang_parse::parse_program;
use helpers::*;

fn assert_if(expr: &Spanned<Expr>) -> (&Spanned<Expr>, &Spanned<Expr>, Option<&Spanned<Expr>>) {
    match &expr.node {
        Expr::If {
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
        assert!(matches!(&cond.node, Expr::Literal(Literal::Bool(true))));
        assert!(is_int(then, 1));
        assert!(alt.is_none());
    }

    #[test]
    fn if_then_else() {
        let expr = parse_expr("if true: 1 else: 2");
        let (cond, then, alt) = assert_if(&expr);
        assert!(matches!(&cond.node, Expr::Literal(Literal::Bool(true))));
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
        assert!(matches!(&cond2.node, Expr::Literal(Literal::Bool(false))));
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
        assert!(matches!(&then.node, Expr::Call { .. }));
        assert!(matches!(&alt.unwrap().node, Expr::Call { .. }));
    }

    #[test]
    fn if_as_value_in_def() {
        let program = parse("def x = if true: 1 else: 2");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].node {
            Statement::Def { expr, .. } => {
                assert!(matches!(&expr.node, Expr::If { .. }));
            }
            other => panic!("expected Def, got {other:?}"),
        }
    }

    #[test]
    fn if_in_call_arg() {
        let expr = parse_expr("f(if true: 1 else: 2)");
        match &expr.node {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0].node, Expr::If { .. }));
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn nested_if() {
        let expr = parse_expr("if true: if false: 1 else: 2 else: 3");
        let (_, then, alt) = assert_if(&expr);
        assert!(matches!(&then.node, Expr::If { .. }));
        assert!(is_int(alt.unwrap(), 3));
    }
}

mod if_newlines {
    use super::*;

    #[test]
    fn newline_before_else() {
        let source = r"
            if true: 1
            else: 2
        ";
        let expr = parse_expr(source);
        let (_, _, alt) = assert_if(&expr);
        assert!(is_int(alt.unwrap(), 2));
    }

    #[test]
    fn newline_before_elif() {
        let source = r"
            if true: 1
            elif false: 2
            else: 3
        ";
        let expr = parse_expr(source);
        let (_, _, alt) = assert_if(&expr);
        let (_, _, alt2) = assert_if(alt.unwrap());
        assert!(is_int(alt2.unwrap(), 3));
    }

    #[test]
    fn multiple_newlines_before_else() {
        let source = r"
            if true: 1


            else: 2
        ";
        let expr = parse_expr(source);
        let (_, _, alt) = assert_if(&expr);
        assert!(is_int(alt.unwrap(), 2));
    }

    #[test]
    fn all_clauses_on_separate_lines() {
        let source = r"
            if a: 1
            elif b: 2
            elif c: 3
            else: 4
        ";
        let expr = parse_expr(source);
        let (_, _, alt1) = assert_if(&expr);
        let (_, _, alt2) = assert_if(alt1.unwrap());
        let (_, _, alt3) = assert_if(alt2.unwrap());
        assert!(is_int(alt3.unwrap(), 4));
    }

    #[test]
    fn if_in_parens_with_newlines() {
        let source = r"
            (
                if true:
                    1
                else:
                    2
            )
        ";
        let expr = parse_expr(source);
        let (_, then, alt) = assert_if(&expr);
        assert!(is_int(then, 1));
        assert!(is_int(alt.unwrap(), 2));
    }

    #[test]
    fn each_branch_may_start_on_the_line_after_its_colon() {
        let source = r"
            if a:
                1
            elif b:
                2
            else:
                3
        ";
        let expr = parse_expr(source);
        let (_, then, alt) = assert_if(&expr);
        assert!(is_int(then, 1));
        let (_, then2, alt2) = assert_if(alt.unwrap());
        assert!(is_int(then2, 2));
        assert!(is_int(alt2.unwrap(), 3));
    }

    #[test]
    fn blank_lines_and_a_comment_may_follow_a_colon() {
        let source = r"
            if a: # a note

                1
            else: # another

                2
        ";
        let expr = parse_expr(source);
        let (_, then, alt) = assert_if(&expr);
        assert!(is_int(then, 1));
        assert!(is_int(alt.unwrap(), 2));
    }

    #[test]
    fn a_branch_on_its_own_line_still_ends_at_its_newline() {
        let source = r"
            def v = if a:
                1
            else:
                2
            v
        ";
        let program = parse(source);
        assert_eq!(program.statements.len(), 2, "{program:?}");
    }

    #[test]
    fn if_without_else_then_newline_statement() {
        let source = r"
            if true: 1
            2
        ";
        let program = parse_program("test.frst", source).expect("failed to parse");
        assert_eq!(program.statements.len(), 2);
    }
}

mod if_errors {
    use super::*;

    #[test]
    fn missing_colon_after_condition() {
        assert_eq!(
            parse_err_message("if true 1"),
            "expected `:`, but found `1`"
        );
    }

    #[test]
    fn missing_consequent() {
        assert_eq!(
            parse_err_message("if true:"),
            "expected an expression, but found the end of input"
        );
    }

    #[test]
    fn missing_colon_after_else() {
        assert_eq!(
            parse_err_message("if true: 1 else 2"),
            "expected `:`, but found `2`"
        );
    }

    #[test]
    fn missing_alternate_after_else_colon() {
        assert_eq!(
            parse_err_message("if true: 1 else:"),
            "expected an expression, but found the end of input"
        );
    }

    #[test]
    fn missing_colon_after_elif_condition() {
        assert_eq!(
            parse_err_message("if true: 1 elif false 2"),
            "expected `:`, but found `2`"
        );
    }
}

// ============================================================
// Do blocks
// ============================================================

fn assert_do(expr: &Spanned<Expr>) -> (&[Spanned<Statement>], &Spanned<Expr>) {
    match &expr.node {
        Expr::Do { body, value } => (body, value),
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
        assert!(matches!(&body[0].node, Statement::Def { .. }));
        assert!(matches!(&value.node, Expr::NameLookup(n) if n == "x"));
    }

    #[test]
    fn nested_do() {
        // A `do` block can be the tail value of another `do` block.
        let expr = parse_expr("do { do { x } }");
        let (body, value) = assert_do(&expr);
        assert!(body.is_empty());
        let (inner_body, inner_value) = assert_do(value);
        assert!(inner_body.is_empty());
        assert!(matches!(&inner_value.node, Expr::NameLookup(n) if n == "x"));
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
        assert!(matches!(&body[0].node, Statement::Expr(_)));
        assert!(matches!(&body[1].node, Statement::Expr(_)));
        assert!(is_int(value, 42));
    }
}

mod do_newlines {
    use super::*;

    #[test]
    fn multiline() {
        let source = r"
            do {
                def x = 5
                x
            }
        ";
        let expr = parse_expr(source);
        let (body, value) = assert_do(&expr);
        assert_eq!(body.len(), 1);
        assert!(matches!(&value.node, Expr::NameLookup(n) if n == "x"));
    }

    #[test]
    fn multiline_multiple_defs() {
        let source = r"
            do {
                def a = 1
                def b = 2
                a + b
            }
        ";
        let expr = parse_expr(source);
        let (body, _) = assert_do(&expr);
        assert_eq!(body.len(), 2);
    }

    #[test]
    fn blank_lines() {
        let source = r"
            do {
                def x = 1

                x
            }
        ";
        let expr = parse_expr(source);
        let (body, _) = assert_do(&expr);
        assert_eq!(body.len(), 1);
    }

    #[test]
    fn semicolons_and_newlines_mixed() {
        let source = r"
            do {
                def x = 1; def y = 2
                x + y
            }
        ";
        let expr = parse_expr(source);
        let (body, _) = assert_do(&expr);
        assert_eq!(body.len(), 2);
    }
}

/// Inside `()`, `[]`, or a Map literal's `{}`, newlines are insignificant, but a
/// block nested there is a scope of its own, where they end statements again.
mod blocks_inside_delimiters {
    use super::*;

    /// The one argument of the call `expr` must be.
    fn only_argument(expr: &Spanned<Expr>) -> &Spanned<Expr> {
        match &expr.node {
            Expr::Call { args, .. } if args.len() == 1 => &args[0],
            other => panic!("expected a call with one argument, got {other:?}"),
        }
    }

    #[test]
    fn a_multiline_do_block_in_a_call() {
        let source = r"
            f(do {
                def x = 1
                def y = 2
                x + y
            })
        ";
        let expr = parse_expr(source);
        let (body, value) = assert_do(only_argument(&expr));
        assert_eq!(body.len(), 2);
        assert!(is_binop(value).is_some());
    }

    #[test]
    fn a_multiline_do_block_in_an_array_or_map() {
        for source in [
            r"
            [do {
                def x = 1
                x
            }]
            ",
            r"
            {a: do {
                def x = 1
                x
            }}
            ",
            r"
            f(g(do {
                def x = 1
                x
            }))
            ",
        ] {
            let program = parse(source);
            assert_eq!(program.statements.len(), 1, "{source:?}");
        }
    }

    #[test]
    fn a_multiline_lambda_body_in_a_call() {
        let source = r"
            f(fn x -> {
                def y = x + 1
                y * 2
            })
        ";
        let expr = parse_expr(source);
        assert!(
            matches!(&only_argument(&expr).node, Expr::Lambda { .. }),
            "{expr:?}"
        );
    }

    #[test]
    fn newlines_around_the_block_are_still_insignificant() {
        let source = r"
            f(
                do {
                    def x = 1
                    x
                }
                ,
                2
            )
        ";
        let expr = parse_expr(source);
        match &expr.node {
            Expr::Call { args, .. } => assert_eq!(args.len(), 2),
            other => panic!("expected a call, got {other:?}"),
        }
    }

    #[test]
    fn a_line_break_mid_expression_in_the_block_is_still_an_error() {
        // As it is for a block anywhere else: the line ends the statement.
        let source = r"
            f(do {
                x +
                y
            })
        ";
        parse_err(source);
    }
}

mod do_in_expressions {
    use super::*;

    #[test]
    fn do_in_def() {
        let program = parse("def x = do { 42 }");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].node {
            Statement::Def { expr, .. } => {
                assert!(matches!(&expr.node, Expr::Do { .. }));
            }
            other => panic!("expected Def, got {other:?}"),
        }
    }

    #[test]
    fn do_in_call() {
        let expr = parse_expr("f(do { 42 })");
        assert!(matches!(&expr.node, Expr::Call { .. }));
    }

    #[test]
    fn do_in_if_branch() {
        let source = r"
            if true: do {
                def x = 1
                x
            }
            else: 0
        ";
        let expr = parse_expr(source);
        let (_, then, _) = assert_if(&expr);
        assert!(matches!(&then.node, Expr::Do { .. }));
    }

    #[test]
    fn do_in_arithmetic() {
        let expr = parse_expr("do { 1 } + do { 2 }");
        let (left, op, right) = is_binop(&expr).unwrap();
        assert!(matches!(op, BinOp::Add));
        assert!(matches!(&left.node, Expr::Do { .. }));
        assert!(matches!(&right.node, Expr::Do { .. }));
    }

    #[test]
    fn nested_do() {
        let source = r"
            do {
                def x = do { 5 }
                x
            }
        ";
        let expr = parse_expr(source);
        let (body, _) = assert_do(&expr);
        assert_eq!(body.len(), 1);
        match &body[0].node {
            Statement::Def { expr, .. } => {
                assert!(matches!(&expr.node, Expr::Do { .. }));
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
        let source = r"
            do {
                export def x = 5
                x
            }
        ";
        assert_eq!(
            parse_err_message(source),
            "expected an expression, but found `export`"
        );
    }

    #[test]
    fn missing_closing_brace() {
        assert_eq!(
            parse_err_message("do { 42"),
            "expected `}`, but found the end of input"
        );
    }

    #[test]
    fn missing_opening_brace() {
        assert_eq!(parse_err_message("do 42"), "expected `{`, but found `42`");
    }

    // Block content must follow the same grammar as the top level (minus
    // `export`): adjacent expressions require a separator.
    #[test]
    fn missing_separator_between_exprs() {
        assert_eq!(
            parse_err_message("do { 1 2 }"),
            "expected a line break or `;`, but found `2`"
        );
    }

    // A trailing binary operator must not silently continue onto the next line
    // inside a block: newlines are significant at the top level and blocks
    // mirror that.
    #[test]
    fn operator_continuation_across_newline() {
        let source = r"
            do {
                x +
                y
            }
        ";
        assert_eq!(
            parse_err_message(source),
            "expected an expression, but found a line break"
        );
    }
}
