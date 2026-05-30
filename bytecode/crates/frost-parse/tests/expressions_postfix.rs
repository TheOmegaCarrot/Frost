mod helpers;

use frost_parse::ast::*;
use frost_parse::parse_program;
use helpers::*;

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
                ExprKind::Call {
                    callee: inner,
                    args: inner_args,
                } => {
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
    let program = parse_program("test.frst", "f\n(1)").expect("failed to parse");
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
    let expr = parse_expr("a[0][1]");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(is_int(key, 1));
            match &target.kind {
                ExprKind::Index {
                    target: inner,
                    key: inner_key,
                } => {
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
    let expr = parse_expr("a.b.c");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"c"));
            match &target.kind {
                ExprKind::Index {
                    target: inner,
                    key: inner_key,
                } => {
                    assert!(matches!(&inner.kind, ExprKind::NameLookup(n) if n == "a"));
                    assert!(
                        matches!(&inner_key.kind, ExprKind::Literal(Literal::String(s)) if s == b"b")
                    );
                }
                other => panic!("expected inner Index, got {other:?}"),
            }
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn dot_then_call() {
    let expr = parse_expr("a.foo(1)");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert_eq!(args.len(), 1);
            assert!(is_int(&args[0], 1));
            match &callee.kind {
                ExprKind::Index { target, key } => {
                    assert!(matches!(&target.kind, ExprKind::NameLookup(n) if n == "a"));
                    assert!(
                        matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"foo")
                    );
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
fn thread_multiple_extra_args() {
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
fn thread_chained() {
    // a @ f(1) @ g(2) == g(f(a, 1), 2)
    let expr = parse_expr("a @ f(1) @ g(2)");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert!(matches!(&callee.kind, ExprKind::NameLookup(n) if n == "g"));
            assert_eq!(args.len(), 2);
            assert!(is_int(&args[1], 2));
            match &args[0].kind {
                ExprKind::Call {
                    callee: inner_callee,
                    args: inner_args,
                } => {
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
    let expr = parse_expr("a @ m.f()");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert_eq!(args.len(), 1);
            assert!(matches!(&args[0].kind, ExprKind::NameLookup(n) if n == "a"));
            match &callee.kind {
                ExprKind::Index { target, key } => {
                    assert!(matches!(&target.kind, ExprKind::NameLookup(n) if n == "m"));
                    assert!(
                        matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"f")
                    );
                }
                other => panic!("expected Index callee, got {other:?}"),
            }
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn thread_after_postfix() {
    // a.b @ f() == f(Index(a, "b"))
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
    // a @ f()[0]
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

// -- Mixed postfix chains --

#[test]
fn call_then_index() {
    let expr = parse_expr("f()[0]");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(is_int(key, 0));
            assert!(matches!(&target.kind, ExprKind::Call { .. }));
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn index_then_call() {
    let expr = parse_expr("a[0](1)");
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            assert_eq!(args.len(), 1);
            assert!(is_int(&args[0], 1));
            assert!(matches!(&callee.kind, ExprKind::Index { .. }));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_then_dot() {
    let expr = parse_expr("f().bar");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"bar"));
            assert!(matches!(&target.kind, ExprKind::Call { .. }));
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn dot_then_index() {
    let expr = parse_expr("a.b[0]");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(is_int(key, 0));
            match &target.kind {
                ExprKind::Index { key: inner_key, .. } => {
                    assert!(
                        matches!(&inner_key.kind, ExprKind::Literal(Literal::String(s)) if s == b"b")
                    );
                }
                other => panic!("expected inner Index, got {other:?}"),
            }
        }
        other => panic!("expected Index, got {other:?}"),
    }
}

#[test]
fn index_then_dot() {
    let expr = parse_expr("a[0].bar");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"bar"));
            match &target.kind {
                ExprKind::Index { key: inner_key, .. } => {
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
    // a.b[0].c(1).d
    let expr = parse_expr("a.b[0].c(1).d");
    match &expr.kind {
        ExprKind::Index { target, key } => {
            assert!(matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"d"));
            match &target.kind {
                ExprKind::Call { callee, args } => {
                    assert_eq!(args.len(), 1);
                    assert!(is_int(&args[0], 1));
                    match &callee.kind {
                        ExprKind::Index { target, key } => {
                            assert!(
                                matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"c")
                            );
                            match &target.kind {
                                ExprKind::Index { target, key } => {
                                    assert!(is_int(key, 0));
                                    match &target.kind {
                                        ExprKind::Index { target, key } => {
                                            assert!(
                                                matches!(&key.kind, ExprKind::Literal(Literal::String(s)) if s == b"b")
                                            );
                                            assert!(
                                                matches!(&target.kind, ExprKind::NameLookup(n) if n == "a")
                                            );
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
    let expr = parse_expr("f(1) + g(2)");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Add));
    assert!(matches!(&left.kind, ExprKind::Call { .. }));
    assert!(matches!(&right.kind, ExprKind::Call { .. }));
}

#[test]
fn index_in_arithmetic() {
    let expr = parse_expr("a[0] * b[1]");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Mul));
    assert!(matches!(&left.kind, ExprKind::Index { .. }));
    assert!(matches!(&right.kind, ExprKind::Index { .. }));
}

#[test]
fn dot_in_comparison() {
    let expr = parse_expr("a.x == b.y");
    let (left, op, right) = is_binop(&expr).unwrap();
    assert!(matches!(op, BinOp::Eq));
    assert!(matches!(&left.kind, ExprKind::Index { .. }));
    assert!(matches!(&right.kind, ExprKind::Index { .. }));
}

// -- Postfix binds tighter than prefix --

#[test]
fn negate_index() {
    // -a[0] == -(a[0])
    let expr = parse_expr("-a[0]");
    match &expr.kind {
        ExprKind::UnaryOp {
            op: UnaryOp::Negate,
            operand,
        } => {
            assert!(matches!(&operand.kind, ExprKind::Index { .. }));
        }
        other => panic!("expected Negate(Index), got {other:?}"),
    }
}

#[test]
fn negate_dot() {
    // -a.b == -(a.b): dot indexing binds tighter than prefix negate.
    let expr = parse_expr("-a.b");
    match &expr.kind {
        ExprKind::UnaryOp {
            op: UnaryOp::Negate,
            operand,
        } => assert!(matches!(&operand.kind, ExprKind::Index { .. })),
        other => panic!("expected Negate(Index), got {other:?}"),
    }
}

#[test]
fn not_dot() {
    // not a.b == not (a.b): dot indexing binds tighter than prefix not.
    let expr = parse_expr("not a.b");
    match &expr.kind {
        ExprKind::UnaryOp {
            op: UnaryOp::Not,
            operand,
        } => assert!(matches!(&operand.kind, ExprKind::Index { .. })),
        other => panic!("expected Not(Index), got {other:?}"),
    }
}

// -- Newline sensitivity with postfix --

#[test]
fn newline_before_bracket_is_two_statements() {
    let program = parse_program("test.frst", "a\n[0]").expect("failed to parse");
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn newline_before_dot_is_error() {
    let err = parse_err("a\n.foo");
    assert!(err.contains("unexpected"), "error was: {err}");
}

#[test]
fn newline_before_call_is_two_statements() {
    let program = parse_program("test.frst", "f\n(1)").expect("failed to parse");
    assert_eq!(program.statements.len(), 2);
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
