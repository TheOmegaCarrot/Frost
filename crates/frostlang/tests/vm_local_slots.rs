//! A call's local slots are released the moment the call ends, however it ends:
//! a return, a tail call, a return through a native, or an error. A tail call
//! that keeps the calling frame, to a native or from the bottom frame, releases
//! its locals before the callee runs. Within a call, the locals of a `do` block
//! or a `match` arm are released once their scope is done with them.
//!
//! A slot left filled after its call or scope does not change any result, but it
//! keeps a reference to its value, so a value it shares can no longer be updated
//! in place. These tests watch for exactly that. Each script hands a value to a
//! call or scope that keeps it in a local, then asks `unshared`, during or after
//! it, whether anything else still holds the value.
//!
//! Scripts are compiled with consume-locals on, so a local's last read gives up
//! its reference, and every other optimization off, so their values are built at
//! runtime rather than shared with the constant pool.

use crate::unshared::unshared;

use std::collections::BTreeMap;

use frostlang::compile::{CompilerOptions, Optimization, OptimizationOptions, compile_in_scope};
use frostlang::{RunError, Value, Vm};

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

    // Out of tail position, where the tail call would release `ys` itself.
    let held = r"
        def xs = [1, 2]
        def ys = xs
        [unshared(xs)]
    ";
    assert_eq!(run(held), run("[false]"), "{held}");
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
    // `map` calls back into `hold`; the lambda, and its capture of `xs`,
    // are gone once `map` returns.
    assert_released(
        r"
        defn hold(v) -> {
            def held = [v]
            null
        }

        def xs = [1, 2]
        map([1, 2, 3], fn _ -> hold(xs))
        unshared(xs)
        ",
    );
}

// These pin from source the release `vm_tail_call_locals` pins at the opcode.

#[test]
fn a_tail_call_to_a_native_from_a_function_releases_its_locals() {
    assert_released(
        r"
        defn probe(xs) -> {
            def held = xs
            unshared(xs)
        }

        probe([1, 2])
        ",
    );
}

#[test]
fn a_tail_call_to_a_native_from_the_top_level_releases_its_locals() {
    assert_released(
        r"
        def xs = [1, 2]
        def held = xs
        unshared(xs)
        ",
    );
}

#[test]
fn a_tail_call_to_a_closure_from_the_top_level_releases_its_locals() {
    // `check` calls the probe out of tail position, so its own frame stays out
    // of the way: only the top level's tail call can release `held`.
    let source = r"
        defn check(v) -> [unshared(v)]

        def xs = [1, 2]
        def held = xs
        check(xs)
        ";
    assert_eq!(run(source), run("[true]"), "{source}");
}

// In the error tests, `error` is not called in tail position: a tail call would
// release the calling frame's locals before the error is raised.

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
        try_call(fn -> map([1], fn _ -> fail(xs)))
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

// In the scope tests, no read of a local on the path taken is its last, so none
// gives up its reference: only leaving its scope can. Each probe is out of tail
// position, where the tail call would release the calling frame's locals itself.

#[test]
fn a_do_block_releases_its_locals_after_its_value() {
    assert_released(
        r"
        defn probe(c, xs) -> {
            def n = do {
                def held = xs
                if c: len(held) else: 0
            }
            [unshared(xs)]
        }

        probe(false, [1, 2]) == [true]
        ",
    );
}

#[test]
fn a_match_arm_failing_its_guard_releases_its_locals() {
    assert_released(
        r"
        defn probe(xs) -> match xs {
            [held] if: len(held) > 5 => [held],
            [held] => [unshared(held)],
        }

        probe([[1, 2]]) == [true]
        ",
    );
}

#[test]
fn a_match_arm_failing_after_binding_releases_its_locals() {
    assert_released(
        r"
        defn probe(xs) -> match xs {
            [held, 0] => [held],
            [held, _] => [unshared(held)],
        }

        probe([[1, 2], 1]) == [true]
        ",
    );
}

#[test]
fn a_match_arm_failing_into_a_catch_all_releases_its_locals() {
    assert_released(
        r"
        defn probe(c, xs) -> match [xs] {
            [held] if: c => 0,
            _ => [unshared(xs)],
        }

        probe(false, [1, 2]) == [true]
        ",
    );
}

#[test]
fn a_match_arm_failing_into_the_no_match_error_releases_its_locals() {
    assert_released(
        r"
        defn fail(v) -> [match [v] {
            [held] if: false => held,
        }]

        def xs = [1, 2]
        try_call(fail, [xs])
        unshared(xs)
        ",
    );
}

#[test]
fn a_taken_match_arm_releases_its_locals_after_its_result() {
    assert_released(
        r"
        defn probe(c, xs) -> {
            def n = match [xs] {
                [held] => if c: len(held) else: 0,
            }
            [unshared(xs)]
        }

        probe(false, [1, 2]) == [true]
        ",
    );
}

#[test]
fn a_taken_catch_all_arm_releases_its_locals_after_its_result() {
    assert_released(
        r"
        defn probe(c, xs) -> {
            def n = match [xs] {
                [] => 0,
                held => if c: len(held) else: 0,
            }
            [unshared(xs)]
        }

        probe(false, [1, 2]) == [true]
        ",
    );
}
