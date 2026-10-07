//! Lambda behavior, end to end: definition, calling, arity, both lambda forms,
//! `defn`, captures and scoping, recursion, higher-order use, closures as
//! values, and the errors a lambda can raise or fail to compile with.
//!
//! Some errors are reported at compile time rather than runtime: any unbound
//! name (even in a lambda never called, or a branch never taken), and a name
//! bound twice in one scope (duplicate parameters, a parameter or local `def`
//! shadowing the self-name or another parameter).
//!
//! Expected values are written as Frost expressions. The harness runs every case
//! under every optimization permutation. What the compiler emits for lambdas
//! (folding, effects, tail calls) is covered by `lambda_compilation.rs`.

mod script;

use frostlang::Value;
use script::{Script, compile_errors, raises, run};

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

// --- Defining and calling ---

#[test]
fn a_lambda_is_called_with_its_arguments() {
    assert_values(&[
        (
            r"
            def f = fn -> 42
            f()
            ",
            "42",
        ),
        (
            r"
            def f = fn x -> x + 1
            f(2)
            ",
            "3",
        ),
        (
            r"
            def f = fn x, y -> x + y
            f(2, 3)
            ",
            "5",
        ),
        ("(fn(a, b, c) -> [a, b, c])(1, 2, 3)", "[1, 2, 3]"),
        ("(fn x -> x)(1)", "1"),
        ("(fn() -> 42)()", "42"),
        (
            r"
            def f = fn foo() -> 1
            f()
            ",
            "1",
        ),
    ]);
}

#[test]
fn a_lambda_body_may_be_a_block() {
    assert_values(&[
        (
            r"
            def f = fn x -> {
                def y = 1
                y + x
            }
            f(4)
            ",
            "5",
        ),
        (
            r"
            def f = fn x -> do {
                def y = x + 1
                y * 2
            }
            f(3)
            ",
            "8",
        ),
    ]);
}

#[test]
fn a_lambda_body_may_be_a_map_literal() {
    // `{ }` and `{a: 1}` after the arrow are Map literals, not blocks.
    assert_values(&[
        (
            r"
            def f = fn (p, q) -> { }
            f(1, 2)
            ",
            "{}",
        ),
        (
            r"
            def f = fn -> {a: 1}
            f()
            ",
            "{a: 1}",
        ),
        (
            r"
            def k = 1
            def v = 10
            def f = fn -> ({[k]: v})
            f()
            ",
            "{[1]: 10}",
        ),
    ]);
}

#[test]
fn a_lambda_body_may_be_a_shorthand_map_literal() {
    // `{a}` after the arrow is the Map `{a: a}`, not a block returning `a`.
    assert_values(&[
        ("(fn a -> {a})(4)", "{a: 4}"),
        ("(fn a, b -> {a, b})(1, 2)", "{a: 1, b: 2}"),
        (
            r"
            def a = 3
            (fn -> {a})()
            ",
            "{a: 3}",
        ),
    ]);
}

#[test]
fn a_single_name_before_the_arrow_is_a_parameter() {
    // `fn f -> f` is the identity function, not a lambda named `f`.
    assert_values(&[(
        r"
        def g = fn f -> f
        g(42)
        ",
        "42",
    )]);
}

#[test]
fn a_discarded_parameter_takes_an_argument_it_ignores() {
    assert_values(&[("(fn _, b -> b)(1, 2)", "2")]);
}

#[test]
fn any_number_of_parameters_may_be_discarded() {
    // A discard binds no name, so several never collide.
    assert_values(&[
        ("(fn _, _ -> 1)(2, 3)", "1"),
        ("(fn _, b, _ -> b)(1, 2, 3)", "2"),
        ("(fn _, ..._ -> 1)(2, 3, 4)", "1"),
        ("$($3)(1, 2, 3)", "3"),
    ]);
}

#[test]
fn every_parameter_list_shape_binds_its_arguments() {
    // Unparenthesized, parenthesized, and named, each with discards, a variadic
    // parameter, and a block body in turn.
    assert_values(&[
        ("(fn _ -> 1)(2)", "1"),
        ("(fn ...rest -> rest)(1, 2)", "[1, 2]"),
        ("(fn ..._ -> 1)(2, 3)", "1"),
        ("(fn a, _, ...rest -> [a, rest])(1, 2, 3, 4)", "[1, [3, 4]]"),
        ("(fn(_) -> 1)(2)", "1"),
        ("(fn(_, b, ..._) -> b)(1, 2, 3)", "2"),
        (
            r"
            (fn(a, _, ...rest) -> {
                def n = [a, rest]
                n
            })(1, 2, 3)
            ",
            "[1, [3]]",
        ),
        ("(fn f(_) -> 1)(2)", "1"),
        ("(fn f(_, _) -> 1)(2, 3)", "1"),
        ("(fn f(...rest) -> rest)(1, 2)", "[1, 2]"),
        ("(fn f(..._) -> 1)()", "1"),
        ("(fn f(a, _, ...rest) -> [a, rest])(1, 2, 3)", "[1, [3]]"),
        (
            r"
            (fn f(a, ...rest) -> {
                def s = [a, rest]
                s
            })(1, 2)
            ",
            "[1, [2]]",
        ),
        (
            r"
            (fn f() -> {
                def a = 1
                a + 1
            })()
            ",
            "2",
        ),
    ]);
}

#[test]
fn a_parameter_list_may_end_in_a_trailing_comma() {
    assert_values(&[
        ("(fn(x, y,) -> x + y)(1, 2)", "3"),
        ("(fn f(a, b,) -> a + b)(1, 2)", "3"),
        (
            r"
            defn f(a, b,) -> a - b
            f(3, 1)
            ",
            "2",
        ),
    ]);
}

#[test]
fn a_braced_single_expression_is_the_body() {
    // `{42}` and `{a.b}` cannot begin a Map, so they are blocks.
    assert_values(&[("(fn -> { 42 })()", "42"), ("(fn a -> {a.b})({b: 4})", "4")]);
}

// --- When the body runs ---

#[test]
fn defining_a_lambda_does_not_run_its_body() {
    assert_values(&[(
        r"
        def c = mutable_cell(0)
        def f = fn -> c.exchange(99)
        c.get()
        ",
        "0",
    )]);
}

#[test]
fn calling_a_lambda_runs_its_body() {
    assert_values(&[(
        r"
        def c = mutable_cell(0)
        def f = fn -> c.exchange(99)
        f()
        c.get()
        ",
        "99",
    )]);
}

#[test]
fn body_statements_run_in_order_before_the_result() {
    assert_values(&[
        (
            r"
            def c = mutable_cell(0)
            def f = fn -> {
                c.exchange(c.get() + 1)
                c.exchange(c.get() + 10)
                c.get()
            }
            f()
            ",
            "11",
        ),
        (
            r"
            def c = mutable_cell(0)
            def f = fn -> {
                c.exchange(1)
                c.get() + 100
            }
            f()
            ",
            "101",
        ),
        (
            r"
            def c = mutable_cell(0)
            def f = fn -> {
                c.exchange(7)
                null
            }
            f()
            c.get()
            ",
            "7",
        ),
    ]);
}

#[test]
fn each_call_runs_the_body_afresh() {
    assert_values(&[(
        r"
        def c = mutable_cell(0)
        def inc = fn -> c.exchange(c.get() + 1)
        def peek = fn -> c.get()
        inc()
        inc()
        peek()
        ",
        "2",
    )]);
}

// --- Arity ---

#[test]
fn a_call_must_supply_exactly_the_parameters() {
    assert_raises(&[
        (
            r"
            def f = fn x, y -> x + y
            f(1)
            ",
            "expects 2 arguments",
        ),
        (
            r"
            def f = fn x -> x + 1
            f(1, 2)
            ",
            "expects 1 arguments",
        ),
        (
            r"
            def f = fn -> 1
            f(1)
            ",
            "expects 0 arguments",
        ),
    ]);
}

#[test]
fn arity_is_checked_before_the_body_runs() {
    // Were the body to run, calling the Int `p` would raise instead.
    assert_raises(&[(
        r"
        def f = fn p -> {
            p()
            p
        }
        f(1, 2)
        ",
        "expects 1 arguments",
    )]);
    assert_raises(&[(
        r"
        def f = fn p -> {
            p()
            p
        }
        f(1)
        ",
        "non-function",
    )]);
}

#[test]
fn a_variadic_parameter_collects_the_extra_arguments() {
    assert_values(&[
        (
            r"
            def f = fn (...rest) -> rest
            f(1, 2)
            ",
            "[1, 2]",
        ),
        (
            r"
            def f = fn (...rest) -> rest
            f()
            ",
            "[]",
        ),
        (
            r"
            def f = fn a, ...rest -> rest
            f(1)
            ",
            "[]",
        ),
        (
            r"
            def f = fn a, ...rest -> rest
            f(1, 2, 3)
            ",
            "[2, 3]",
        ),
        (
            r"
            def f = fn (p, ...rest) -> [p, rest]
            f(1, 2, 3)
            ",
            "[1, [2, 3]]",
        ),
        (
            r"
            def f = fn (a, b, ...rest) -> [a, b, rest]
            f(1, 2)
            ",
            "[1, 2, []]",
        ),
        (
            r"
            defn f(...args) -> args
            f(1, 2)
            ",
            "[1, 2]",
        ),
    ]);
}

#[test]
fn a_discarded_variadic_parameter_still_accepts_extra_arguments() {
    assert_values(&[(
        r"
        def f = fn a, ..._ -> a
        f(1, 2, 3)
        ",
        "1",
    )]);
}

#[test]
fn a_variadic_lambda_still_requires_its_fixed_parameters() {
    assert_raises(&[
        (
            r"
            def f = fn a, ...rest -> rest
            f()
            ",
            "expects at least 1 arguments",
        ),
        (
            r"
            def f = fn (a, b, ...rest) -> [a, b, rest]
            f(1)
            ",
            "expects at least 2 arguments",
        ),
    ]);
}

#[test]
fn a_discarded_parameter_still_counts_toward_arity() {
    assert_raises(&[
        ("(fn _, _ -> 1)(1)", "expects 2 arguments"),
        ("(fn f(_, _) -> 1)(1, 2, 3)", "expects 2 arguments"),
        ("(fn _, ..._ -> 1)()", "expects at least 1 arguments"),
        ("$($3)(1, 2)", "expects 3 arguments"),
    ]);
}

#[test]
fn a_named_lambda_is_checked_for_arity_like_any_other() {
    assert_raises(&[
        (
            r"
            defn f(a, ...rest) -> rest
            f()
            ",
            "expects at least 1 arguments",
        ),
        (
            r"
            defn f() -> 1
            f(1)
            ",
            "expects 0 arguments",
        ),
        // A recursive call is checked too.
        (
            r"
            defn f(n) -> f()
            f(1)
            ",
            "expects 1 arguments",
        ),
        (
            r"
            defn f(n) -> if n == 0: 0 else: f(n - 1, 0)
            f(2)
            ",
            "expects 1 arguments",
        ),
    ]);
}

#[test]
fn arity_is_checked_when_a_higher_order_function_calls_a_lambda() {
    assert_raises(&[
        ("call(fn x -> x, [])", "expects 1 arguments"),
        ("call($($2), [1])", "expects 2 arguments"),
        (
            "call(fn a, ...rest -> rest, [])",
            "expects at least 1 arguments",
        ),
        ("transform([1], fn a, b -> a)", "expects 2 arguments"),
        ("and_then(1, fn -> 1)", "expects 0 arguments"),
    ]);
    assert_values(&[
        ("call(fn _, ...rest -> rest, [1, 2, 3])", "[2, 3]"),
        ("try_call(fn x -> x, []).ok", "false"),
    ]);
}

// --- Abbreviated lambdas ---

#[test]
fn an_abbreviated_lambda_takes_as_many_arguments_as_its_highest_placeholder() {
    assert_values(&[
        ("$($1 + $2)(1, 2)", "3"),
        (
            r"
            def f = $($2)
            f(1, 2)
            ",
            "2",
        ),
        (
            r"
            def f = $(42)
            f()
            ",
            "42",
        ),
    ]);
    assert_raises(&[
        (
            r"
            def f = $($2)
            f(1)
            ",
            "expects 2 arguments",
        ),
        (
            r"
            def f = $($1)
            f(1, 2, 3)
            ",
            "expects 1 arguments",
        ),
        (
            r"
            def f = $(42)
            f(1, 2)
            ",
            "expects 0 arguments",
        ),
    ]);
}

#[test]
fn dollar_is_the_first_placeholder() {
    assert_values(&[
        ("$($ * 2)(5)", "10"),
        (
            r"
            def f = $($ + $1)
            f(5)
            ",
            "10",
        ),
        ("$($ + $2)(1, 2)", "3"),
    ]);
}

#[test]
fn double_dollar_is_the_rest_of_the_arguments() {
    assert_values(&[
        (
            r"
            def f = $($$)
            f(1, 2, 3)
            ",
            "[1, 2, 3]",
        ),
        (
            r"
            def f = $($$)
            f()
            ",
            "[]",
        ),
        ("$([$1, $$])(1, 2, 3)", "[1, [2, 3]]"),
    ]);
}

#[test]
fn every_placeholder_combination_binds_by_position() {
    assert_values(&[
        ("$([$1, $3])(1, 2, 3)", "[1, 3]"),
        ("$([$3, $1])(1, 2, 3)", "[3, 1]"),
        ("$($9)(1, 2, 3, 4, 5, 6, 7, 8, 9)", "9"),
        ("$([$2, $$])(1, 2, 3, 4)", "[2, [3, 4]]"),
        ("$([$2, $$])(1, 2)", "[2, []]"),
        ("$([$, $$])(1)", "[1, []]"),
        ("$([$$, $1])(1, 2)", "[[2], 1]"),
        // A placeholder may be used any number of times.
        ("$($ * $)(3)", "9"),
        ("$([$1, $, $1])(7)", "[7, 7, 7]"),
    ]);
    assert_raises(&[
        ("$($1 + $3)(1, 2)", "expects 3 arguments"),
        ("$($9)(1, 2, 3, 4, 5, 6, 7, 8)", "expects 9 arguments"),
        ("$([$2, $$])(1)", "expects at least 2 arguments"),
        ("$([$, $$])()", "expects at least 1 arguments"),
    ]);
}

#[test]
fn an_abbreviated_lambda_body_may_be_any_expression() {
    assert_values(&[
        (
            r"
            $(do {
                def y = $ + 1
                y * 2
            })(3)
            ",
            "8",
        ),
        (r#"$({[$1]: $2})("a", 1)"#, "{a: 1}"),
        ("$(if $2: $1 else: $3)(1, false, 3)", "3"),
        (r#"$($'<${$}>')("a")"#, r#""<a>""#),
        ("$(-$)(3)", "-3"),
        ("$(not $)(false)", "true"),
        ("$($1.a)({a: 5})", "5"),
        ("$($1[$2])([10, 20], 1)", "20"),
        ("$($1 @ $2())(3, fn v -> v * 2)", "6"),
        ("$(map $1 with $2)([1, 2], fn v -> v + 1)", "[2, 3]"),
        ("$($1 and $2)(1, 2)", "2"),
        ("$($1 or $2)(null, 2)", "2"),
    ]);
}

#[test]
fn an_abbreviated_lambda_captures_the_names_around_it() {
    assert_values(&[
        ("(fn k -> $($ + k))(10)(1)", "11"),
        (
            r"
            defn adder(k) -> $($ + k)
            adder(2)(3)
            ",
            "5",
        ),
        ("(fn ...rest -> $([$, rest]))(1, 2)(0)", "[0, [1, 2]]"),
        (
            r"
            def g = fn me() -> $(me)
            g()() == g
            ",
            "true",
        ),
        (
            r"
            def k = 3
            def f = $($ * k)
            f(2)
            ",
            "6",
        ),
    ]);
}

#[test]
fn a_nested_abbreviated_lambda_has_its_own_placeholders() {
    // The inner `$($1)` claims `$1`, so the outer takes no arguments.
    assert_values(&[(
        r"
        def f = $($($1))
        f()(7)
        ",
        "7",
    )]);
    assert_raises(&[(
        r"
        def f = $($($1))
        f(7)
        ",
        "expects 0 arguments",
    )]);
}

#[test]
fn a_placeholder_is_captured_by_a_plain_lambda_inside() {
    // A plain `fn` opens no placeholder scope: its `$1` is the enclosing one's.
    assert_values(&[
        (
            r"
            def f = $(fn -> $1)
            f(5)()
            ",
            "5",
        ),
        (
            r"
            def f = $(fn -> $)
            f(5)()
            ",
            "5",
        ),
        // Within a nested abbreviated lambda, it is that one's.
        ("$($(fn -> $1)(7)())()", "7"),
        // It counts toward the enclosing one's arity.
        ("$(fn -> $2)(1, 2)()", "2"),
        ("$(fn -> $$)(1, 2)()", "[1, 2]"),
        ("$(fn v -> v + $1)(1)(2)", "3"),
        ("$(fn -> fn -> $1)(4)()()", "4"),
    ]);
    assert_raises(&[("$(fn -> $2)(1)", "expects 2 arguments")]);
}

#[test]
fn each_placeholder_belongs_to_its_immediately_containing_abbreviated_lambda() {
    // Nesting abbreviated lambdas is inadvisable, but supported: an inner
    // lambda's placeholders shadow the outer one's entirely.
    assert_values(&[
        ("$($1 + $($1 * 10)(2))(3)", "23"),
        ("$([$1, $($1)(9)])(1)", "[1, 9]"),
        ("$($ + $($ * 2)(3))(1)", "7"),
        ("$([$$, $($$)(9)])(1, 2)", "[[1, 2], [9]]"),
        ("$($1 + $($$)(1, 2)[1])(10)", "12"),
        ("$($2 + $($1)(5))(0, 1)", "6"),
        // Three levels, each with its own `$1`.
        ("$($([$1, $($1 * 100)($1 + 1)])($1 * 10))(1)", "[10, 1100]"),
    ]);
}

#[test]
fn a_nested_abbreviated_lambda_has_its_own_arity() {
    // An outer lambda whose only placeholders are an inner one's takes none.
    assert_values(&[("$($($($1)))()()(4)", "4"), ("$($($1 + $2))()(1, 2)", "3")]);
    assert_raises(&[
        ("$($($2))()(1)", "expects 2 arguments"),
        ("$($($1))(1)", "expects 0 arguments"),
    ]);
}

#[test]
fn a_nested_abbreviated_lambda_captures_like_any_lambda() {
    assert_values(&[
        (
            r"
            def k = 5
            $($1 + $($1 + k)(1))(10)
            ",
            "16",
        ),
        ("$(transform($1, $($ + 1)))([1, 2])", "[2, 3]"),
    ]);
}

// --- Captures ---

#[test]
fn a_lambda_reads_the_names_around_it() {
    assert_values(&[
        (
            r"
            def x = 10
            def y = 20
            def f = fn -> x + y
            f()
            ",
            "30",
        ),
        (
            r"
            def base = 10
            defn f(x) -> x + base
            f(5)
            ",
            "15",
        ),
        (
            r"
            def a = 1
            def b = 2
            def c = 3
            def f = fn -> [a, b + c]
            f()
            ",
            "[1, 5]",
        ),
        (
            r"
            def cond = true
            def t = 1
            def e = 0
            def g = fn -> if cond: t else: e
            g()
            ",
            "1",
        ),
        (
            r"
            def cond = false
            def t = 1
            def e = 0
            def g = fn -> if cond: t else: e
            g()
            ",
            "0",
        ),
    ]);
}

#[test]
fn a_parameter_shadows_an_outer_name() {
    assert_values(&[
        (
            r"
            def p = 1
            def x = 2
            def f = fn p -> p + x
            f(5)
            ",
            "7",
        ),
        (
            r"
            def x = 1
            def f = fn x -> x + 1
            [f(10), x]
            ",
            "[11, 1]",
        ),
        (
            r"
            def rest = 99
            def f = fn (...rest) -> rest
            f(1, 2)
            ",
            "[1, 2]",
        ),
    ]);
}

#[test]
fn a_parameter_may_shadow_a_global() {
    assert_values(&[
        (
            r"
            def f = fn len -> len + 1
            f(5)
            ",
            "6",
        ),
        (
            r"
            def f = fn type -> type
            f(5)
            ",
            "5",
        ),
    ]);
}

#[test]
fn a_capture_is_the_value_at_the_closures_creation() {
    assert_values(&[
        (
            r"
            def outer = fn p -> fn -> p
            def f1 = outer(10)
            def f2 = outer(20)
            [f1(), f2()]
            ",
            "[10, 20]",
        ),
        (
            r"
            def make_adder = fn n -> fn x -> x + n
            def add5 = make_adder(5)
            make_adder(100)
            add5(10)
            ",
            "15",
        ),
        (
            r"
            defn build(n) -> if n <= 0: [] else: [fn -> n] @ (fn r -> r + build(n - 1))()
            def fns = build(3)
            [fns[0](), fns[1](), fns[2]()]
            ",
            "[3, 2, 1]",
        ),
    ]);
}

#[test]
fn closures_in_a_structure_keep_their_own_captures() {
    assert_values(&[
        (
            r"
            def x = 1
            def y = 2
            def arr = [fn -> x, fn -> y]
            [arr[0](), arr[1]()]
            ",
            "[1, 2]",
        ),
        (
            r"
            def x = 1
            def y = 2
            def outer = fn -> [fn -> x, fn -> x + y, fn -> y]
            def arr = outer()
            [arr[0](), arr[1](), arr[2]()]
            ",
            "[1, 3, 2]",
        ),
        (
            r"
            def x = 7
            def outer = fn -> [fn -> x, 0]
            outer()[0]()
            ",
            "7",
        ),
        (
            r"
            def a = [1]
            def b = [1]
            def f = fn -> {
                def m = {a: a, b: b}
                [m.a, m.b]
            }
            f()
            ",
            "[[1], [1]]",
        ),
        (
            r"
            def f = fn a -> do {
                def b = 2 + a
                {c: b, d: fn p -> p * a}
            }
            def r = f(3)
            [r.c, r.d(10)]
            ",
            "[5, 30]",
        ),
    ]);
}

#[test]
fn captures_pass_through_nested_lambdas() {
    assert_values(&[
        (
            r"
            def a = 1
            def f = fn -> fn -> a
            f()()
            ",
            "1",
        ),
        (
            r"
            def f = fn a -> fn b -> fn c -> a + b + c
            f(1)(2)(3)
            ",
            "6",
        ),
        (
            r"
            def g = 1
            def outer = fn a -> fn b -> fn c -> g + a + b + c
            outer(2)(3)(4)
            ",
            "10",
        ),
        (
            r"
            def outer = fn p -> fn q -> p + q
            outer(3)(4)
            ",
            "7",
        ),
        (
            r"
            def x = 1
            def outer = fn -> {
                def y = 2
                fn -> x + y
            }
            outer()()
            ",
            "3",
        ),
        (
            r"
            def x = 1
            def outer = fn p -> {
                def y = 2
                fn q -> x + y + p + q
            }
            outer(3)(4)
            ",
            "10",
        ),
        (
            r"
            def x = 1
            def outer = fn -> {
                def mid = fn -> fn -> x
                mid
            }
            outer()()()
            ",
            "1",
        ),
        (
            r"
            def outer = fn (...rest) -> fn -> rest
            outer(1, 2)()
            ",
            "[1, 2]",
        ),
    ]);
}

#[test]
fn a_nested_parameter_shadows_an_outer_one() {
    assert_values(&[
        (
            r"
            def x = 10
            def outer = fn -> fn x -> x
            outer()(20)
            ",
            "20",
        ),
        (
            r"
            def outer = fn -> {
                def x = 1
                fn x -> x
            }
            outer()(2)
            ",
            "2",
        ),
        (
            r"
            def outer = fn x -> fn x -> x
            outer(1)(3)
            ",
            "3",
        ),
        (
            r"
            def outer = fn x -> fn x -> fn -> x
            outer(1)(2)()
            ",
            "2",
        ),
        (
            r"
            def outer = fn (...rest) -> fn rest -> rest
            outer(1, 2)(9)
            ",
            "9",
        ),
        (
            r"
            def outer = fn (...rest) -> fn (...rest) -> rest
            outer(1, 2)(9, 8)
            ",
            "[9, 8]",
        ),
        (
            r"
            def outer = fn -> fn (...rest) -> rest
            outer()(1, 2)
            ",
            "[1, 2]",
        ),
        (
            r"
            def outer = fn -> fn -> {
                def x = 1
                x
            }
            outer()()
            ",
            "1",
        ),
    ]);
}

#[test]
fn a_lambda_chooses_among_closures_by_its_parameters() {
    assert_values(&[
        (
            r"
            def cond = true
            def x = 1
            def y = 2
            def outer = fn p -> if cond: fn q -> x + p + q else: fn q -> y + p + q
            outer(3)(4)
            ",
            "8",
        ),
        (
            r"
            def cond = false
            def x = 1
            def y = 2
            def outer = fn p -> if cond: fn q -> x + p + q else: fn q -> y + p + q
            outer(3)(4)
            ",
            "9",
        ),
        (
            r"
            def cond = true
            def x = 1
            def f = if cond: fn -> x + 1 else: fn -> x - 1
            f()
            ",
            "2",
        ),
    ]);
}

#[test]
fn a_lambda_captures_block_locals() {
    assert_values(&[
        (
            r"
            def x = 10
            def f = do {
                def y = 5
                fn -> x + y
            }
            f()
            ",
            "15",
        ),
        (
            r"
            def f = if true: do {
                def z = 99
                fn -> z
            } else: fn -> 0
            f()
            ",
            "99",
        ),
        (
            r"
            def f = fn base -> do {
                def scaled = base * 2
                fn -> scaled
            }
            f(5)()
            ",
            "10",
        ),
        (
            r"
            def outer = 5
            def f = fn -> do {
                def x = outer
                x
            }
            f()
            ",
            "5",
        ),
    ]);
}

#[test]
fn a_capture_passes_through_any_number_of_lambdas() {
    // Parameters are known only at runtime, so these are captured whatever the
    // optimization; `k` is a constant, and so may be hoisted instead.
    assert_values(&[
        ("(fn a -> fn -> fn -> fn -> fn -> a)(1)()()()()", "1"),
        (
            "(fn a -> fn b -> fn -> fn c -> fn -> [a, b, c])(1)(2)()(3)()",
            "[1, 2, 3]",
        ),
        (
            r"
            def k = 5
            (fn a -> fn -> fn -> [a, k])(1)()()
            ",
            "[1, 5]",
        ),
        (
            r"
            def k = 5
            (fn -> fn -> fn -> fn -> k)()()()()
            ",
            "5",
        ),
        // A middle lambda that uses the name itself, as well as passing it on.
        ("(fn a -> fn -> [a, fn -> a])(1)()[1]()", "1"),
    ]);
}

#[test]
fn many_captures_each_keep_their_own_value() {
    assert_values(&[
        (
            "(fn a, b, c, d, e, f, g, h -> fn -> [h, a, g, b, f, c, e, d])(1, 2, 3, 4, 5, 6, 7, 8)()",
            "[8, 1, 7, 2, 6, 3, 5, 4]",
        ),
        // Constants, which may be hoisted, interleaved with runtime values.
        (
            r"
            def k1 = 10
            def k2 = 20
            def k3 = 30
            (fn a, b -> fn -> [k2, a, k1, b, k3, a])(1, 2)()
            ",
            "[20, 1, 10, 2, 30, 1]",
        ),
        // Each nested lambda captures a different subset.
        (
            "(fn a, b, c -> fn -> [c, (fn -> [b, (fn -> a)()])()])(1, 2, 3)()",
            "[3, [2, 1]]",
        ),
    ]);
}

#[test]
fn a_lambda_captures_destructured_bindings() {
    assert_values(&[
        (
            r"
            def [a, b] = [1, 2]
            def f = fn -> a + b
            f()
            ",
            "3",
        ),
        (
            r"
            def {k, v: w} = {k: 1, v: 2}
            (fn -> [k, w])()
            ",
            "[1, 2]",
        ),
        (
            r"
            def f = fn p -> {
                def [h, ...t] = p
                fn -> [h, t]
            }
            f([1, 2, 3])()
            ",
            "[1, [2, 3]]",
        ),
        (
            r"
            def f = fn m -> {
                def {a} as whole = m
                fn -> fn -> [a, whole]
            }
            f({a: 1})()()
            ",
            "[1, {a: 1}]",
        ),
        (
            r"
            def f = fn p -> do {
                def [x1, x2] = p
                fn -> fn -> x1 * x2
            }
            f([3, 4])()()
            ",
            "12",
        ),
    ]);
}

#[test]
fn a_capture_may_supply_a_computed_key() {
    assert_values(&[
        (
            r#"
            def f = fn k -> fn v -> ({[k]: v})
            f("a")(1)
            "#,
            "{a: 1}",
        ),
        // A key used only in a destructuring pattern is still captured.
        (
            r#"
            def f = fn k, m -> fn -> {
                def {[k]: found} = m
                found
            }
            f("b", {b: 7})()
            "#,
            "7",
        ),
        (
            r#"
            def f = fn key -> fn m -> {
                def {[key]: v} = m
                v
            }
            f(1)({[1]: "one"})
            "#,
            r#""one""#,
        ),
        (
            r"
            def f = fn m, k -> fn -> m[k]
            f([5, 6], 1)()
            ",
            "6",
        ),
    ]);
}

#[test]
fn a_capture_is_read_in_every_kind_of_expression() {
    assert_values(&[
        ("(fn p -> fn -> not p)(false)()", "true"),
        ("(fn p -> fn -> -p)(2)()", "-2"),
        ("(fn p, q -> fn -> p and q)(1, 2)()", "2"),
        ("(fn p, q -> fn -> p or q)(null, 2)()", "2"),
        ("(fn p -> fn -> if p: 1 else: 2)(false)()", "2"),
        (r#"(fn n -> fn -> $'hi ${n}')("you")()"#, r#""hi you""#),
        ("(fn m -> fn -> m.a)({a: 1})()", "1"),
        ("(fn f -> fn -> f(1))(fn v -> v + 1)()", "2"),
        ("(fn f -> fn -> 2 @ f())(fn v -> v * 5)()", "10"),
        (
            "(fn k -> fn -> map [1, 2] with fn v -> v * k)(3)()",
            "[3, 6]",
        ),
        (
            "(fn lim -> fn -> filter [1, 2, 3] with fn v -> v > lim)(1)()",
            "[2, 3]",
        ),
        (
            "(fn k -> fn -> reduce [1, 2] with fn a, b -> a + b + k)(10)()",
            "13",
        ),
        (
            "(fn a, b -> fn -> {x: a, y: [b]})(1, 2)()",
            "{x: 1, y: [2]}",
        ),
    ]);
}

#[test]
fn a_capture_named_like_a_global_is_the_capture() {
    assert_values(&[
        (
            r"
            def len = 5
            def f = fn -> len
            f()
            ",
            "5",
        ),
        ("(fn print -> fn -> print(1))(fn v -> v + 1)()", "2"),
        ("(fn type -> fn -> fn -> type)(7)()()", "7"),
        (
            r#"
            def f = fn len(n) -> if n == 0: "done" else: len(n - 1)
            f(3)
            "#,
            r#""done""#,
        ),
        // Outside the shadowing scope, the name is the global again.
        (
            r"
            def f = fn type -> type
            [f(1), type(1)]
            ",
            r#"[1, "Int"]"#,
        ),
        (
            r"
            def g = do {
                def type = 5
                fn -> type
            }
            [g(), type(g)]
            ",
            r#"[5, "Function"]"#,
        ),
    ]);
}

#[test]
fn a_lambda_captures_a_name_from_the_enclosing_scope() {
    // `x` and `len` come from the scope the script is compiled in, known only
    // when the script runs.
    for (source, expected) in [
        (
            r"
            def f = fn -> fn -> x
            f()()
            ",
            Value::Int(7),
        ),
        (
            r"
            def f = fn x -> fn -> x
            f(2)()
            ",
            Value::Int(2),
        ),
        ("(fn -> len)()", Value::Int(3)),
        ("(fn -> fn a -> [a, x, len])()(1)", run("[1, 7, 3]")),
    ] {
        let tail = Script::new(source)
            .capture("x", Value::Int(7))
            .capture("len", Value::Int(3))
            .run();
        assert_eq!(tail, expected, "{source:?}");
    }
}

#[test]
fn a_name_a_lambda_binds_itself_is_not_taken_from_the_enclosing_scope() {
    // `x` is in the enclosing scope with no value supplied, so the script fails
    // to run if anything captures it.
    for (source, expected) in [
        ("(fn x -> x)(1)", "1"),
        ("(fn ...x -> x)(1)", "[1]"),
        ("is_function((fn x() -> x)())", "true"),
        (
            r"
            (fn -> {
                def x = 2
                x
            })()
            ",
            "2",
        ),
        ("(fn -> fn x -> x)()(3)", "3"),
        (
            r"
            (fn -> do {
                def [x] = [4]
                x
            })()
            ",
            "4",
        ),
    ] {
        let tail = Script::new(source).in_scope("x").run();
        assert_eq!(tail, run(expected), "{source:?} is {expected}");
    }
}

#[test]
fn a_nested_scope_may_rebind_a_parameter_or_self_name() {
    // A `do` block is its own scope, and a nested lambda its own function.
    assert_values(&[
        (
            r"
            (fn x -> do {
                def x = 2
                x
            })(1)
            ",
            "2",
        ),
        (
            r"
            (fn x -> do {
                def x = x * 10
                x
            })(2)
            ",
            "20",
        ),
        (
            r"
            (fn x -> fn -> {
                def x = x + 1
                x
            })(1)()
            ",
            "2",
        ),
        (
            r"
            (fn f() -> do {
                def f = 1
                f
            })()
            ",
            "1",
        ),
        (
            r"
            (fn x -> do {
                defn x() -> 5
                x()
            })(1)
            ",
            "5",
        ),
        ("is_function((fn f -> fn f() -> f)(1)())", "true"),
        ("(fn f() -> fn f -> f)()(5)", "5"),
        (
            r"
            def h = (fn -> {
                def g = 1
                fn g() -> g
            })()
            h() == h
            ",
            "true",
        ),
    ]);
}

#[test]
fn a_use_before_a_later_definition_reads_the_outer_name() {
    // A definition shadows only the uses after it.
    assert_values(&[
        (
            r"
            def x = 5
            def f = fn -> {
                x + 1
                def x = 2
                x
            }
            f()
            ",
            "2",
        ),
        (
            r"
            def x = 10
            def y = 20
            def f = fn -> {
                x + y
                def x = 3
                x
            }
            f()
            ",
            "3",
        ),
        (
            r"
            def x = 10
            def f = fn -> {
                def x = 1 + x
                x
            }
            f()
            ",
            "11",
        ),
        (
            r"
            def x = 2
            def f = fn -> {
                def y = x
                def x = 4
                x + y
            }
            f()
            ",
            "6",
        ),
        (
            r"
            def outer = fn p -> fn -> {
                p
                def p = 1
                p
            }
            outer(5)()
            ",
            "1",
        ),
        (
            r"
            def y = 10
            def outer = fn -> fn -> {
                y
                def y = 1
                y
            }
            outer()()
            ",
            "1",
        ),
        (
            r"
            def x = 100
            def f = fn -> do {
                x
                def x = 5
                x
            }
            [f(), x]
            ",
            "[5, 100]",
        ),
        (
            r"
            def x = 5
            def f = fn -> {
                def x = 7
                null
            }
            [f(), x]
            ",
            "[null, 5]",
        ),
    ]);
}

#[test]
fn a_closure_created_before_a_shadowing_definition_keeps_the_outer_name() {
    assert_values(&[
        (
            r"
            def x = 1
            def outer = fn -> {
                def inner = fn -> x
                def x = 2
                inner
            }
            outer()()
            ",
            "1",
        ),
        (
            r#"
            def later = "outer"
            def f = do {
                def g = fn -> later
                def later = "inner"
                g()
            }
            f
            "#,
            r#""outer""#,
        ),
    ]);
}

#[test]
fn a_parameter_may_be_used_only_in_a_definition() {
    assert_values(&[(
        r"
        def f = fn p -> {
            def x = p + 1
            x
        }
        f(10)
        ",
        "11",
    )]);
}

// --- `defn` ---

#[test]
fn defn_works_in_every_scope() {
    assert_values(&[
        (
            r"
            (fn -> {
                defn g(v) -> v * 2
                g(3)
            })()
            ",
            "6",
        ),
        (
            r"
            (fn -> do {
                defn g(v) -> v * 2
                g(4)
            })()
            ",
            "8",
        ),
        (
            r"
            (fn -> fn -> {
                defn g() -> 1
                g()
            })()()
            ",
            "1",
        ),
        (
            r"
            if true: do {
                defn g() -> 1
                g()
            } else: 0
            ",
            "1",
        ),
        (
            r"
            $(do {
                defn g(v) -> v * 2
                g($)
            })(5)
            ",
            "10",
        ),
        (
            r"
            {a: do {
                defn g() -> 1
                g
            }}.a()
            ",
            "1",
        ),
        (
            r"
            transform([1, 2], do {
                defn dbl(v) -> v * 2
                dbl
            })
            ",
            "[2, 4]",
        ),
        (
            r#"
            $'${do {
                defn g() -> 1
                g()
            }}'
            "#,
            r#""1""#,
        ),
    ]);
}

#[test]
fn defn_takes_every_parameter_shape() {
    assert_values(&[
        (
            r"
            defn f(_) -> 1
            f(2)
            ",
            "1",
        ),
        (
            r"
            defn f(_, ...rest) -> rest
            f(1, 2, 3)
            ",
            "[2, 3]",
        ),
        (
            r"
            defn f(..._) -> 1
            f(1, 2)
            ",
            "1",
        ),
        (
            r"
            defn f() -> {
                def a = 1
                a
            }
            f()
            ",
            "1",
        ),
        (
            r"
            defn f(a, _, ...rest) -> [a, rest]
            f(1, 2)
            ",
            "[1, []]",
        ),
    ]);
}

#[test]
fn a_defn_captures_the_enclosing_lambdas_parameters() {
    assert_values(&[(
        r"
        def sum_to = fn n -> {
            defn go(i, acc) -> if i > n: acc else: go(i + 1, acc + i)
            go(1, 0)
        }
        sum_to(4)
        ",
        "10",
    )]);
}

#[test]
fn a_defn_in_a_nested_scope_shadows_an_outer_name() {
    assert_values(&[
        (
            r"
            def f = 1
            do {
                defn f() -> 2
                f()
            }
            ",
            "2",
        ),
        (
            r#"
            def f = 1
            def g = do {
                defn f(n) -> if n == 0: "base" else: f(n - 1)
                f(2)
            }
            [g, f]
            "#,
            r#"["base", 1]"#,
        ),
    ]);
}

// --- Recursion ---

#[test]
fn a_named_lambda_recurses_by_its_name() {
    assert_values(&[
        (
            r"
            def fact = fn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)
            fact(5)
            ",
            "120",
        ),
        (
            r"
            defn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)
            fact(5)
            ",
            "120",
        ),
        (
            "(fn fact(n) -> if n <= 1: 1 else: n * fact(n - 1))(5)",
            "120",
        ),
        (
            r"
            defn call_it(g) -> g(5)
            call_it(fn fact(n) -> if n <= 1: 1 else: n * fact(n - 1))
            ",
            "120",
        ),
        (
            r"
            defn sum(n) -> {
                if n <= 0: 0
                else: n + sum(n - 1)
            }
            sum(4)
            ",
            "10",
        ),
        (
            r"
            do {
                defn double(x) -> x * 2
                double(6)
            }
            ",
            "12",
        ),
    ]);
}

#[test]
fn a_self_name_shadows_an_outer_name_throughout_the_body() {
    assert_values(&[
        (
            r"
            def f = 99
            def g = fn f(n) -> if n <= 0: 0 else: f(n - 1)
            g(3)
            ",
            "0",
        ),
        // Even where it looks like a plain value, `f` is the closure itself.
        (
            r"
            def f = 100
            def rec = fn f(n) -> if n <= 0: f else: f(n - 1)
            is_function(rec(3))
            ",
            "true",
        ),
    ]);
}

#[test]
fn a_recursive_lambda_also_captures_outer_names() {
    assert_values(&[
        (
            r"
            def base = 42
            def f = fn f(n) -> if n <= 0: base else: f(n - 1)
            f(3)
            ",
            "42",
        ),
        (
            r"
            def outer = fn -> fn f(x) -> if x <= 0: 0 else: f(x - 1)
            outer()(3)
            ",
            "0",
        ),
    ]);
}

#[test]
fn a_nested_lambda_may_call_its_enclosing_named_lambda() {
    assert_values(&[
        (
            r"
            def f = fn f(n) -> fn -> if n <= 0: 0 else: f(n - 1)()
            f(3)()
            ",
            "0",
        ),
        (
            r"
            def maker = fn -> {
                def outer = fn outer(arg) -> {
                    def inner = fn inner_arg -> outer(inner_arg)
                    if arg: arg else: inner
                }
                outer
            }
            maker()(false)(42)
            ",
            "42",
        ),
    ]);
}

#[test]
fn a_function_returning_itself_is_equal_to_itself() {
    assert_values(&[(
        r"
        defn outer() -> {
            defn inner() -> inner
            inner
        }
        def f = outer()
        is_function(f) and is_function(f()) and (f == f())
        ",
        "true",
    )]);
}

// --- Unusual self-reference ---
//
// A self-name is the closure itself: it can be returned, stored, compared,
// passed along, and captured by closures that outlive the call. Of dubious use,
// but it must work.

#[test]
fn a_lambda_may_return_itself() {
    assert_values(&[
        (
            r"
            def f = fn me() -> me
            [f() == f, f()()() == f]
            ",
            "[true, true]",
        ),
        (
            r"
            def f = fn me(x) -> me
            f(1)(2)(3) == f
            ",
            "true",
        ),
        (
            r"
            def f = fn f() -> f
            def g = f
            g() == f
            ",
            "true",
        ),
        // Recursing down to itself, in tail position.
        (
            r"
            def f = fn me(n) -> if n > 0: me(n - 1) else: me
            f(3) == f
            ",
            "true",
        ),
        ("is_function((fn me() -> me)())", "true"),
    ]);
}

#[test]
fn returning_itself_deeply_runs_in_bounded_depth() {
    let tail = Script::new(
        r"
        def f = fn me(n) -> if n == 0: me else: me(n - 1)
        f(100000) == f
        ",
    )
    .max_call_depth(100)
    .run();
    assert_eq!(tail, Value::Bool(true));
}

#[test]
fn an_inner_lambda_may_return_its_enclosing_lambda() {
    assert_values(&[
        (
            r"
            def outer = fn outer() -> fn -> outer
            outer()() == outer
            ",
            "true",
        ),
        (
            r"
            def f = fn me() -> fn -> fn -> me
            f()()() == f
            ",
            "true",
        ),
        // Each call makes a new inner closure, but they all lead back.
        (
            r"
            def a = fn a() -> fn b() -> a
            [a()() == a, a()()() == a()()()]
            ",
            "[true, false]",
        ),
        (
            r"def f = fn outer() -> fn middle() -> fn -> [outer, middle]
            def m = f()
            def pair = m()()
            [pair[0] == f, pair[1] == m]",
            "[true, true]",
        ),
    ]);
}

#[test]
fn a_self_name_captured_by_an_inner_lambda_shadows_an_outer_name() {
    assert_values(&[(
        r"
        def me = 5
        def f = fn me() -> fn -> me
        f()() == f
        ",
        "true",
    )]);
}

#[test]
fn a_nested_lambda_reusing_the_self_name_refers_to_itself() {
    assert_values(&[(
        r"
        def f = fn g() -> fn g() -> g
        def inner = f()
        [inner() == inner, inner == f]
        ",
        "[true, false]",
    )]);
}

#[test]
fn a_lambda_may_store_itself_in_a_structure() {
    assert_values(&[
        (
            r"
            def f = fn me() -> [me, me]
            [f()[0] == f, f()[1]()[0] == f]
            ",
            "[true, true]",
        ),
        (
            r"
            def f = fn me() -> {self: me}
            f().self().self == f
            ",
            "true",
        ),
        (
            r"
            def counter = fn count(n) -> {value: n, next: fn -> count(n + 1)}
            counter(0).next().next().value
            ",
            "2",
        ),
        (
            r"
            defn make(n) -> {n: n, next: fn -> make(n + 1)}
            make(0).next().next().next().n
            ",
            "3",
        ),
    ]);
}

#[test]
fn a_lambda_may_compare_and_pass_itself() {
    assert_values(&[
        (
            r"
            def f = fn me(other) -> other == me
            [f(f), f(1)]
            ",
            "[true, false]",
        ),
        (
            r#"
            def f = fn me(x) -> if x == me: "self" else: x(me)
            [f(f), f(fn g -> is_function(g))]
            "#,
            r#"["self", true]"#,
        ),
    ]);
}

#[test]
fn recursion_works_through_a_nested_named_lambda() {
    // `odd` exists only inside `even`, and calls back out to it.
    assert_values(&[(
        r"def even = fn even(n) -> if n == 0: true else: (fn odd(m) -> if m == 0: false else: even(m - 1))(n - 1)
        [even(10), even(7)]",
        "[true, false]",
    )]);
}

#[test]
fn a_named_lambda_recurses_from_nested_lambdas_at_any_depth() {
    assert_values(&[
        (
            r"
            defn f(n) -> if n == 0: 0 else: (fn -> (fn -> f(n - 1))())()
            f(5)
            ",
            "0",
        ),
        (
            r"
            defn f(n) -> if n == 0: 0 else: $(f($))(n - 1)
            f(5)
            ",
            "0",
        ),
    ]);
}

#[test]
fn a_named_lambda_recurses_through_a_higher_order_function() {
    assert_values(&[
        (
            r"
            defn sum(n) -> if n == 0: 0 else: n + transform([n - 1], sum)[0]
            sum(4)
            ",
            "10",
        ),
        (
            r"defn depth(t) -> if is_array(t): 1 + fold(transform(t, depth), fn a, b -> if a > b: a else: b, 0) else: 0
            depth([[1], [[2]]])",
            "3",
        ),
        (
            r"
            defn f(n) -> if n == 0: 0 else: 1 + call(f, [n - 1])
            f(3)
            ",
            "3",
        ),
    ]);
}

#[test]
fn a_variadic_lambda_recurses_over_its_arguments() {
    assert_values(&[(
        r"defn total(...xs) -> if xs == []: 0 else: do {
            def [head, ...tail] = xs
            head + call(total, tail)
        }
        total(1, 2, 3)",
        "6",
    )]);
}

#[test]
fn anonymous_lambdas_may_recurse_by_self_application() {
    assert_values(&[
        (
            r"def fact = (fn f -> fn n -> if n <= 1: 1 else: n * f(f)(n - 1))(fn f -> fn n -> if n <= 1: 1 else: n * f(f)(n - 1))
            fact(5)",
            "120",
        ),
        (
            r"
            def u = fn self -> self(self)
            def f = fn me(s) -> fn -> s
            is_function(u(f))
            ",
            "true",
        ),
    ]);
}

// --- Higher-order use ---

#[test]
fn a_lambda_is_a_callback() {
    assert_values(&[
        ("transform([1, 2, 3], fn x -> x * 2)", "[2, 4, 6]"),
        ("transform([1, 2, 3], $($ * 2))", "[2, 4, 6]"),
        ("select([1, 2, 3, 4], fn x -> x > 2)", "[3, 4]"),
        ("fold([1, 2, 3], fn acc, x -> acc + x, 0)", "6"),
        ("and_then(5, fn x -> x + 1)", "6"),
        ("and_then(null, fn x -> x + 1)", "null"),
        ("or_else(5, fn -> 99)", "5"),
        ("or_else(null, fn -> 99)", "99"),
    ]);
    assert_raises(&[("fold([1, 2, 3], 0, fn acc, x -> acc + x)", "argument 2")]);
}

#[test]
fn call_applies_a_lambda_to_an_argument_array() {
    assert_values(&[
        ("call(fn -> 42)", "42"),
        ("call(fn x, y -> x + y, [3, 4])", "7"),
    ]);
    assert_raises(&[
        (
            "call(5, [1])",
            "Function call requires Function as argument 1, got Int",
        ),
        (
            "call(fn -> null, 5)",
            "Function call requires Array as argument 2, got Int",
        ),
    ]);
}

#[test]
fn try_call_reports_a_lambdas_success_or_error() {
    assert_values(&[
        ("try_call(fn -> 42)", "{ok: true, value: 42}"),
        ("try_call(fn x -> x + 1, [5])", "{ok: true, value: 6}"),
        (
            r"
            def r = try_call(fn -> 1 / 0)
            [r.ok, r.error]
            ",
            r#"[false, "Division by zero"]"#,
        ),
        (
            r#"
            def r = try_call(fn -> error("boom"))
            [r.ok, r.error]
            "#,
            r#"[false, "boom"]"#,
        ),
    ]);
}

#[test]
fn a_lambda_may_return_a_lambda() {
    assert_values(&[
        (
            r"
            def add = fn a -> fn b -> a + b
            add(3)(4)
            ",
            "7",
        ),
        ("(fn x -> fn y -> x + y)(3)(4)", "7"),
        (
            r"
            def f = fn -> fn -> 42
            is_function(f())
            ",
            "true",
        ),
    ]);
}

#[test]
fn a_lambda_works_wherever_an_expression_does() {
    assert_values(&[
        (
            r"
            def m = {inc: fn x -> x + 1, dec: fn x -> x - 1}
            [m.inc(10), m.dec(10)]
            ",
            "[11, 9]",
        ),
        ("5 @ (fn x -> x * 2)()", "10"),
        ("5 @ (fn x, y -> x + y)(10)", "15"),
        (
            r"
            def f = fn x -> x * 2
            $'result: ${f(21)}'
            ",
            r#""result: 42""#,
        ),
    ]);
}

// --- Values ---

#[test]
fn a_lambda_is_a_function_value() {
    assert_values(&[
        ("type(fn -> 1)", r#""Function""#),
        ("is_function(fn -> 1)", "true"),
        ("is_function($(1))", "true"),
    ]);
    let rendered = run("to_string(fn x -> x + 1)");
    assert!(
        matches!(&rendered, Value::String(s) if !s.is_empty()),
        "a function renders as some non-empty String: {rendered:?}"
    );
}

#[test]
fn a_function_is_truthy() {
    assert_values(&[
        ("not (fn -> false)", "false"),
        ("(fn -> null) and 2", "2"),
        ("if $(false): 1 else: 2", "1"),
    ]);
}

#[test]
fn a_function_is_not_an_operand_of_ordering_or_arithmetic() {
    assert_raises(&[
        (
            r"
            def f = fn -> 1
            f < f
            ",
            "Function is not orderable",
        ),
        ("(fn -> 1) + 1", "Function + Int"),
        ("-(fn -> 1)", "Function"),
    ]);
}

#[test]
fn a_closure_equals_only_itself() {
    assert_values(&[
        (
            r"
            def f = fn -> 1
            [f == f, equal(f, f)]
            ",
            "[true, true]",
        ),
        (
            r"
            def f = fn -> 1
            def g = fn -> 1
            [f == g, equal(f, g)]
            ",
            "[false, false]",
        ),
        (
            r"
            def f = fn -> 1
            [f != f, f != fn -> 1]
            ",
            "[false, true]",
        ),
        ("[(fn -> 1) == 1, (fn -> 1) == null]", "[false, false]"),
        (
            r"
            def f = $(1)
            [f == f, $(1) == $(1)]
            ",
            "[true, false]",
        ),
    ]);
}

#[test]
fn each_evaluation_of_a_lambda_makes_a_distinct_closure() {
    // Whatever it captures, and even if it captures nothing.
    assert_values(&[
        (
            r"
            def make = fn -> fn -> 1
            make() == make()
            ",
            "false",
        ),
        (
            r"
            def make = fn p -> fn -> p
            make(1) == make(1)
            ",
            "false",
        ),
        (
            r"
            def make = fn -> $($)
            make() == make()
            ",
            "false",
        ),
        (
            r"
            def fs = transform([1, 2], fn _ -> fn -> 1)
            fs[0] == fs[1]
            ",
            "false",
        ),
        ("(fn -> 1) == (fn -> 1)", "false"),
    ]);
}

#[test]
fn a_closure_equals_itself_wherever_it_is_stored() {
    assert_values(&[(
        r"
        def f = fn -> 1
        [[f] == [f], {a: f} == {a: f}, id(f) == f, transform([f], fn g -> g)[0] == f]
        ",
        "[true, true, true, true]",
    )]);
}

// --- Runtime errors ---

#[test]
fn calling_a_non_function_raises() {
    assert_raises(&[
        ("5()", "Int"),
        ("null()", "Null"),
        (r#""hi"()"#, "String"),
        ("[1, 2, 3]()", "Array"),
    ]);
}

#[test]
fn an_error_in_a_lambda_propagates_to_its_caller() {
    assert_raises(&[
        (
            r"
            def f = fn -> 1 / 0
            f()
            ",
            "Division by zero",
        ),
        ("(fn -> 1 / 0)()", "Division by zero"),
        (
            r#"
            def f = fn -> error("boom")
            f()
            "#,
            "boom",
        ),
        ("(1 / 0)(5)", "Division by zero"),
        (
            r"
            def f = fn x, y -> x + y
            f(null + true, 1 / 0)
            ",
            "Null + Bool",
        ),
        (
            r"
            def f = fn x, y -> x + y
            f(1 / 0, null + true)
            ",
            "Division by zero",
        ),
    ]);
}

#[test]
fn an_error_propagates_out_of_nested_lambdas() {
    assert_raises(&[
        (
            r"
            def f = fn -> (fn -> (fn -> 1 / 0)())()
            f()
            ",
            "Division by zero",
        ),
        ("transform([1, 0], fn v -> 1 / v)", "Division by zero"),
        (
            r"
            defn f(n) -> if n == 0: 1 / 0 else: f(n - 1)
            f(3)
            ",
            "Division by zero",
        ),
        ("(fn -> 1)()()", "Int"),
    ]);
}

#[test]
fn a_lambdas_body_errors_only_when_called() {
    assert_values(&[
        (
            r"
            def f = fn -> 1 / 0
            5
            ",
            "5",
        ),
        ("if false: (fn -> 1 / 0)() else: 1", "1"),
        (
            r"
            def f = fn -> fn -> 1 / 0
            is_function(f())
            ",
            "true",
        ),
    ]);
}

#[test]
fn a_function_is_not_a_map_key() {
    assert_raises(&[(r#"{[fn -> 1]: "x"}"#, "Function")]);
}

// --- Compile errors ---

#[test]
fn an_unbound_name_is_a_compile_error_wherever_it_appears() {
    assert_compile_errors(&[
        (
            r"
            def f = fn -> missing
            1
            ",
            "`missing` is not defined",
        ),
        (
            r"
            def outer = fn -> fn -> missing
            1
            ",
            "`missing` is not defined",
        ),
        (
            "if false: (fn -> missing) else: 2",
            "`missing` is not defined",
        ),
        ("if false: missing else: 2", "`missing` is not defined"),
    ]);
}

#[test]
fn an_unbound_name_is_a_compile_error_in_any_lambda_form() {
    assert_compile_errors(&[
        (
            r"
            def f = $($ + missing)
            1
            ",
            "`missing` is not defined",
        ),
        (
            r"
            defn f() -> missing
            1
            ",
            "`missing` is not defined",
        ),
        (
            r"
            def f = fn ...rest -> missing
            1
            ",
            "`missing` is not defined",
        ),
        (
            r"
            def f = fn -> {
                missing
                1
            }
            ",
            "`missing` is not defined",
        ),
        (
            r"
            def f = fn -> {
                def y = missing
                y
            }
            ",
            "`missing` is not defined",
        ),
        ("def f = fn -> missing(1)", "`missing` is not defined"),
        ("def f = fn -> $'${missing}'", "`missing` is not defined"),
        (
            r"
            def f = fn m -> {
                def {[missing]: v} = m
                v
            }
            ",
            "`missing` is not defined",
        ),
        ("{[(fn -> missing)()]: 1}", "`missing` is not defined"),
        (
            "def f = fn -> if true: 1 else: fn -> missing",
            "`missing` is not defined",
        ),
    ]);
}

#[test]
fn a_lambdas_bindings_are_not_visible_outside_it() {
    assert_compile_errors(&[
        (
            r"
            (fn p -> p)(1)
            p
            ",
            "`p` is not defined",
        ),
        (
            r"
            def g = fn f() -> 1
            f()
            ",
            "`f` is not defined",
        ),
        (
            r"
            def f = fn -> {
                def local = 1
                local
            }
            local
            ",
            "`local` is not defined",
        ),
        (
            r"
            def f = fn -> {
                def g = fn y -> y
                y
            }
            ",
            "`y` is not defined",
        ),
        ("[fn a -> a, fn -> a]", "`a` is not defined"),
        (
            r"
            def f = fn ...rest -> rest
            rest
            ",
            "`rest` is not defined",
        ),
    ]);
}

#[test]
fn a_name_defined_later_is_not_yet_bound() {
    assert_compile_errors(&[
        (
            r"
            def f = fn -> later
            def later = 5
            f()
            ",
            "`later` is not defined",
        ),
        (
            r"
            def outer = fn -> {
                fn -> z
                def z = 42
                null
            }
            1
            ",
            "`z` is not defined",
        ),
        (
            r"
            do {
                x
                def x = 5
                x
            }
            ",
            "`x` is not defined",
        ),
        // So two top-level functions cannot call each other.
        (
            r#"defn a(n) -> if n <= 0: "done" else: b(n - 1)
            defn b(n) -> if n <= 0: "done" else: a(n - 1)
            a(5)"#,
            "`b` is not defined",
        ),
    ]);
}

#[test]
fn an_abbreviated_lambda_cannot_recurse_by_name() {
    // It has no self-name; the `def` it is bound to is not yet defined.
    assert_compile_errors(&[(
        r"
        def f = $(if $ <= 1: 1 else: $ * f($ - 1))
        f(5)
        ",
        "`f` is not defined",
    )]);
}

#[test]
fn a_name_bound_twice_in_a_lambdas_scope_is_a_compile_error() {
    assert_compile_errors(&[
        ("def f = fn x, x -> x", "`x` is already bound"),
        ("def f = fn x, ...x -> x", "`x` is already bound"),
        ("def g = fn f(f) -> null", "`f` is already bound"),
        ("def g = fn f(...f) -> null", "`f` is already bound"),
        (
            r"
            def f = fn x -> {
                def x = 2
                null
            }
            ",
            "`x` is already bound",
        ),
        (
            r"
            def f = fn (...rest) -> {
                def rest = 2
                null
            }
            ",
            "`rest` is already bound",
        ),
        (
            r"
            def g = fn f() -> {
                def f = 1
                null
            }
            ",
            "`f` is already bound",
        ),
    ]);
}

#[test]
fn every_way_of_binding_a_name_twice_in_a_lambda_is_a_compile_error() {
    assert_compile_errors(&[
        // Among parameters, however far apart.
        ("def f = fn a, b, a -> a", "`a` is already bound"),
        ("def g = fn f(x, f) -> 1", "`f` is already bound"),
        ("def g = fn f(a, ...f) -> 1", "`f` is already bound"),
        ("defn f(a, a) -> a", "`a` is already bound"),
        ("defn f(f) -> 1", "`f` is already bound"),
        // Among the body's own definitions.
        (
            r"
            def f = fn -> {
                def y = 1
                def y = 2
                y
            }
            ",
            "`y` is already bound",
        ),
        (
            r"
            def f = fn -> {
                defn g() -> 1
                defn g() -> 2
                g()
            }
            ",
            "`g` is already bound",
        ),
        // A body definition against a parameter or the self-name.
        (
            r"
            def f = fn x -> {
                def [x, y] = [1, 2]
                y
            }
            ",
            "`x` is already bound",
        ),
        (
            r"
            def f = fn x -> {
                def {x} = {x: 1}
                x
            }
            ",
            "`x` is already bound",
        ),
        (
            r"
            def h = fn f(x) -> {
                def [a, f] = [1, 2]
                a
            }
            ",
            "`f` is already bound",
        ),
        (
            r"
            def f = fn g -> {
                defn g() -> 1
                g()
            }
            ",
            "`g` is already bound",
        ),
        (
            r"
            def h = fn f() -> {
                defn f() -> 1
                f
            }
            ",
            "`f` is already bound",
        ),
        // Top-level `defn`s are definitions in one scope, too.
        (
            r"
            defn f() -> 1
            defn f() -> 2
            f()
            ",
            "`f` is already bound",
        ),
        // In a nested lambda, even one never created.
        ("def f = fn -> fn x, x -> x", "`x` is already bound"),
        (
            r"
            if false: (fn y -> {
                def y = 1
                y
            }) else: 1
            ",
            "`y` is already bound",
        ),
    ]);
}

#[test]
fn a_lambda_block_must_end_in_an_expression() {
    assert_compile_errors(&[
        (
            r"
            def f = fn -> {
                def x = 1
            }
            ",
            "must end with an expression",
        ),
        (
            r"
            def f = fn x -> {
                def x = 1
            }
            ",
            "must end with an expression",
        ),
    ]);
}

#[test]
fn a_discard_cannot_be_read() {
    assert_compile_errors(&[("def f = fn _, b -> _", "not defined")]);
}

#[test]
fn a_placeholder_is_only_valid_in_an_abbreviated_lambda() {
    for source in ["$", "$1", "fn -> $1", "def f = fn -> $$"] {
        assert!(
            !Script::new(source).compile_errors().is_empty(),
            "{source:?} is rejected"
        );
    }
}
