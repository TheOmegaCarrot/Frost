//! The generated Functions globals, from Frost source: `inv`, `curry`, `bcurry`,
//! `collect`, `spread`, `rev_args`, `tap`, `const`, and `compose`.
//!
//! These are written in Frost (see the runtime's `generated` module), so the
//! function each returns tail-calls the function it wraps, wherever that call is
//! the last thing it does. The tail-call cases check that under a small call-depth
//! limit, with recursion far deeper than the limit.
//!
//! The harness runs every case under every optimization permutation, so a call
//! over literals is checked both folded and at run time.

mod common;

use common::{Script, raises, run};
use frost_runtime::Value;

/// Assert each `source` runs to the value of the Frost expression `expected`.
fn assert_values(cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(run(source), run(expected), "{source:?} is {expected}");
    }
}

/// Assert each `source` raises exactly what `equivalent` raises.
fn assert_raises_as(cases: &[(&str, &str)]) {
    for (source, equivalent) in cases {
        assert_eq!(
            raises(source),
            raises(equivalent),
            "{source:?} raises as {equivalent:?} does"
        );
    }
}

/// Assert each call in `calls` (source, argument count) raises `function`'s
/// arity error, where `expects` is how many arguments it takes, such as `1` or
/// `at least 2`.
fn assert_arity(function: &str, expects: &str, calls: &[(&str, usize)]) {
    for (source, argc) in calls {
        assert_eq!(
            raises(source),
            format!("Function {function} expects {expects} arguments, but was called with {argc}"),
            "{source:?}"
        );
    }
}

/// Run `source` with the call depth limited far below the recursion it does, so
/// it completes only if its calls through the global are tail calls.
fn run_deep(source: &str) -> Value {
    Script::new(source).max_call_depth(32).run()
}

// --- inv ---

#[test]
fn inv_negates_what_its_function_returns() {
    assert_values(&[
        ("inv(fn x -> x > 1)(2)", "false"),
        ("inv(fn x -> x > 1)(0)", "true"),
        // Negation follows truthiness: only Null and False are falsy.
        ("inv(fn -> null)()", "true"),
        ("inv(fn -> 0)()", "false"),
        ("inv(fn -> '')()", "false"),
        // Every argument is passed through.
        ("inv(fn a, b -> a == b)(1, 1)", "false"),
        ("inv(fn ...xs -> xs == [])()", "false"),
        ("filter [1, 2, 3, 4] with inv(fn x -> x > 2)", "[1, 2]"),
    ]);
}

#[test]
fn inv_raises_when_its_function_cannot_be_called() {
    assert_raises_as(&[
        ("inv(1)()", "call(1, [])"),
        ("inv(fn x -> x)()", "(fn x -> x)()"),
    ]);
}

#[test]
fn inv_takes_exactly_one_argument() {
    assert_arity("inv", "1", &[("inv()", 0), ("inv(id, id)", 2)]);
}

// --- curry, bcurry ---

#[test]
fn curry_puts_its_arguments_first() {
    assert_values(&[
        ("curry(minus, 10)(3)", "7"),
        ("curry(minus)(10, 3)", "7"),
        ("curry(minus, 10, 3)()", "7"),
        ("curry(collect, 1, 2)(3, 4)", "[1, 2, 3, 4]"),
        ("curry(collect)()", "[]"),
    ]);
}

#[test]
fn bcurry_puts_its_arguments_last() {
    assert_values(&[
        ("bcurry(minus, 10)(3)", "-7"),
        ("bcurry(minus)(10, 3)", "7"),
        ("bcurry(collect, 1, 2)(3, 4)", "[3, 4, 1, 2]"),
        ("bcurry(collect)()", "[]"),
    ]);
}

#[test]
fn a_curried_function_may_be_called_repeatedly() {
    let source = r"
        def add5 = curry(plus, 5)
        def halve = bcurry(divide, 2)
        [add5(1), add5(2), halve(10), halve(4)]
    ";
    assert_values(&[(source, "[6, 7, 5, 2]")]);
}

#[test]
fn a_curried_function_raises_as_its_function_does() {
    assert_raises_as(&[
        ("curry(minus, 1)('a')", "minus(1, 'a')"),
        ("bcurry(minus, 1)('a')", "minus('a', 1)"),
        ("curry(minus, 1, 2)(3)", "minus(1, 2, 3)"),
        ("curry(1)()", "call(1, [])"),
        ("bcurry(1)()", "call(1, [])"),
    ]);
}

#[test]
fn curry_and_bcurry_take_at_least_one_argument() {
    assert_arity("curry", "at least 1", &[("curry()", 0)]);
    assert_arity("bcurry", "at least 1", &[("bcurry()", 0)]);
}

#[test]
fn curried_calls_are_tail_calls() {
    let source = r"
        defn countdown(n) -> if n == 0: 'done' else: curry(countdown)(n - 1)
        countdown(1000)
    ";
    assert_eq!(run_deep(source), Value::from("done"));
    let source = r"
        defn countdown(n) -> if n == 0: 'done' else: bcurry(countdown, n - 1)()
        countdown(1000)
    ";
    assert_eq!(run_deep(source), Value::from("done"));
}

// --- collect ---

#[test]
fn collect_returns_its_arguments_as_an_array() {
    assert_values(&[
        ("collect()", "[]"),
        ("collect(1)", "[1]"),
        ("collect(1, [2], null, 'x')", "[1, [2], null, 'x']"),
    ]);
}

// --- spread ---

#[test]
fn spread_calls_its_function_with_an_arrays_elements() {
    assert_values(&[
        ("spread(minus)([10, 3])", "7"),
        ("spread(collect)([])", "[]"),
        ("spread(collect)([[1], 2])", "[[1], 2]"),
        (
            "map [[1, 2], [3, 4]] with spread(fn a, b -> a * b)",
            "[2, 12]",
        ),
    ]);
}

#[test]
fn a_spread_function_raises_as_its_call_does() {
    assert_raises_as(&[
        ("spread(minus)([1])", "minus(1)"),
        ("spread(minus)(5)", "call(minus, 5)"),
        ("spread(1)([])", "call(1, [])"),
    ]);
}

#[test]
fn spread_takes_exactly_one_argument() {
    assert_arity("spread", "1", &[("spread()", 0), ("spread(id, id)", 2)]);
}

#[test]
fn spread_calls_are_tail_calls() {
    let source = r"
        defn countdown(n) -> if n == 0: 'done' else: spread(countdown)([n - 1])
        countdown(1000)
    ";
    assert_eq!(run_deep(source), Value::from("done"));
}

// --- rev_args ---

#[test]
fn rev_args_reverses_the_arguments() {
    assert_values(&[
        ("rev_args(minus)(3, 10)", "7"),
        ("rev_args(collect)(1, 2, 3)", "[3, 2, 1]"),
        ("rev_args(collect)(1)", "[1]"),
        ("rev_args(collect)()", "[]"),
    ]);
}

#[test]
fn a_reversed_function_raises_as_its_function_does() {
    assert_raises_as(&[
        ("rev_args(minus)(1)", "minus(1)"),
        ("rev_args(minus)('a', 1)", "minus(1, 'a')"),
        ("rev_args(1)()", "call(1, [])"),
    ]);
}

#[test]
fn rev_args_takes_exactly_one_argument() {
    assert_arity(
        "rev_args",
        "1",
        &[("rev_args()", 0), ("rev_args(id, id)", 2)],
    );
}

#[test]
fn reversed_calls_are_tail_calls() {
    let source = r"
        defn countdown(n, label) -> if n == 0: label else: rev_args(countdown)(label, n - 1)
        countdown(1000, 'done')
    ";
    assert_eq!(run_deep(source), Value::from("done"));
}

// --- tap ---

#[test]
fn tap_calls_its_function_and_returns_its_value() {
    assert_values(&[
        ("tap(5, fn x -> x * 100)", "5"),
        ("tap([1], fn x -> null)", "[1]"),
        ("[1, 2] @ tap(fn xs -> 'ignored')", "[1, 2]"),
    ]);
    assert_eq!(
        Script::new("5 @ tap(print) @ tap(fn x -> print(x + 1))").printed(),
        ["5", "6"]
    );
}

#[test]
fn tap_raises_as_its_function_does() {
    assert_raises_as(&[
        ("tap(1, fn x -> x + 'a')", "1 + 'a'"),
        ("tap(1, 2)", "call(2, [1])"),
    ]);
}

#[test]
fn tap_takes_exactly_two_arguments() {
    assert_arity(
        "tap",
        "2",
        &[("tap()", 0), ("tap(1)", 1), ("tap(1, id, id)", 3)],
    );
}

// --- const ---

#[test]
fn const_returns_a_function_that_always_returns_its_value() {
    assert_values(&[
        ("const(7)()", "7"),
        ("const(7)(1, 2, 3)", "7"),
        ("const(null)(1)", "null"),
        ("const([1])('x')", "[1]"),
        ("map [1, 2, 3] with const(0)", "[0, 0, 0]"),
    ]);
}

#[test]
fn const_takes_exactly_one_argument() {
    assert_arity("const", "1", &[("const()", 0), ("const(1, 2)", 2)]);
}

// --- compose ---

#[test]
fn compose_applies_its_functions_left_to_right() {
    assert_values(&[
        ("compose(fn x -> x + 1, fn x -> x * 2)(5)", "12"),
        ("compose(fn x -> x * 2, fn x -> x + 1)(5)", "11"),
        (
            "compose(fn x -> x + 1, fn x -> x * 2, fn x -> x - 3)(5)",
            "9",
        ),
        (
            "compose(fn x -> x + 1, fn x -> x * 2, fn x -> x - 3, fn x -> [x])(5)",
            "[9]",
        ),
        // The first function receives every argument; the rest, one result each.
        ("compose(plus, fn x -> x * 10)(1, 2)", "30"),
        ("compose(collect, fn xs -> xs + [0])()", "[0]"),
    ]);
}

#[test]
fn compose_requires_functions() {
    for (source, got) in [
        ("compose(1, id)", "Int"),
        ("compose(id, 'x')", "String"),
        ("compose(id, id, null)", "Null"),
        ("compose(id, id, id, [id])", "Array"),
    ] {
        assert_eq!(
            raises(source),
            format!("Failed assertion: Compose requires functions, got {got}"),
            "{source:?}"
        );
    }
}

#[test]
fn a_composed_function_raises_as_its_functions_do() {
    assert_raises_as(&[
        ("compose(minus, id)(1)", "minus(1)"),
        ("compose(id, fn x -> x + 'a')(1)", "1 + 'a'"),
    ]);
}

#[test]
fn compose_takes_at_least_two_arguments() {
    assert_arity(
        "compose",
        "at least 2",
        &[("compose()", 0), ("compose(id)", 1)],
    );
}

#[test]
fn composed_calls_end_in_a_tail_call() {
    let source = r"
        defn countdown(n) -> if n == 0: 'done' else: compose(fn m -> m - 1, countdown)(n)
        countdown(1000)
    ";
    assert_eq!(run_deep(source), Value::from("done"));
    let source = r"
        defn countdown(n) -> if n == 0: 'done' else: compose(id, id, fn m -> m - 1, countdown)(n)
        countdown(1000)
    ";
    assert_eq!(run_deep(source), Value::from("done"));
}

// --- Generated functions as values ---

/// The frame names in the backtrace of the error `source` raises.
fn trace(source: &str) -> Vec<Value> {
    let source = format!("try_call(fn -> {source}).trace");
    match run(&source) {
        Value::Array(names) => names.iter().cloned().collect(),
        other => panic!("{source:?} should give a trace, but gave {other:?}"),
    }
}

#[test]
fn a_returned_function_is_named_for_its_global() {
    // Most returned functions take any arguments, but `spread_fn` takes one.
    assert_eq!(
        raises("spread(minus)()"),
        "Function spread_fn expects 1 arguments, but was called with 0"
    );
    // A returned function shows in a backtrace while it awaits a call it makes.
    for (source, name) in [
        ("inv(fn -> error('boom'))()", "inv_fn"),
        ("compose(fn -> error('boom'), id)()", "compose_fn"),
    ] {
        assert!(
            trace(source).contains(&Value::from(name)),
            "{source:?} has {name} in its backtrace: {:?}",
            trace(source)
        );
    }
}

#[test]
fn generated_globals_are_functions() {
    // Each arity test above also shows that errors name the global.
    for name in [
        "inv", "curry", "bcurry", "collect", "spread", "rev_args", "tap", "const", "compose",
    ] {
        assert_eq!(
            run(&format!("is_function({name})")),
            Value::Bool(true),
            "{name}"
        );
    }
}
