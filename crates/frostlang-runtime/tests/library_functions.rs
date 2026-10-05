//! The Functions globals, from Frost source: calling and error handling with
//! `call`, `try_call`, and `error`; the Null combinators `and_then` and
//! `or_else`; and the generated `inv`, `curry`, `bcurry`, `collect`, `spread`,
//! `rev_args`, `tap`, `const`, and `compose`.
//!
//! The generated globals are written in Frost (see the runtime's `generated`
//! module), so the function each returns tail-calls the function it wraps,
//! wherever that call is the last thing it does. The tail-call cases check that
//! under a small call-depth limit, with recursion far deeper than the limit.
//!
//! The harness runs every case under every optimization permutation, so a call
//! over literals is checked both folded and at run time.

mod source;

use frostlang_runtime::{Arity, Value};
use source::assertions::{Library, library_assertions};
use source::{Script, raises, run};

library_assertions!(Library::GLOBALS);

/// Run `source` with the call depth limited far below the recursion it does, so
/// it completes only if its calls through the global are tail calls.
fn run_deep(source: &str) -> Value {
    Script::new(source).max_call_depth(32).run()
}

// --- call ---

#[test]
fn call_calls_its_function_with_an_arrays_elements() {
    assert_values(&[
        ("call(fn -> 7)", "7"),
        ("call(collect)", "[]"),
        ("call(collect, [])", "[]"),
        ("call(minus, [10, 3])", "7"),
        // Each element is one argument; nested Arrays are not spread.
        ("call(collect, [1, [2, 3], null])", "[1, [2, 3], null]"),
        ("call(fn a, ...rest -> [a, rest], [1, 2, 3])", "[1, [2, 3]]"),
    ]);
}

#[test]
fn call_raises_as_its_function_does() {
    assert_raises_as(&[
        ("call(minus, [1])", "minus(1)"),
        ("call(minus)", "minus()"),
        ("call(fn x -> x + 'a', [1])", "1 + 'a'"),
    ]);
}

#[test]
fn call_checks_its_argument_types_in_order() {
    assert_raises(&[
        (
            "call(1)",
            "Function call requires Function as argument 1, got Int",
        ),
        (
            "call(1, [])",
            "Function call requires Function as argument 1, got Int",
        ),
        (
            "call(plus, 5)",
            "Function call requires Array as argument 2, got Int",
        ),
        (
            "call(plus, null)",
            "Function call requires Array as argument 2, got Null",
        ),
        // The function is checked first, as `try_call` checks it.
        (
            "call(1, 5)",
            "Function call requires Function as argument 1, got Int",
        ),
    ]);
}

#[test]
fn call_calls_a_runtime_function() {
    let wrap = Value::native("wrap", Arity::Exact(1), |_, args| {
        Ok(Value::array([args[0].take()]))
    });
    let called = Script::new("call(f, [2])").capture("f", wrap).run();
    assert_eq!(called, run("[2]"));
}

#[test]
fn calls_through_call_are_tail_calls() {
    let source = r"
        defn countdown(n) -> if n == 0: 'done' else: call(countdown, [n - 1])
        countdown(1000)
    ";
    assert_eq!(run_deep(source), Value::from("done"));
}

#[test]
fn call_takes_one_or_two_arguments() {
    assert_arity("call", "between 1 and 2", &[0, 3]);
}

// --- try_call ---

#[test]
fn try_call_reports_a_returned_value() {
    assert_values(&[
        ("try_call(fn -> 1)", "{ok: true, value: 1}"),
        ("try_call(fn -> null)", "{ok: true, value: null}"),
        ("try_call(plus, [1, 2])", "{ok: true, value: 3}"),
        ("try_call(collect, [])", "{ok: true, value: []}"),
    ]);
}

#[test]
fn try_call_reports_a_raised_error() {
    assert_values(&[
        ("try_call(fn -> error('boom')).ok", "false"),
        ("try_call(fn -> error('boom')).error", "'boom'"),
        (
            "sorted(keys(try_call(fn -> error('boom'))))",
            "['error', 'ok', 'trace']",
        ),
        // A trace is the names of the frames the error passed through.
        ("is_array(try_call(fn -> error('boom')).trace)", "true"),
        (
            "try_call(fn -> error('boom')).trace @ all(is_string)",
            "true",
        ),
    ]);
}

#[test]
fn a_caught_error_is_the_message_it_would_raise() {
    for (call, uncaught) in [
        ("try_call(fn x -> x + 'a', [1])", "1 + 'a'"),
        ("try_call(minus, [1])", "minus(1)"),
        ("try_call(fn -> assert(false, 'no'))", "assert(false, 'no')"),
        ("try_call(fn -> [1].k)", "[1].k"),
    ] {
        assert_eq!(
            run(&format!("{call}.error")),
            Value::from(raises(uncaught)),
            "{call:?} catches what {uncaught:?} raises"
        );
    }
}

#[test]
fn try_call_reports_an_error_value_intact() {
    for payload in [
        "''",
        "42",
        "1.5",
        "null",
        "false",
        "x'ff'",
        "[1, 'a']",
        "{code: 3}",
    ] {
        assert_values(&[(&format!("try_call(fn -> error({payload})).error"), payload)]);
    }
}

#[test]
fn try_call_may_nest() {
    let source = r"
        def outer = try_call(fn -> try_call(fn -> error('inner')))
        [outer.ok, outer.value.ok, outer.value.error]
    ";
    assert_values(&[(source, "[true, false, 'inner']")]);
}

#[test]
fn try_call_raises_its_own_argument_errors() {
    // Its own arguments are checked before anything is caught.
    assert_raises(&[
        (
            "try_call(1)",
            "Function try_call requires Function as argument 1, got Int",
        ),
        (
            "try_call(null, [])",
            "Function try_call requires Function as argument 1, got Null",
        ),
        (
            "try_call(plus, 1)",
            "Function try_call requires Array as argument 2, got Int",
        ),
        (
            "try_call(plus, null)",
            "Function try_call requires Array as argument 2, got Null",
        ),
        (
            "try_call(plus, {a: 1})",
            "Function try_call requires Array as argument 2, got Map",
        ),
    ]);
}

#[test]
fn try_call_takes_one_or_two_arguments() {
    assert_arity("try_call", "between 1 and 2", &[0, 3]);
}

// --- error ---

#[test]
fn error_raises_a_string_as_its_message() {
    assert_raises(&[
        ("error('boom')", "boom"),
        ("error('')", ""),
        (r"error('two\nlines')", "two\nlines"),
    ]);
}

#[test]
fn error_raises_any_value() {
    // The value is caught intact; see `try_call_reports_an_error_value_intact`.
    // Uncaught, its message is the value as `to_string` renders it.
    for payload in ["42", "null", "[1, 'a']", "{code: 3}", "x'ff'"] {
        assert_eq!(
            Value::from(raises(&format!("error({payload})"))),
            run(&format!("to_string({payload})")),
            "{payload}"
        );
    }
}

#[test]
fn error_raises_a_runtime_value() {
    let message = Script::new("error(e)")
        .capture("e", Value::from("from the host"))
        .raises();
    assert_eq!(message, "from the host");
}

#[test]
fn error_takes_exactly_one_argument() {
    assert_arity("error", 1, &[0, 2]);
}

// --- and_then, or_else ---

#[test]
fn and_then_calls_its_function_on_a_value_that_is_not_null() {
    assert_values(&[
        ("and_then(1, fn x -> x + 1)", "2"),
        // Falsy is not Null.
        ("and_then(false, fn x -> [x])", "[false]"),
        ("and_then(0, fn x -> [x])", "[0]"),
        ("and_then(1, fn x -> null)", "null"),
    ]);
}

#[test]
fn and_then_passes_null_through_without_a_call() {
    assert_values(&[("and_then(null, fn x -> error('called'))", "null")]);
}

#[test]
fn or_else_calls_its_function_only_for_null() {
    assert_values(&[
        ("or_else(null, fn -> 2)", "2"),
        ("or_else(null, fn -> null)", "null"),
        ("or_else(1, fn -> error('called'))", "1"),
        // Falsy is not Null.
        ("or_else(false, fn -> error('called'))", "false"),
        ("or_else(0, fn -> error('called'))", "0"),
    ]);
}

#[test]
fn and_then_and_or_else_chain_in_a_pipeline() {
    assert_values(&[
        ("1 @ and_then(fn x -> x * 10) @ or_else(fn -> 0)", "10"),
        ("null @ and_then(fn x -> x * 10) @ or_else(fn -> 0)", "0"),
        (
            "[1, 2] @ find(fn x -> x > 5) @ and_then(fn x -> x + 1) @ or_else(fn -> 'none')",
            "'none'",
        ),
    ]);
}

#[test]
fn and_then_and_or_else_raise_as_their_functions_do() {
    assert_raises_as(&[
        ("and_then(1, plus)", "plus(1)"),
        ("and_then(1, fn x -> x + 'a')", "1 + 'a'"),
        // The fallback is called with no arguments.
        ("or_else(null, fn x -> x)", "(fn x -> x)()"),
        (
            "or_else(null, fn -> error('fallback'))",
            "error('fallback')",
        ),
    ]);
}

#[test]
fn and_then_and_or_else_require_a_function_even_when_it_is_not_called() {
    for function in ["and_then", "or_else"] {
        for (source, got) in [
            (format!("{function}(1, 2)"), "Int"),
            (format!("{function}(null, 2)"), "Int"),
            (format!("{function}(1, null)"), "Null"),
            (format!("{function}(null, null)"), "Null"),
        ] {
            assert_eq!(
                raises(&source),
                format!("Function {function} requires Function as argument 2, got {got}"),
                "{source:?}"
            );
        }
    }
}

#[test]
fn and_then_and_or_else_take_exactly_two_arguments() {
    assert_arity("and_then", 2, &[0, 1, 3]);
    assert_arity("or_else", 2, &[0, 1, 3]);
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
fn an_inverted_function_raises_as_its_function_does() {
    assert_raises_as(&[("inv(fn x -> x)()", "(fn x -> x)()")]);
}

#[test]
fn inv_takes_exactly_one_argument() {
    assert_arity("inv", 1, &[0, 2]);
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
    ]);
}

#[test]
fn curry_and_bcurry_take_at_least_one_argument() {
    assert_arity("curry", "at least 1", &[0]);
    assert_arity("bcurry", "at least 1", &[0]);
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
    ]);
}

#[test]
fn spread_takes_exactly_one_argument() {
    assert_arity("spread", 1, &[0, 2]);
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
    ]);
}

#[test]
fn rev_args_takes_exactly_one_argument() {
    assert_arity("rev_args", 1, &[0, 2]);
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
    assert_raises_as(&[("tap(1, fn x -> x + 'a')", "1 + 'a'")]);
}

#[test]
fn tap_takes_exactly_two_arguments() {
    assert_arity("tap", 2, &[0, 1, 3]);
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
    assert_arity("const", 1, &[0, 2]);
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
    for (source, position, got) in [
        ("compose(1, id)", 1, "Int"),
        ("compose(id, 'x')", 2, "String"),
        ("compose(id, id, null)", 3, "Null"),
        ("compose(id, id, id, [id])", 4, "Array"),
    ] {
        assert_eq!(
            raises(source),
            format!("Function compose requires Function as argument {position}, got {got}"),
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
    assert_arity("compose", "at least 2", &[0, 1]);
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
fn a_function_argument_is_checked_before_it_is_called() {
    // The check raises when the global is called, not later when the function
    // it returns is, and names the global as a native's type check does.
    for (source, function, position) in [
        ("inv(1)", "inv", 1),
        ("curry(1)", "curry", 1),
        ("curry(1, 2)", "curry", 1),
        ("bcurry(1)", "bcurry", 1),
        ("spread(1)", "spread", 1),
        ("rev_args(1)", "rev_args", 1),
        ("tap(1, 2)", "tap", 2),
    ] {
        assert_eq!(
            raises(source),
            format!("Function {function} requires Function as argument {position}, got Int"),
            "{source:?}"
        );
    }
}
