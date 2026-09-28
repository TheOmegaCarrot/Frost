//! The consume-locals optimization: each local's last read moves the value out
//! of its slot instead of copying it, so a structure held only by that local can
//! be updated in place.
//!
//! Behavioral cases run under every optimization permutation, so consuming is
//! checked never to change a result, above all where a value is shared: with
//! another local, a closure, or an export. Code-shape cases pin exactly the
//! options they are about.

mod common;

use common::{Emitted, Script, UNOPTIMIZED, run};
use frost_compile::OptimizationOptions;
use frost_runtime::{Bytecode, Value};

const CONSUME: OptimizationOptions = OptimizationOptions {
    consume_locals: true,
    ..UNOPTIMIZED
};

/// The local reads in `emitted`, in order: `true` for a consuming read.
fn reads(emitted: &Emitted) -> Vec<bool> {
    emitted
        .code
        .iter()
        .filter_map(|op| match op {
            Bytecode::LoadLocal(_) => Some(false),
            Bytecode::ConsumeLocal(_) => Some(true),
            _ => None,
        })
        .collect()
}

// --- Which reads are consumed ---

#[test]
fn only_a_locals_last_read_is_consumed() {
    for (source, expected) in [
        ("def x = [1]; x", vec![true]),
        ("def x = [1]; [x, x]", vec![false, true]),
        ("def x = [1]; [x, x, x]", vec![false, false, true]),
        // Each local has its own last read.
        (
            "def a = 1; def b = 2; [a, b, a, b]",
            vec![false, false, true, true],
        ),
        ("def a = 1; def b = 2; [a, b]", vec![true, true]),
    ] {
        let emitted = Script::new(source).code(CONSUME);
        assert_eq!(reads(&emitted), expected, "{source:?}: {emitted:?}");
    }
}

#[test]
fn without_the_option_nothing_is_consumed() {
    let emitted = Script::new("def x = [1]; [x, x]").code(UNOPTIMIZED);
    assert_eq!(reads(&emitted), [false, false], "{emitted:?}");
}

#[test]
fn an_exported_local_is_never_consumed() {
    for script in [
        Script::new("export def x = [1]; [x, x]"),
        Script::new("def x = [1]; [x, x]").implicit_export(),
    ] {
        let emitted = script.code(CONSUME);
        assert_eq!(reads(&emitted), [false, false], "{emitted:?}");
    }
    // A local bound in a nested scope is not exported, so it is consumed.
    let emitted = Script::new("def r = do { def x = [1]; [x, x] }; r")
        .implicit_export()
        .code(CONSUME);
    assert_eq!(reads(&emitted), [false, true, false], "{emitted:?}");
}

#[test]
fn of_reads_in_separate_branches_only_the_latest_is_consumed() {
    // Either branch's read is the last to run, but only the later one in code
    // order is known to be last whichever way the branch goes.
    let emitted = Script::new("fn c, x -> if c: x else: x")
        .code(CONSUME)
        .nested(0);
    assert_eq!(reads(&emitted), [true, false, true], "{emitted:?}");
}

#[test]
fn a_parameter_and_a_capture_are_consumed_on_their_last_read() {
    let emitted = Script::new("fn xs -> xs + [1]").code(CONSUME).nested(0);
    assert_eq!(reads(&emitted), [true], "a parameter: {emitted:?}");

    // The outer function's last read of `x` is the push that captures it.
    let outer = Script::new("fn x -> fn -> x").code(CONSUME).nested(0);
    assert_eq!(reads(&outer), [true], "captured: {outer:?}");
    let inner = outer.nested(0);
    assert_eq!(reads(&inner), [true], "a capture: {inner:?}");
}

// --- Consuming never changes a result ---

#[test]
fn a_value_shared_with_another_local_is_unaffected() {
    for (source, expected) in [
        (
            "def a = [1]; def b = a; def c = b + [2]; [a, b, c]",
            "[[1], [1], [1, 2]]",
        ),
        (
            "def a = [1]; def c = a + [2]; def d = a + [3]; [c, d]",
            "[[1, 2], [1, 3]]",
        ),
        (
            "def m = {k: 1}; def n = m + {j: 2}; [m, n]",
            "[{k: 1}, {k: 1, j: 2}]",
        ),
    ] {
        assert_eq!(run(source), run(expected), "{source:?}");
    }
}

#[test]
fn a_value_shared_with_a_closure_is_unaffected() {
    for (source, expected) in [
        (
            "def xs = [1]; def f = fn -> xs; def ys = xs + [2]; [f(), ys]",
            "[[1], [1, 2]]",
        ),
        (
            "def f = fn xs -> [fn -> xs, xs + [2]]; def r = f([1]); [r[0](), r[1]]",
            "[[1], [1, 2]]",
        ),
    ] {
        assert_eq!(run(source), run(expected), "{source:?}");
    }
}

#[test]
fn an_exported_value_survives_its_last_read() {
    let finished = Script::new("export def xs = [1]; def ys = xs + [2]; ys").finish();
    assert_eq!(finished.tail, run("[1, 2]"));
    assert_eq!(finished.exports["xs"], run("[1]"));
}

#[test]
fn a_local_read_in_branches_is_read_correctly_on_either_path() {
    let source = "def f = fn c, xs -> [if c: xs + [0] else: xs, xs]; [f(true, [1]), f(false, [1])]";
    assert_eq!(run(source), run("[[[1, 0], [1]], [[1], [1]]]"));
}

#[test]
fn a_slot_rebound_by_a_match_alternative_is_read_correctly() {
    // Both branches store to the same local; a failed branch may have consumed it.
    for (source, expected) in [
        ("match [1, 2] { [x, 3] | [1, x] => [x, x] }", "[2, 2]"),
        (
            "match [[1], [1], 5] { [x, (x), 2] | [x, _, _] => x + [0] }",
            "[1, 0]",
        ),
    ] {
        assert_eq!(run(source), run(expected), "{source:?}");
    }
}

#[test]
fn an_accumulator_built_in_place_holds_every_entry() {
    let source = r"defn build(n, acc) -> if n == 0: acc else: build(n - 1, acc + [n])
        def xs = build(200, [])
        def m = fold(xs, fn m, x -> m + {[x % 7]: x}, {})
        [xs[0], xs[199], m[0], m[6]]";
    assert_eq!(
        run(source),
        Value::from_iter([200, 1, 7, 6].map(Value::Int))
    );
}
