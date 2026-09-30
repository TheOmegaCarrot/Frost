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
use frost_runtime::{Arity, Bytecode, Value};

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
        (
            r"
            def x = [1]
            x
            ",
            vec![true],
        ),
        (
            r"
            def x = [1]
            [x, x]
            ",
            vec![false, true],
        ),
        (
            r"
            def x = [1]
            [x, x, x]
            ",
            vec![false, false, true],
        ),
        // Each local has its own last read.
        (
            r"
            def a = 1
            def b = 2
            [a, b, a, b]
            ",
            vec![false, false, true, true],
        ),
        (
            r"
            def a = 1
            def b = 2
            [a, b]
            ",
            vec![true, true],
        ),
    ] {
        let emitted = Script::new(source).code(CONSUME);
        assert_eq!(reads(&emitted), expected, "{source:?}: {emitted:?}");
    }
}

#[test]
fn without_the_option_nothing_is_consumed() {
    let source = r"
        def x = [1]
        [x, x]
    ";
    let emitted = Script::new(source).code(UNOPTIMIZED);
    assert_eq!(reads(&emitted), [false, false], "{emitted:?}");
}

#[test]
fn an_exported_local_is_never_consumed() {
    let explicit = r"
        export def x = [1]
        [x, x]
    ";
    let implicit = r"
        def x = [1]
        [x, x]
    ";
    for script in [
        Script::new(explicit),
        Script::new(implicit).implicit_export(),
    ] {
        let emitted = script.code(CONSUME);
        assert_eq!(reads(&emitted), [false, false], "{emitted:?}");
    }
    // A local bound in a nested scope is not exported, so it is consumed.
    let nested = r"
        def r = do {
            def x = [1]
            [x, x]
        }
        r
    ";
    let emitted = Script::new(nested).implicit_export().code(CONSUME);
    assert_eq!(reads(&emitted), [false, true, false], "{emitted:?}");
}

/// The reads of the local `name` in `emitted`, in order: `true` for a consuming
/// read.
fn reads_of(emitted: &Emitted, name: &str) -> Vec<bool> {
    let slot = emitted.slot_named(name);
    emitted
        .code
        .iter()
        .filter_map(|op| match *op {
            Bytecode::LoadLocal(at) if at == slot => Some(false),
            Bytecode::ConsumeLocal(at) if at == slot => Some(true),
            _ => None,
        })
        .collect()
}

#[test]
fn a_last_read_in_each_branch_is_consumed() {
    // Reads of `c`, then of `x` in each branch.
    for source in [
        "fn c, x -> if c: x else: x",
        "fn c, x -> if c: [x] else: [x, 1]",
    ] {
        let emitted = Script::new(source).code(CONSUME).nested(0);
        assert_eq!(
            reads(&emitted),
            [true, true, true],
            "{source:?}: {emitted:?}"
        );
    }
}

#[test]
fn a_read_before_a_branch_that_reads_again_is_a_copy() {
    for (source, expected) in [
        // Reads of `x`, then `c`, then `x` in the branch that reads it again.
        ("fn c, x -> [x, if c: x else: 0]", vec![false, true, true]),
        ("fn c, x -> [x, if c: 0 else: x]", vec![false, true, true]),
        // No branch reads `x` again.
        ("fn c, x -> [x, if c: 0 else: 1]", vec![true, true]),
    ] {
        let emitted = Script::new(source).code(CONSUME).nested(0);
        assert_eq!(reads(&emitted), expected, "{source:?}: {emitted:?}");
    }
}

#[test]
fn a_read_in_a_branch_before_a_later_read_is_a_copy() {
    // Reads of `c`, then `x` in the consequent, then `x` after the `if`.
    let source = "fn c, x -> [if c: x else: 0, x]";
    let emitted = Script::new(source).code(CONSUME).nested(0);
    assert_eq!(
        reads(&emitted),
        [true, false, true],
        "{source:?}: {emitted:?}"
    );
}

#[test]
fn a_short_circuit_that_may_read_again_copies_its_first_read() {
    // `or` keeps a truthy `x` and otherwise reads `x` again; `and` the reverse.
    for source in ["fn x -> x or x", "fn x -> x and x"] {
        let emitted = Script::new(source).code(CONSUME).nested(0);
        assert_eq!(reads(&emitted), [false, true], "{source:?}: {emitted:?}");
    }
}

#[test]
fn a_last_read_in_each_match_arm_is_consumed() {
    let source = r"
        fn v, x -> match v {
            1 => x,
            2 => [x],
            _ => [x, x]
        }
    ";
    let emitted = Script::new(source).code(CONSUME).nested(0);
    assert_eq!(
        reads_of(&emitted, "x"),
        [true, true, false, true],
        "{emitted:?}"
    );
}

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

#[test]
fn a_last_read_in_either_branch_hands_over_the_only_reference() {
    // The Array is built at runtime, so only `xs` holds it.
    let source = r"
        defn probe(c, xs) -> if c: unshared(xs) else: unshared(xs)
        def make = fn n -> [n]
        [probe(true, make(1)), probe(false, make(2))]
    ";
    let tail = Script::new(source)
        .capture("unshared", unshared())
        .run_under(CONSUME);
    assert_eq!(tail, Value::array([true, true]));
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
            r"
            def a = [1]
            def b = a
            def c = b + [2]
            [a, b, c]
            ",
            "[[1], [1], [1, 2]]",
        ),
        (
            r"
            def a = [1]
            def c = a + [2]
            def d = a + [3]
            [c, d]
            ",
            "[[1, 2], [1, 3]]",
        ),
        (
            r"
            def m = {k: 1}
            def n = m + {j: 2}
            [m, n]
            ",
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
            r"
            def xs = [1]
            def f = fn -> xs
            def ys = xs + [2]
            [f(), ys]
            ",
            "[[1], [1, 2]]",
        ),
        (
            r"
            def f = fn xs -> [fn -> xs, xs + [2]]
            def r = f([1])
            [r[0](), r[1]]
            ",
            "[[1], [1, 2]]",
        ),
    ] {
        assert_eq!(run(source), run(expected), "{source:?}");
    }
}

#[test]
fn an_exported_value_survives_its_last_read() {
    let source = r"
        export def xs = [1]
        def ys = xs + [2]
        ys
    ";
    let finished = Script::new(source).finish();
    assert_eq!(finished.tail, run("[1, 2]"));
    assert_eq!(finished.exports["xs"], run("[1]"));
}

#[test]
fn a_local_read_in_branches_is_read_correctly_on_either_path() {
    let source = r"
        def f = fn c, xs -> [if c: xs + [0] else: xs, xs]
        [f(true, [1]), f(false, [1])]
    ";
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
    let source = r"
        defn build(n, acc) -> if n == 0: acc else: build(n - 1, acc + [n])
        def xs = build(200, [])
        def m = fold(xs, fn m, x -> m + {[x % 7]: x}, {})
        [xs[0], xs[199], m[0], m[6]]
    ";
    assert_eq!(
        run(source),
        Value::from_iter([200, 1, 7, 6].map(Value::Int))
    );
}
