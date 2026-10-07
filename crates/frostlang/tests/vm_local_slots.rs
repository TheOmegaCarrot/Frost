//! A call's local slots are released the moment the call ends, however it ends:
//! a return, a tail call, a return through a native, or an error.
//!
//! A slot left filled after its call does not change any result, but it keeps a
//! reference to its value, so a value it shares can no longer be updated in
//! place. These tests watch for exactly that. Each script hands a value to a call
//! that keeps it in a local, then, once the call is over, asks `unshared` whether
//! anything else still holds the value.
//!
//! Scripts are compiled with consume-locals on, so a local's last read gives up
//! its reference, and every other optimization off, so their values are built at
//! runtime rather than shared with the constant pool.

use std::collections::BTreeMap;

use frostlang::compile::{CompilerOptions, Optimization, OptimizationOptions, compile_in_scope};
use frostlang::{Arity, RunError, Value, Vm};

/// `unshared(structure)`: whether the Array or Map passed in is referenced by
/// nothing but the argument itself.
fn unshared() -> Value {
    Value::native("unshared", Arity::Exact(1), |_, args| {
        let unshared = match args[0].take() {
            Value::Array(array) => array.try_into_vec().is_ok(),
            Value::Map(map) => map.try_into_map().is_ok(),
            other => panic!("`unshared` takes an Array or Map, not {other:?}"),
        };
        Ok(Value::Bool(unshared))
    })
}

/// Run `source`, with `unshared` in scope, to its tail value.
fn run(source: &str) -> Value {
    let options = CompilerOptions::new()
        .with_optimization(OptimizationOptions::NONE.with(Optimization::ConsumeLocals, true));
    let program = compile_in_scope("test.frst", source, options, &["unshared"])
        .unwrap_or_else(|errors| panic!("{source}\n{}", errors.render_plain()))
        .code;
    let closure = program
        .close(BTreeMap::from([("unshared".to_string(), unshared())]))
        .expect("`unshared` is supplied");
    let result = Vm::factory()
        .build(closure)
        .unwrap()
        .run()
        .map_err(RunError::into_error)
        .unwrap_or_else(|error| panic!("{source}\nraised: {}", error.message()));
    result.tail().clone()
}

/// Assert that `source` finishes with `xs` held by nothing else.
fn assert_released(source: &str) {
    assert_eq!(run(source), Value::Bool(true), "{source}");
}

#[test]
fn the_probe_sees_a_value_another_local_holds() {
    let alone = r"
        def xs = [1, 2]
        unshared(xs)
    ";
    assert_eq!(run(alone), Value::Bool(true), "{alone}");

    let held = r"
        def xs = [1, 2]
        def ys = xs
        unshared(xs)
    ";
    assert_eq!(run(held), Value::Bool(false), "{held}");
}

#[test]
fn a_returning_call_releases_its_locals() {
    assert_released(
        r"
        defn hold(v) -> {
            def held = [v]
            null
        }

        def xs = [1, 2]
        hold(xs)
        unshared(xs)
        ",
    );
}

#[test]
fn nested_calls_each_release_their_locals() {
    assert_released(
        r"
        defn nest(v, n) -> {
            def held = [v]
            if n == 0: null
            else: [nest(v, n - 1)]
        }

        def xs = [1, 2]
        nest(xs, 50)
        unshared(xs)
        ",
    );
}

#[test]
fn a_tail_call_releases_the_locals_of_the_frame_it_replaces() {
    assert_released(
        r"
        defn finish() -> null

        defn relay(v) -> {
            def held = [v]
            finish()
        }

        def xs = [1, 2]
        relay(xs)
        unshared(xs)
        ",
    );
}

#[test]
fn a_chain_of_tail_calls_releases_every_replaced_frames_locals() {
    assert_released(
        r"
        defn countdown(v, n) -> {
            def held = [v]
            if n == 0: null
            else: countdown(v, n - 1)
        }

        def xs = [1, 2]
        countdown(xs, 50)
        unshared(xs)
        ",
    );
}

#[test]
fn a_call_returning_through_a_native_releases_its_locals() {
    // `transform` calls back into `hold`; the lambda, and its capture of `xs`,
    // are gone once `transform` returns.
    assert_released(
        r"
        defn hold(v) -> {
            def held = [v]
            null
        }

        def xs = [1, 2]
        transform([1, 2, 3], fn _ -> hold(xs))
        unshared(xs)
        ",
    );
}

// In the error tests, `error` is not called in tail position: a tail call would
// end the calling frame, releasing its locals, before the error is raised.

#[test]
fn a_caught_error_releases_the_locals_of_the_frame_that_raised() {
    assert_released(
        r#"
        defn fail(v) -> {
            def held = [v]
            [error("boom")]
        }

        def xs = [1, 2]
        try_call(fail, [xs])
        unshared(xs)
        "#,
    );
}

#[test]
fn a_caught_error_releases_the_locals_of_every_abandoned_frame() {
    assert_released(
        r#"
        defn fail(v) -> [error("boom")]

        defn outer(v) -> {
            def held = [v]
            [fail(v)]
        }

        def xs = [1, 2]
        try_call(outer, [xs])
        unshared(xs)
        "#,
    );
}

#[test]
fn a_caught_error_through_a_native_releases_the_locals_above_it() {
    assert_released(
        r#"
        defn fail(v) -> {
            def held = [v]
            [error("boom")]
        }

        def xs = [1, 2]
        try_call(fn -> transform([1], fn _ -> fail(xs)))
        unshared(xs)
        "#,
    );
}

#[test]
fn locals_released_by_one_call_are_reused_by_the_next() {
    // The second call's locals start where the first's were; neither sees the
    // other's values.
    assert_eq!(
        run(r"
        defn first(v) -> {
            def a = v + [1]
            def b = a + [2]
            b
        }

        defn second(v) -> {
            def c = v
            [c]
        }

        [first([0]), second([9]), first([5])]
        "),
        run(r"[[0, 1, 2], [[9]], [5, 1, 2]]"),
    );
}
