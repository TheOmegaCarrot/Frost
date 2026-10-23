//! The iteration functions (`map`, `filter`, `reduce`, `each`) are ordinary names, as
//! are `foreach`, `init`, and `with`: the grammar gives none of them special meaning.

use crate::helpers;

use frostlang_parse::ast::*;
use helpers::*;

/// Every word these tests hold to be an ordinary name.
const WORDS: [&str; 7] = ["map", "filter", "reduce", "each", "foreach", "init", "with"];

fn is_name(expr: &Spanned<Expr>, name: &str) -> bool {
    matches!(&expr.node, Expr::NameLookup(n) if n == name)
}

/// The callee and arguments of `expr`, a call.
fn assert_call(expr: &Spanned<Expr>) -> (&Spanned<Expr>, &[Spanned<Expr>]) {
    match &expr.node {
        Expr::Call { callee, args } => (callee, args),
        other => panic!("expected Call, got {other:?}"),
    }
}

/// The single statement of `src`, a `def`, as its pattern and value.
fn parse_def(src: &str) -> (Pattern, Spanned<Expr>) {
    let program = parse(src);
    assert_eq!(
        program.statements.len(),
        1,
        "{src:?} should be one statement"
    );
    match program.statements.into_iter().next().unwrap().node {
        Statement::Def { pattern, expr, .. } => (pattern.node, expr),
        other => panic!("expected Def from {src:?}, got {other:?}"),
    }
}

fn is_binding(pattern: &Pattern, name: &str) -> bool {
    matches!(
        pattern,
        Pattern::Binding { name: binding, .. } if binding.node == Binding::Named(name.to_owned())
    )
}

// ============================================================
// Calls
// ============================================================

mod calls {
    use super::*;

    #[test]
    fn two_argument_calls() {
        for word in ["map", "filter", "reduce", "each"] {
            let src = format!("{word}([1, 2, 3], fn x -> x)");
            let expr = parse_expr(&src);
            let (callee, args) = assert_call(&expr);
            assert!(is_name(callee, word), "{src:?} should call `{word}`");
            assert_eq!(args.len(), 2, "{src:?} should pass two arguments");
            assert!(
                matches!(args[0].node, Expr::Array(_)),
                "{src:?} should pass the Array first"
            );
            assert!(
                matches!(args[1].node, Expr::Lambda { .. }),
                "{src:?} should pass the lambda last"
            );
        }
    }

    #[test]
    fn seeded_reduce_is_a_three_argument_call() {
        let expr = parse_expr("reduce(xs, 0, fn acc, x -> acc + x)");
        let (callee, args) = assert_call(&expr);
        assert!(is_name(callee, "reduce"));
        assert_eq!(args.len(), 3);
        assert!(is_name(&args[0], "xs"));
        assert!(is_int(&args[1], 0));
        assert!(matches!(args[2].node, Expr::Lambda { .. }));
    }

    #[test]
    fn threaded_calls() {
        for word in ["map", "filter", "reduce", "each"] {
            let src = format!("xs @ {word}(f)");
            let expr = parse_expr(&src);
            let (callee, args) = assert_call(&expr);
            assert!(is_name(callee, word), "{src:?} should call `{word}`");
            assert_eq!(args.len(), 2, "{src:?} should pass two arguments");
            assert!(is_name(&args[0], "xs"), "{src:?} should thread `xs` first");
            assert!(is_name(&args[1], "f"), "{src:?} should pass `f` second");
        }
    }

    // A call ends at its `)`, so a following `@` threads the whole result onward.
    #[test]
    fn a_thread_after_the_call_takes_its_result() {
        let expr = parse_expr("map(xs, fn x -> x) @ len()");
        let (callee, args) = assert_call(&expr);
        assert!(is_name(callee, "len"), "the outer call should be `len`");
        assert_eq!(args.len(), 1);
        let (inner, inner_args) = assert_call(&args[0]);
        assert!(
            is_name(inner, "map"),
            "`len` should take the result of `map`"
        );
        assert_eq!(inner_args.len(), 2);
    }

    #[test]
    fn nested_calls() {
        let expr = parse_expr("reduce(filter(map(xs, f), g), plus)");
        let (callee, args) = assert_call(&expr);
        assert!(is_name(callee, "reduce"));
        let (callee, args) = assert_call(&args[0]);
        assert!(is_name(callee, "filter"));
        let (callee, _) = assert_call(&args[0]);
        assert!(is_name(callee, "map"));
    }

    #[test]
    fn bare_names_are_lookups() {
        for word in WORDS {
            assert!(
                is_name(&parse_expr(word), word),
                "{word:?} alone should be a name lookup"
            );
        }
    }

    #[test]
    fn passed_as_arguments() {
        let expr = parse_expr("compose(map, filter, with, init)");
        let (_, args) = assert_call(&expr);
        let names: Vec<_> = args
            .iter()
            .map(|arg| match &arg.node {
                Expr::NameLookup(n) => n.as_str(),
                other => panic!("expected a name, got {other:?}"),
            })
            .collect();
        assert_eq!(names, ["map", "filter", "with", "init"]);
    }
}

// ============================================================
// Names
// ============================================================

mod names {
    use super::*;

    #[test]
    fn def_binds_each_word() {
        for word in WORDS {
            let src = format!("def {word} = 1");
            let (pattern, expr) = parse_def(&src);
            assert!(is_binding(&pattern, word), "{src:?} should bind `{word}`");
            assert!(is_int(&expr, 1), "{src:?} should bind 1");
        }
    }

    #[test]
    fn defn_names_each_word() {
        for word in WORDS {
            let src = format!("defn {word}(xs, f) -> f(xs)");
            let (pattern, expr) = parse_def(&src);
            assert!(is_binding(&pattern, word), "{src:?} should bind `{word}`");
            match &expr.node {
                Expr::Lambda { self_name, .. } => assert_eq!(
                    self_name.as_ref().map(|name| name.node.as_str()),
                    Some(word),
                    "{src:?} should name its lambda `{word}`"
                ),
                other => panic!("expected Lambda from {src:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn lambda_parameters() {
        let expr = parse_expr("fn init, with -> init + with");
        match &expr.node {
            Expr::Lambda { params, .. } => {
                let names: Vec<_> = params.iter().map(|p| &p.node).collect();
                assert_eq!(
                    names,
                    [
                        &Binding::Named("init".to_owned()),
                        &Binding::Named("with".to_owned())
                    ]
                );
            }
            other => panic!("expected Lambda, got {other:?}"),
        }
    }

    #[test]
    fn rest_parameter() {
        let expr = parse_expr("fn ...with -> with");
        match &expr.node {
            Expr::Lambda { variadic_param, .. } => assert_eq!(
                variadic_param.as_ref().map(|p| &p.node),
                Some(&Binding::Named("with".to_owned()))
            ),
            other => panic!("expected Lambda, got {other:?}"),
        }
    }

    #[test]
    fn map_literal_shorthand_keys() {
        let expr = parse_expr("{map, with}");
        let Expr::Map(entries) = &expr.node else {
            panic!("expected Map, got {:?}", expr.node);
        };
        assert_eq!(entries.len(), 2);
        for (entry, word) in entries.iter().zip(["map", "with"]) {
            assert!(
                matches!(&entry.node.key.node, Expr::Literal(Literal::String(k)) if k == word),
                "the key should be the String {word:?}"
            );
            assert!(
                is_name(&entry.node.value, word),
                "the value should look up `{word}`"
            );
        }
    }

    #[test]
    fn map_literal_explicit_keys() {
        let expr = parse_expr("{filter: 1, init: 2}");
        let Expr::Map(entries) = &expr.node else {
            panic!("expected Map, got {:?}", expr.node);
        };
        let keys: Vec<_> = entries
            .iter()
            .map(|entry| match &entry.node.key.node {
                Expr::Literal(Literal::String(k)) => k.as_str(),
                other => panic!("expected a String key, got {other:?}"),
            })
            .collect();
        assert_eq!(keys, ["filter", "init"]);
    }

    #[test]
    fn map_pattern_shorthand_keys() {
        let (pattern, _) = parse_def("def {init, with} = m");
        let Pattern::Map { entries, .. } = pattern else {
            panic!("expected a Map pattern, got {pattern:?}");
        };
        assert_eq!(entries.len(), 2);
        for (entry, word) in entries.iter().zip(["init", "with"]) {
            assert!(
                matches!(&entry.node.key.node, Expr::Literal(Literal::String(k)) if k == word),
                "the key should be the String {word:?}"
            );
            assert!(
                is_binding(&entry.node.pattern.node, word),
                "the entry should bind `{word}`"
            );
        }
    }

    #[test]
    fn match_binding() {
        let expr = parse_expr("match x { with => with }");
        let Expr::Match { arms, .. } = &expr.node else {
            panic!("expected Match, got {:?}", expr.node);
        };
        assert!(is_binding(&arms[0].node.pattern.node, "with"));
        assert!(is_name(&arms[0].node.result, "with"));
    }

    #[test]
    fn after_a_dot() {
        for word in WORDS {
            let src = format!("data.{word}");
            let expr = parse_expr(&src);
            match &expr.node {
                Expr::HardIndex { target, key } => {
                    assert!(is_name(target, "data"), "{src:?} should index `data`");
                    assert_eq!(key.node, word, "{src:?} should index the key {word:?}");
                }
                other => panic!("expected HardIndex from {src:?}, got {other:?}"),
            }
        }
    }

    // A module's entry named for an iteration function is called like any other.
    #[test]
    fn method_style_call() {
        let expr = parse_expr("db.reduce(0, f)");
        let (callee, args) = assert_call(&expr);
        assert!(matches!(
            &callee.node,
            Expr::HardIndex { key, .. } if key.node == "reduce"
        ));
        assert_eq!(args.len(), 2);
    }
}
