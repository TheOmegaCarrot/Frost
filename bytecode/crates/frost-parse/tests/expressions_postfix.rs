mod helpers;

use frost_parse::ast::*;
use frost_parse::parse_program;
use helpers::*;

// -- Function calls --

#[test]
fn call_no_args() {
    let expr = parse_expr("f()");
    match &expr.node {
        Expr::Call { callee, args } => {
            assert!(matches!(&callee.node, Expr::NameLookup(n) if n == "f"));
            assert!(args.is_empty());
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_one_arg() {
    let expr = parse_expr("f(1)");
    match &expr.node {
        Expr::Call { callee, args } => {
            assert!(matches!(&callee.node, Expr::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 1);
            assert!(is_int(&args[0], 1));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_multiple_args() {
    let expr = parse_expr("f(1, 2, 3)");
    match &expr.node {
        Expr::Call { callee, args } => {
            assert!(matches!(&callee.node, Expr::NameLookup(n) if n == "f"));
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
    match &expr.node {
        Expr::Call { callee, args } => {
            assert!(matches!(&callee.node, Expr::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 2);
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_with_expression_args() {
    let expr = parse_expr("f(1 + 2, 3 * 4)");
    match &expr.node {
        Expr::Call { args, .. } => {
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
    match &expr.node {
        Expr::Call { args, .. } => {
            assert_eq!(args.len(), 2);
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn chained_calls() {
    // f(1)(2) == Call(Call(f, [1]), [2])
    let expr = parse_expr("f(1)(2)");
    match &expr.node {
        Expr::Call { callee, args } => {
            assert_eq!(args.len(), 1);
            assert!(is_int(&args[0], 2));
            match &callee.node {
                Expr::Call {
                    callee: inner,
                    args: inner_args,
                } => {
                    assert!(matches!(&inner.node, Expr::NameLookup(n) if n == "f"));
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
    let program = parse_program("test.frst", "f\n(1)").expect("failed to parse");
    assert_eq!(program.statements.len(), 2);
}

// -- Indexing --

#[test]
fn index_literal() {
    let expr = parse_expr("a[0]");
    match &expr.node {
        Expr::SoftIndex { target, key } => {
            assert!(matches!(&target.node, Expr::NameLookup(n) if n == "a"));
            assert!(is_int(key, 0));
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn index_expression() {
    let expr = parse_expr("a[1 + 2]");
    match &expr.node {
        Expr::SoftIndex { target, key } => {
            assert!(matches!(&target.node, Expr::NameLookup(n) if n == "a"));
            assert!(is_binop(key).is_some());
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn chained_index() {
    let expr = parse_expr("a[0][1]");
    match &expr.node {
        Expr::SoftIndex { target, key } => {
            assert!(is_int(key, 1));
            match &target.node {
                Expr::SoftIndex {
                    target: inner,
                    key: inner_key,
                } => {
                    assert!(matches!(&inner.node, Expr::NameLookup(n) if n == "a"));
                    assert!(is_int(inner_key, 0));
                }
                other => panic!("expected inner SoftIndex, got {other:?}"),
            }
        }
        other => panic!("expected SoftIndex, got {other:?}"),
    }
}

#[test]
fn index_with_newlines() {
    let expr = parse_expr("a[\n0\n]");
    match &expr.node {
        Expr::SoftIndex { target, key } => {
            assert!(matches!(&target.node, Expr::NameLookup(n) if n == "a"));
            assert!(is_int(key, 0));
        }
        other => panic!("expected SoftIndex, got {other:?}"),
    }
}

// -- Dot access --

#[test]
fn dot_access() {
    let expr = parse_expr("a.foo");
    match &expr.node {
        Expr::HardIndex { target, key } => {
            assert!(matches!(&target.node, Expr::NameLookup(n) if n == "a"));
            assert!(key == "foo");
        }
        other => panic!("expected HardIndex, got {other:?}"),
    }
}

#[test]
fn chained_dot() {
    let expr = parse_expr("a.b.c");
    match &expr.node {
        Expr::HardIndex { target, key } => {
            assert!(key == "c");
            match &target.node {
                Expr::HardIndex {
                    target: inner,
                    key: inner_key,
                } => {
                    assert!(matches!(&inner.node, Expr::NameLookup(n) if n == "a"));
                    assert!(inner_key == "b");
                }
                other => panic!("expected inner HardIndex, got {other:?}"),
            }
        }
        other => panic!("expected HardIndex, got {other:?}"),
    }
}

#[test]
fn dot_then_call() {
    let expr = parse_expr("a.foo(1)");
    match &expr.node {
        Expr::Call { callee, args } => {
            assert_eq!(args.len(), 1);
            assert!(is_int(&args[0], 1));
            match &callee.node {
                Expr::HardIndex { target, key } => {
                    assert!(matches!(&target.node, Expr::NameLookup(n) if n == "a"));
                    assert!(key == "foo");
                }
                other => panic!("expected HardIndex inside Call, got {other:?}"),
            }
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

// -- Threading --

#[test]
fn thread_no_extra_args() {
    let expr = parse_expr("a @ f()");
    match &expr.node {
        Expr::Call { callee, args } => {
            assert!(matches!(&callee.node, Expr::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 1);
            assert!(matches!(&args[0].node, Expr::NameLookup(n) if n == "a"));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn thread_with_args() {
    let expr = parse_expr("a @ f(1)");
    match &expr.node {
        Expr::Call { callee, args } => {
            assert!(matches!(&callee.node, Expr::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0].node, Expr::NameLookup(n) if n == "a"));
            assert!(is_int(&args[1], 1));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn thread_multiple_extra_args() {
    let expr = parse_expr("a @ f(1, 2)");
    match &expr.node {
        Expr::Call { callee, args } => {
            assert!(matches!(&callee.node, Expr::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 3);
            assert!(matches!(&args[0].node, Expr::NameLookup(n) if n == "a"));
            assert!(is_int(&args[1], 1));
            assert!(is_int(&args[2], 2));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn thread_chained() {
    // a @ f(1) @ g(2) == g(f(a, 1), 2)
    let expr = parse_expr("a @ f(1) @ g(2)");
    match &expr.node {
        Expr::Call { callee, args } => {
            assert!(matches!(&callee.node, Expr::NameLookup(n) if n == "g"));
            assert_eq!(args.len(), 2);
            assert!(is_int(&args[1], 2));
            match &args[0].node {
                Expr::Call {
                    callee: inner_callee,
                    args: inner_args,
                } => {
                    assert!(matches!(&inner_callee.node, Expr::NameLookup(n) if n == "f"));
                    assert_eq!(inner_args.len(), 2);
                    assert!(matches!(&inner_args[0].node, Expr::NameLookup(n) if n == "a"));
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
    let expr = parse_expr("a @ m.f()");
    match &expr.node {
        Expr::Call { callee, args } => {
            assert_eq!(args.len(), 1);
            assert!(matches!(&args[0].node, Expr::NameLookup(n) if n == "a"));
            match &callee.node {
                Expr::HardIndex { target, key } => {
                    assert!(matches!(&target.node, Expr::NameLookup(n) if n == "m"));
                    assert!(key == "f");
                }
                other => panic!("expected HardIndex callee, got {other:?}"),
            }
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn thread_after_postfix() {
    // a.b @ f() == f(Index(a, "b"))
    let expr = parse_expr("a.b @ f()");
    match &expr.node {
        Expr::Call { callee, args } => {
            assert!(matches!(&callee.node, Expr::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 1);
            assert!(matches!(&args[0].node, Expr::HardIndex { .. }));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn thread_result_indexed() {
    // a @ f()[0]
    let expr = parse_expr("a @ f()[0]");
    match &expr.node {
        Expr::SoftIndex { target, key } => {
            assert!(is_int(key, 0));
            match &target.node {
                Expr::Call { callee, args } => {
                    assert!(matches!(&callee.node, Expr::NameLookup(n) if n == "f"));
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0].node, Expr::NameLookup(n) if n == "a"));
                }
                other => panic!("expected Call, got {other:?}"),
            }
        }
        other => panic!("expected SoftIndex, got {other:?}"),
    }
}

// -- Mixed postfix chains --

#[test]
fn call_then_index() {
    let expr = parse_expr("f()[0]");
    match &expr.node {
        Expr::SoftIndex { target, key } => {
            assert!(is_int(key, 0));
            assert!(matches!(&target.node, Expr::Call { .. }));
        }
        other => panic!("expected SoftIndex, got {other:?}"),
    }
}

#[test]
fn index_then_call() {
    let expr = parse_expr("a[0](1)");
    match &expr.node {
        Expr::Call { callee, args } => {
            assert_eq!(args.len(), 1);
            assert!(is_int(&args[0], 1));
            assert!(matches!(&callee.node, Expr::SoftIndex { .. }));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_then_dot() {
    let expr = parse_expr("f().bar");
    match &expr.node {
        Expr::HardIndex { target, key } => {
            assert!(key == "bar");
            assert!(matches!(&target.node, Expr::Call { .. }));
        }
        other => panic!("expected HardIndex, got {other:?}"),
    }
}

#[test]
fn dot_then_index() {
    let expr = parse_expr("a.b[0]");
    match &expr.node {
        Expr::SoftIndex { target, key } => {
            assert!(is_int(key, 0));
            match &target.node {
                Expr::HardIndex { key: inner_key, .. } => {
                    assert!(inner_key == "b");
                }
                other => panic!("expected inner HardIndex, got {other:?}"),
            }
        }
        other => panic!("expected SoftIndex, got {other:?}"),
    }
}

#[test]
fn index_then_dot() {
    let expr = parse_expr("a[0].bar");
    match &expr.node {
        Expr::HardIndex { target, key } => {
            assert!(key == "bar");
            match &target.node {
                Expr::SoftIndex { key: inner_key, .. } => {
                    assert!(is_int(inner_key, 0));
                }
                other => panic!("expected inner SoftIndex, got {other:?}"),
            }
        }
        other => panic!("expected HardIndex, got {other:?}"),
    }
}

#[test]
fn long_postfix_chain() {
    // a.b[0].c(1).d
    let expr = parse_expr("a.b[0].c(1).d");
    match &expr.node {
        Expr::HardIndex { target, key } => {
            assert!(key == "d");
            match &target.node {
                Expr::Call { callee, args } => {
                    assert_eq!(args.len(), 1);
                    assert!(is_int(&args[0], 1));
                    match &callee.node {
                        Expr::HardIndex { target, key } => {
                            assert!(key == "c");
                            match &target.node {
                                Expr::SoftIndex { target, key } => {
                                    assert!(is_int(key, 0));
                                    match &target.node {
                                        Expr::HardIndex { target, key } => {
                                            assert!(key == "b");
                                            assert!(
                                                matches!(&target.node, Expr::NameLookup(n) if n == "a")
                                            );
                                        }
                                        other => panic!("expected .b HardIndex, got {other:?}"),
                                    }
                                }
                                other => panic!("expected [0] SoftIndex, got {other:?}"),
                            }
                        }
                        other => panic!("expected .c HardIndex, got {other:?}"),
                    }
                }
                other => panic!("expected Call, got {other:?}"),
            }
        }
        other => panic!("expected .d HardIndex, got {other:?}"),
    }
}

// -- Postfix in binary operands --

#[test]
fn calls_in_arithmetic() {
    let expr = parse_expr("f(1) + g(2)");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Add));
    assert!(matches!(&left.node, Expr::Call { .. }));
    assert!(matches!(&right.node, Expr::Call { .. }));
}

#[test]
fn index_in_arithmetic() {
    let expr = parse_expr("a[0] * b[1]");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Mul));
    assert!(matches!(&left.node, Expr::SoftIndex { .. }));
    assert!(matches!(&right.node, Expr::SoftIndex { .. }));
}

#[test]
fn dot_in_comparison() {
    let expr = parse_expr("a.x == b.y");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Eq));
    assert!(matches!(&left.node, Expr::HardIndex { .. }));
    assert!(matches!(&right.node, Expr::HardIndex { .. }));
}

// -- Postfix binds tighter than prefix --

#[test]
fn negate_index() {
    // -a[0] == -(a[0])
    let expr = parse_expr("-a[0]");
    match &expr.node {
        Expr::UnaryOp {
            op: Spanned {
                node: UnaryOp::Negate,
                ..
            },
            operand,
        } => {
            assert!(matches!(&operand.node, Expr::SoftIndex { .. }));
        }
        other => panic!("expected Negate(SoftIndex), got {other:?}"),
    }
}

#[test]
fn negate_dot() {
    // -a.b == -(a.b): dot indexing binds tighter than prefix negate.
    let expr = parse_expr("-a.b");
    match &expr.node {
        Expr::UnaryOp {
            op: Spanned {
                node: UnaryOp::Negate,
                ..
            },
            operand,
        } => assert!(matches!(&operand.node, Expr::HardIndex { .. })),
        other => panic!("expected Negate(Index), got {other:?}"),
    }
}

#[test]
fn not_dot() {
    // not a.b == not (a.b): dot indexing binds tighter than prefix not.
    let expr = parse_expr("not a.b");
    match &expr.node {
        Expr::UnaryOp {
            op: Spanned {
                node: UnaryOp::Not, ..
            },
            operand,
        } => assert!(matches!(&operand.node, Expr::HardIndex { .. })),
        other => panic!("expected Not(Index), got {other:?}"),
    }
}

// -- Newline sensitivity with postfix --

#[test]
fn newline_before_bracket_is_two_statements() {
    // `[` can begin a statement, so it does not continue across a newline.
    let program = parse_program("test.frst", "a\n[0]").expect("failed to parse");
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn newline_before_call_is_two_statements() {
    // `(` can begin a statement, so it does not continue across a newline.
    let program = parse_program("test.frst", "f\n(1)").expect("failed to parse");
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn newline_before_dot_continues() {
    // `.` cannot begin a statement, so it continues the expression.
    let expr = parse_expr("a\n.foo");
    match &expr.node {
        Expr::HardIndex { target, key } => {
            assert!(matches!(&target.node, Expr::NameLookup(n) if n == "a"));
            assert_eq!(key, "foo");
        }
        other => panic!("expected HardIndex, got {other:?}"),
    }
}

#[test]
fn newline_before_thread_continues() {
    // `@` cannot begin a statement, so it continues the expression: a @ f() => f(a).
    let expr = parse_expr("a\n@ f()");
    match &expr.node {
        Expr::Call { callee, args } => {
            assert!(matches!(&callee.node, Expr::NameLookup(n) if n == "f"));
            assert_eq!(args.len(), 1);
            assert!(matches!(&args[0].node, Expr::NameLookup(n) if n == "a"));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn newline_dot_chain() {
    // A whole leading-dot chain across newlines: ((a.b).c).d
    let expr = parse_expr("a\n.b\n.c\n.d");
    let Expr::HardIndex {
        target: abc,
        key: d,
    } = &expr.node
    else {
        panic!("expected HardIndex, got {:?}", expr.node)
    };
    assert_eq!(d, "d");
    let Expr::HardIndex { target: ab, key: c } = &abc.node else {
        panic!("expected nested HardIndex")
    };
    assert_eq!(c, "c");
    assert!(matches!(&ab.node, Expr::HardIndex { key, .. } if key == "b"));
}

#[test]
fn newline_thread_chain() {
    // Leading-`@` chain across newlines: g(f(x)).
    let expr = parse_expr("x\n@ f()\n@ g()");
    let Expr::Call {
        callee: g,
        args: outer,
    } = &expr.node
    else {
        panic!("expected Call, got {:?}", expr.node)
    };
    assert!(matches!(&g.node, Expr::NameLookup(n) if n == "g"));
    assert!(matches!(&outer[0].node, Expr::Call { .. }));
}

#[test]
fn multiple_newlines_before_dot_continue() {
    // Blank lines between the operand and the dot are still a continuation.
    let expr = parse_expr("a\n\n\n.foo");
    assert!(matches!(&expr.node, Expr::HardIndex { key, .. } if key == "foo"));
}

#[test]
fn comment_then_newline_before_dot_continues() {
    // Comments are lexer-skipped, so a trailing comment does not break the chain.
    let expr = parse_expr("a # comment\n.foo");
    assert!(matches!(&expr.node, Expr::HardIndex { key, .. } if key == "foo"));
}

#[test]
fn newline_before_dot_inside_delimiters() {
    // Inside delimiters a dot after a newline continues the inner expression.
    let expr = parse_expr("(a\n.foo)");
    assert!(matches!(&expr.node, Expr::HardIndex { key, .. } if key == "foo"));
}

// -- Postfix error cases --

#[test]
fn error_dot_no_identifier() {
    let err = parse_err("a.42");
    assert!(
        err.contains("unexpected") || err.contains("expected identifier"),
        "error was: {err}"
    );
}

#[test]
fn error_unclosed_index() {
    let err = parse_err("a[0");
    assert!(
        err.contains("end of input") || err.contains("Expected ]"),
        "error was: {err}"
    );
}

#[test]
fn error_unclosed_call() {
    let err = parse_err("f(1, 2");
    assert!(
        err.contains("end of input") || err.contains("Expected )"),
        "error was: {err}"
    );
}

#[test]
fn error_thread_no_parens() {
    let err = parse_err("a @ f");
    assert!(
        err.contains("Expected (") || err.contains("unexpected"),
        "error was: {err}"
    );
}
