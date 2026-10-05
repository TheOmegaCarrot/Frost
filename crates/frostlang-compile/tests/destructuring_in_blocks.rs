//! Destructuring inside blocks: `do` expressions and block-bodied lambdas.
//!
//! A block is its own scope, so its destructured bindings are visible only inside
//! it, may shadow enclosing names, and leave the stack as they found it. In a
//! lambda, destructuring is how a structured argument is taken apart. Array and
//! Map patterns themselves are covered by `array_destructuring.rs` and
//! `map_destructuring.rs`.
//!
//! The harness runs every behavioral case under every optimization permutation;
//! code-shape cases pin exactly the options they are about.

mod common;

use common::{Emitted, Script, UNOPTIMIZED, compile_errors, raises, run};
use frostlang_compile::{Optimization, OptimizationOptions};
use frostlang_runtime::{Bytecode, Value};

const FOLD: OptimizationOptions = UNOPTIMIZED.with(Optimization::ConstantFold, true);

/// Assert each `source` runs to the value of the Frost expression `expected`.
fn assert_values(cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(run(source), run(expected), "{source:?} is {expected}");
    }
}

/// Assert each `source` raises an error mentioning `message`.
fn assert_raises(cases: &[(&str, &str)]) {
    for (source, message) in cases {
        let raised = raises(source);
        assert!(
            raised.contains(message),
            "{source:?} raises about {message:?}, but raised: {raised}"
        );
    }
}

/// Assert each `source` fails to compile, with a diagnostic mentioning `message`.
fn assert_compile_errors(cases: &[(&str, &str)]) {
    for (source, message) in cases {
        let rendered = compile_errors(source).render_plain();
        assert!(
            rendered.contains(message),
            "{source:?} is rejected for {message:?}, but the diagnostic is:\n{rendered}"
        );
    }
}

fn calls(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
        .filter(|op| matches!(op, Bytecode::Call(_) | Bytecode::TailCall(_)))
        .count()
}

// --- In a `do` expression ---

#[test]
fn a_do_block_may_destructure() {
    assert_values(&[
        ("do { def [a, b] = [1, 2]; a + b }", "3"),
        ("do { def {w, h} = {w: 3, h: 4}; w * h }", "12"),
        (
            r"
            do {
                def [a, ...r] = [1, 2, 3]
                def {b} as m = {b: a}
                [a, r, b, m]
            }
            ",
            "[1, [2, 3], 1, {b: 1}]",
        ),
        ("do { def [{a}, [b]] = [{a: 1}, [2]]; [a, b] }", "[1, 2]"),
    ]);
}

#[test]
fn a_do_block_may_destructure_across_lines() {
    assert_values(&[(
        r"
        do {
            def [
                a,
                ...rest
            ] = [1, 2, 3]
            def {
                b: [c],
                [a]: d,
            } as m = {b: [4], [1]: 5}

            [a, rest, c, d, m]
        }
        ",
        "[1, [2, 3], 4, 5, {b: [4], [1]: 5}]",
    )]);
}

#[test]
fn a_destructure_may_use_an_earlier_one_in_the_block() {
    assert_values(&[
        (
            r"
            do {
                def [a, b] = [1, 2]
                def {c} = {c: a + b}
                c
            }
            ",
            "3",
        ),
        (
            r#"
            do {
                def [k] = ["b"]
                def {[k]: v} = {b: 5}
                v
            }
            "#,
            "5",
        ),
        (
            r"
            do {
                def [xs] = [[1, 2]]
                def [h, ...t] = xs
                [h, t]
            }
            ",
            "[1, [2]]",
        ),
    ]);
}

#[test]
fn a_do_blocks_bindings_stay_inside_it() {
    assert_compile_errors(&[
        (
            r"
            do { def [a] = [1]; a }
            a
            ",
            "`a` is not defined",
        ),
        (
            r"
            do { def {a} as m = {a: 1}; a }
            m
            ",
            "`m` is not defined",
        ),
        (
            r"
            do { def [a, ...r] = [1]; a }
            r
            ",
            "`r` is not defined",
        ),
        (
            r"
            do { def {a: [b]} = {a: [1]}; b }
            b
            ",
            "`b` is not defined",
        ),
        (
            r"
            def f = fn p -> {
                def [a, ...r] = p
                a
            }
            f([1])
            r
            ",
            "`r` is not defined",
        ),
    ]);
}

#[test]
fn a_do_blocks_destructure_may_shadow_an_enclosing_name() {
    assert_values(&[
        (
            r"
            def a = 1
            do { def [a] = [2]; a } + a
            ",
            "3",
        ),
        (
            r"
            def m = 0
            [do { def {} as m = {k: 1}; m }, m]
            ",
            "[{k: 1}, 0]",
        ),
        // Nested blocks shadow in turn.
        (
            r"
            do {
                def [a] = [1]
                [a, do { def {a} = {a: 2}; a }, a]
            }
            ",
            "[1, 2, 1]",
        ),
    ]);
}

#[test]
fn a_pattern_may_shadow_a_name_its_own_value_reads() {
    // The value is evaluated first, reading the enclosing `x`.
    assert_values(&[
        (
            r"
            def x = [1, 2]
            do { def [x, y] = x; [y, x] }
            ",
            "[2, 1]",
        ),
        (
            r"
            def m = {m: 5}
            do { def {m} = m; m }
            ",
            "5",
        ),
    ]);
}

#[test]
fn a_name_destructured_twice_in_one_block_is_a_compile_error() {
    assert_compile_errors(&[
        (
            r"
            do {
                def [a] = [1]
                def {a} = {a: 2}
                a
            }
            ",
            "`a` is already bound",
        ),
        (
            r"
            do {
                def a = 1
                def [a] = [2]
                a
            }
            ",
            "`a` is already bound",
        ),
    ]);
}

#[test]
fn block_destructuring_leaves_the_stack_balanced() {
    assert_values(&[
        (
            r"
            do {
                def [a, ...r] = [1, 2, 3]
                def {b} as m = {b: 4}
                5
            }
            ",
            "5",
        ),
        (
            r"
            do { def [a] = [1]; a }
            do { def {b} = {b: 2}; b }
            7
            ",
            "7",
        ),
        (
            "[do { def [a, b] = [1, 2]; a }, do { def {c} = {c: 3}; c }]",
            "[1, 3]",
        ),
    ]);
}

#[test]
fn block_destructuring_leaves_the_stack_balanced_in_every_expression_context() {
    assert_values(&[
        (
            r"
            [0, do {
                def [a, [b, ...c]] = [1, [2, 3]]
                def {d} as m = {d: a}
                [a, b, c, d]
            }, 9]
            ",
            "[0, [1, 2, [3], 1], 9]",
        ),
        (
            r"
            plus(
                do { def [a, ...r] = [1, 2]; a },
                do { def {b: [c]} = {b: [2]}; c }
            )
            ",
            "3",
        ),
        ("10 + do { def [a, [b]] = [1, [2]]; a + b }", "13"),
        (
            r"
            {
                x: do { def {k} as m = {k: 1}; k },
                y: do { def [_, v] = [0, 2]; v }
            }
            ",
            "{x: 1, y: 2}",
        ),
        (
            r#"$'<${do { def [s, ...r] = ["a", "b"]; s }}${do { def {t} = {t: "c"}; t }}>'"#,
            r#""<ac>""#,
        ),
        (
            r"
            if do { def [a] = [true]; a }: do { def {b} = {b: 1}; b }
            else: 2
            ",
            "1",
        ),
        (
            "[[10, 20], [30]][do { def [i, ..._] = [1, 0]; i }][0]",
            "30",
        ),
    ]);
}

#[test]
fn a_caught_destructuring_failure_leaves_the_stack_balanced() {
    // Each failure is partway through a pattern, with parts laid out on the stack.
    assert_values(&[
        (
            "[1, try_call(fn -> { def [a, [b, c], d] = [1, [2], 3]; a }).ok, 3]",
            "[1, false, 3]",
        ),
        (
            r"
            [1, try_call(fn -> {
                def [a, ...r] = [1, 2]
                def {b, c} = {b: 2}
                a
            }).ok, 3]
            ",
            "[1, false, 3]",
        ),
        (
            "[1, try_call(fn -> { def {a: {b: [c, d]}} as m = {a: {b: [1]}}; c }).ok, 3]",
            "[1, false, 3]",
        ),
        (
            "[1, try_call(fn -> { def {a, [null]: b} = {a: 1}; a }).ok, 3]",
            "[1, false, 3]",
        ),
    ]);
}

#[test]
fn a_failed_destructure_ends_the_block() {
    assert_raises(&[
        ("do { def [a] = [1, 2]; a }", "exactly 1 element"),
        ("do { def {a} = {}; a }", "no value at key 'a'"),
        (
            r"
            do {
                def [a] = [1]
                def {b} = 5
                a
            }
            ",
            "Map",
        ),
    ]);
}

#[test]
fn a_do_blocks_bindings_are_not_implicitly_exported() {
    let source = r"
        def r = do {
            def [a] = [1]
            def {b} = {b: 2}
            a + b
        }
        r
        ";
    let finished = Script::new(source).implicit_export().finish();
    assert_eq!(finished.exports.keys().collect::<Vec<_>>(), vec!["r"]);
}

#[test]
fn a_lambdas_bindings_are_not_implicitly_exported() {
    // The lambda is called in place rather than bound: an exported Closure would
    // differ between the optimization permutations' compilations.
    let source = r"
        def [x] = [(fn p -> {
            def [a, ...r] = p
            def {b} as m = {b: a}
            [a, r, b, m]
        })([1])]
        0
        ";
    let finished = Script::new(source).implicit_export().finish();
    assert_eq!(
        finished.exports.keys().collect::<Vec<_>>(),
        vec!["x"],
        "{finished:?}"
    );
    assert_eq!(finished.exports["x"], run("[1, [], 1, {b: 1}]"));
}

#[test]
fn a_constant_destructuring_block_folds_whole() {
    for source in [
        "do { def [a, b] = [1, 2]; 5 }",
        r"
        do {
            def {a} as m = {a: 1}
            def [x, ...y] = [1, 2]
            5
        }
        ",
        r"
        do {
            def [{a: [b, ...c]}, {d} as e] = [{a: [1, 2]}, {d: 3}]
            5
        }
        ",
        r#"
        do {
            def {[1 + 1]: v, ["k"]: w} = {[2]: 0, k: 1}
            5
        }
        "#,
        r"
        do {
            def [_, ..._] = [1, 2]
            def {a: _} as _ = {a: 1}
            5
        }
        ",
    ] {
        let emitted = Script::new(source).code(FOLD);
        assert!(
            !emitted
                .code
                .iter()
                .any(|op| matches!(op, Bytecode::DefLocal(_))),
            "{source:?}: {emitted:?}"
        );
        assert_eq!(
            emitted.count(&Bytecode::PushInt(5)),
            1,
            "{source:?}: {emitted:?}"
        );
    }
}

// --- In a lambda's block body ---

#[test]
fn a_lambda_may_destructure_its_arguments() {
    assert_values(&[
        ("(fn pair -> { def [a, b] = pair; a * b })([3, 4])", "12"),
        (
            "(fn opts -> { def {w, h} = opts; w * h })({w: 3, h: 4, unused: 0})",
            "12",
        ),
        (
            r#"
            (fn p -> {
                def {name, tags: [first, ...rest]} as all = p
                [name, first, rest, all.name]
            })({name: "n", tags: [1, 2]})
            "#,
            r#"["n", 1, [2], "n"]"#,
        ),
        (
            r"
            (fn a, b -> {
                def [x] = a
                def {y} = b
                x + y
            })([1], {y: 2})
            ",
            "3",
        ),
    ]);
}

#[test]
fn a_lambda_may_destructure_its_rest_argument() {
    assert_values(&[
        (
            r"
            (fn first, ...others -> {
                def [second, ...more] = others
                [first, second, more]
            })(1, 2, 3, 4)
            ",
            "[1, 2, [3, 4]]",
        ),
        (
            "(fn ...args -> { def [{k}, [v]] = args; [k, v] })({k: 1}, [2])",
            "[1, 2]",
        ),
    ]);
    assert_raises(&[(
        "(fn ...args -> { def [a, b] = args; a })(1)",
        "exactly 2 elements",
    )]);
}

#[test]
fn a_lambda_may_destructure_in_nested_blocks() {
    assert_values(&[(
        r"
        (fn p -> do {
            def [a] = p
            do {
                def [b] = [a + 1]
                def {c} = {c: b + 1}
                [a, b, c]
            }
        })([1])
        ",
        "[1, 2, 3]",
    )]);
}

#[test]
fn a_lambda_destructures_afresh_on_each_call() {
    assert_values(&[(
        r"
        def f = fn p -> {
            def [a, ...r] = p
            [a, r]
        }
        [f([1]), f([2, 3, 4]), f([5, 6])]
        ",
        "[[1, []], [2, [3, 4]], [5, [6]]]",
    )]);
    assert_raises(&[(
        r"
        def f = fn p -> {
            def [a, b] = p
            a
        }
        [f([1, 2]), f([1])]
        ",
        "exactly 2 elements",
    )]);
}

#[test]
fn a_destructured_binding_may_be_captured() {
    assert_values(&[
        (
            r"
            def make = fn p -> {
                def [a, b] = p
                fn -> a + b
            }
            make([1, 2])()
            ",
            "3",
        ),
        (
            r"
            def make = fn m -> {
                def {k} = m
                fn x -> x * k
            }
            transform([1, 2], make({k: 10}))
            ",
            "[10, 20]",
        ),
    ]);
}

#[test]
fn a_destructured_binding_may_be_captured_at_any_depth() {
    assert_values(&[
        (
            r"
            def make = fn p -> {
                def {a: [x, ...ys]} = p
                fn -> fn -> [x, ys]
            }
            make({a: [1, 2]})()()
            ",
            "[1, [2]]",
        ),
        (
            r#"
            def make = fn p -> {
                def [k] = p
                fn m -> {
                    def {[k]: v} = m
                    v
                }
            }
            make(["z"])({z: 5})
            "#,
            "5",
        ),
    ]);
}

#[test]
fn a_lambdas_computed_key_may_read_a_capture() {
    assert_values(&[(
        r#"
        def key = "x"
        (fn m -> { def {[key]: v} = m; v })({x: 1})
        "#,
        "1",
    )]);
}

#[test]
fn a_lambdas_pattern_may_shadow_a_capture() {
    assert_values(&[
        // The pattern's `a` is the lambda's own; the enclosing `a` is untouched.
        (
            r"
            def a = 1
            def f = fn p -> { def [a] = p; a }
            [f([5]), a]
            ",
            "[5, 1]",
        ),
        // The value reads the capture its pattern then shadows.
        (
            r"
            def x = 1
            (fn p -> { def [x, y] = [x, p]; [x, y] })(2)
            ",
            "[1, 2]",
        ),
        // So does a computed key before the entry that shadows it.
        (
            r#"
            def b = "x"
            (fn m -> { def {[b]: a, b} = m; [a, b] })({x: 1, b: 2})
            "#,
            "[1, 2]",
        ),
        // `as` binds last, so a computed key reads the enclosing name it shadows.
        (
            r#"
            def m = {k: "a"}
            (fn v -> { def {[m.k]: x} as m = v; [x, m] })({a: 1})
            "#,
            "[1, {a: 1}]",
        ),
    ]);
}

#[test]
fn a_recursive_lambda_may_destructure() {
    assert_values(&[
        (
            r"
            defn sum(xs) -> if xs == []: 0 else: do {
                def [h, ...t] = xs
                h + sum(t)
            }
            sum([1, 2, 3, 4])
            ",
            "10",
        ),
        (
            r"
            defn pairs(m) -> {
                def {a, b} = m
                if a == 0: b else: pairs({a: a - 1, b: b + a})
            }
            pairs({a: 4, b: 0})
            ",
            "10",
        ),
    ]);
}

#[test]
fn tail_recursion_through_destructuring_runs_in_bounded_depth() {
    // Build a long Array, then walk it by destructuring, both by tail recursion.
    let source = r"
        defn build(n, acc) -> if n == 0: acc else: build(n - 1, acc + [n])
        defn total(xs, acc) -> if xs == []: acc else: do {
            def [h, ...t] = xs
            total(t, acc + h)
        }
        total(build(2000, []), 0)
        ";
    let tail = Script::new(source).max_call_depth(100).run();
    assert_eq!(tail, Value::Int(2_001_000));
}

#[test]
fn tail_recursion_through_map_destructuring_runs_in_bounded_depth() {
    let source = r"
        defn count(state) -> {
            def {n, acc: [total, ...seen]} as whole = state
            if n == 0: [total, seen, whole.n]
            else: count({n: n - 1, acc: [total + n, n]})
        }
        count({n: 2000, acc: [0]})
        ";
    let tail = Script::new(source).max_call_depth(100).run();
    assert_eq!(tail, run("[2001000, [1], 0]"));
}

#[test]
fn a_lambda_passed_to_a_higher_order_function_may_destructure() {
    assert_values(&[
        (
            "select([{n: 1}, {n: 5}, {n: 3}], fn m -> { def {n} = m; n > 2 })",
            "[{n: 5}, {n: 3}]",
        ),
        (
            "fold([[1, 2], [3, 4]], fn acc, p -> { def [a, b] = p; acc + a * b }, 0)",
            "14",
        ),
        (
            r"
            def c = mutable_cell(0)
            each([{v: [1]}, {v: [2]}], fn m -> {
                def {v: [x]} = m
                c.exchange(c.get() + x)
            })
            c.get()
            ",
            "3",
        ),
        (
            "call(fn p -> { def [a, ...r] = p; [a, r] }, [[1, 2]])",
            "[1, [2]]",
        ),
        (
            "map {a: [1, 2], b: [3, 4]} with fn k, v -> { def [x, y] = v; {[k]: x + y} }",
            "{a: 3, b: 7}",
        ),
        (
            "filter [[1, 2], [3, 1]] with fn p -> { def [a, b] = p; a < b }",
            "[[1, 2]]",
        ),
        (
            "reduce [{x: 1}, {x: 2}] init: 10 with fn acc, m -> { def {x} as _ = m; acc + x }",
            "13",
        ),
        (
            "[[1, 2], [3, 4]] @ transform(fn p -> { def [a, b] = p; [b, a] })",
            "[[2, 1], [4, 3]]",
        ),
    ]);
    // A mismatch inside the operation raises out of the iteration.
    assert_raises(&[(
        "transform([[1, 2], [3]], fn p -> { def [a, b] = p; a })",
        "exactly 2 elements",
    )]);
}

#[test]
fn an_abbreviated_lambda_may_destructure_in_a_do_block() {
    assert_values(&[(
        "transform([[1, 2], [3, 4]], $(do { def [a, b] = $; a * b }))",
        "[2, 12]",
    )]);
}

#[test]
fn a_destructure_may_not_rebind_a_parameter() {
    // A lambda's body shares its parameters' scope.
    assert_compile_errors(&[
        (
            r"
            fn a -> {
                def [a] = [1]
                a
            }
            ",
            "`a` is already bound",
        ),
        (
            "fn m -> { def {k} as m = {k: 1}; k }",
            "`m` is already bound",
        ),
        (
            "fn a, ...rest -> { def {rest} = a; rest }",
            "`rest` is already bound",
        ),
        (
            "fn p -> { def [{q: [p]}] = [{q: [1]}]; p }",
            "`p` is already bound",
        ),
        // A named lambda's own name is bound in the same scope as its parameters.
        (
            r"
            defn f(p) -> {
                def [f] = p
                f
            }
            ",
            "`f` is already bound",
        ),
    ]);
}

#[test]
fn a_nested_block_in_a_lambda_may_rebind_a_parameter() {
    assert_values(&[
        ("(fn a -> do { def [a] = [a + 1]; a })(1)", "2"),
        (
            r"
            defn f(p) -> do { def {f} = p; f }
            f({f: 3})
            ",
            "3",
        ),
    ]);
}

#[test]
fn a_pure_lambda_that_destructures_is_usable_in_a_fold() {
    let source = r"
        transform([[1, 2], [3, 4]], fn p -> {
            def [a, b] = p
            a * b
        })
        ";
    assert_values(&[(source, "[2, 12]")]);
    let emitted = Script::new(source).code(FOLD);
    assert_eq!(calls(&emitted), 0, "folded away: {emitted:?}");
}
