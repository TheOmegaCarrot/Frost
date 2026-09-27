//! Constant propagation, end to end: a binding whose value is compile-time known
//! is loaded directly at each use, so a lookup needs no local slot read.
//!
//! The harness runs each behavioral case under every optimization permutation,
//! so propagation is checked never to change a result; the code assertions
//! confirm a lookup was actually propagated.

mod common;

use common::{Emitted, Script, raises, run};
use frost_runtime::{Bytecode, MapKey, Value};

/// The code of `source` under every optimization permutation with constant
/// propagation on (or off, if `propagate` is false).
fn emitted(source: &str, propagate: bool) -> Vec<Emitted> {
    Script::new(source).code_where(|optimization| optimization.constant_propagate == propagate)
}

fn loads_a_local(emitted: &Emitted) -> bool {
    emitted
        .code
        .iter()
        .any(|op| matches!(op, Bytecode::LoadLocal(_)))
}

#[test]
fn propagation_preserves_a_scalar_binding_value() {
    assert_eq!(run("def x = 5; x"), Value::Int(5));
}

#[test]
fn propagation_preserves_a_pooled_binding_value() {
    assert_eq!(run(r#"def s = "hello"; s"#), Value::from("hello"));
}

#[test]
fn a_propagated_lookup_loads_the_constant_not_the_local() {
    for emitted in emitted("def x = 5; x", false) {
        assert!(
            loads_a_local(&emitted),
            "without propagation the lookup is a local load: {emitted:?}"
        );
    }
    for emitted in emitted("def x = 5; x", true) {
        assert!(
            !loads_a_local(&emitted),
            "a propagated lookup pushes the constant, with no slot read: {emitted:?}"
        );
    }
}

#[test]
fn a_propagated_pooled_lookup_loads_a_constant() {
    for emitted in emitted(r#"def s = "hello"; s"#, true) {
        assert!(
            !loads_a_local(&emitted),
            "a propagated String lookup emits no LoadLocal: {emitted:?}"
        );
        assert!(
            emitted
                .code
                .iter()
                .any(|op| matches!(op, Bytecode::LoadConst(_))),
            "the propagated String is loaded from the constant pool: {emitted:?}"
        );
    }
}

#[test]
fn propagation_preserves_an_array_binding_value() {
    assert_eq!(
        run("def a = [1, 2, 3]; a"),
        Value::from(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn propagation_preserves_a_map_binding_value() {
    let expected: Value = [(MapKey::from("a"), Value::Int(1))].into_iter().collect();
    assert_eq!(run("def m = {a: 1}; m"), expected);
}

#[test]
fn a_propagated_array_lookup_loads_a_constant_not_the_local() {
    // Unlike a scalar or String literal, an Array literal's code is several
    // ops (each element pushed, then `MakeArray`), not one; propagation alone
    // does not recognize that as a constant. Only once folding has first
    // collapsed it to a single value does propagation have a constant to
    // record and inline.
    let source = "def a = [1, 2, 3]; a";
    for emitted in Script::new(source).code_where(|o| o.constant_propagate && !o.constant_fold) {
        assert!(
            loads_a_local(&emitted),
            "propagated but not folded, the Array is still built and read as a local: {emitted:?}"
        );
    }
    for emitted in Script::new(source).code_where(|o| o.constant_propagate && o.constant_fold) {
        assert!(
            !loads_a_local(&emitted),
            "folded then propagated, the lookup pushes the constant, with no slot read: {emitted:?}"
        );
        assert_eq!(
            emitted.count(&Bytecode::MakeArray(3)),
            0,
            "the Array is not rebuilt at the use site: {emitted:?}"
        );
        assert!(
            emitted
                .code
                .iter()
                .any(|op| matches!(op, Bytecode::LoadConst(_))),
            "the propagated Array is loaded from the constant pool: {emitted:?}"
        );
    }
}

#[test]
fn a_propagated_map_lookup_loads_a_constant_not_the_local() {
    // Same reasoning as the Array case above: a Map literal needs folding
    // before propagation has a single constant value to work with.
    let source = "def m = {a: 1, b: 2}; m";
    for emitted in Script::new(source).code_where(|o| o.constant_propagate && o.constant_fold) {
        assert!(
            !loads_a_local(&emitted),
            "folded then propagated, the lookup pushes the constant, with no slot read: {emitted:?}"
        );
        assert_eq!(
            emitted.count(&Bytecode::MakeMap(2)),
            0,
            "the Map is not rebuilt at the use site: {emitted:?}"
        );
    }
}

#[test]
fn an_empty_structure_propagates_without_folding() {
    // An empty literal is a single op, so it is a known constant as written.
    for source in ["def a = []; a", "def m = {}; m"] {
        for emitted in emitted(source, true) {
            assert!(
                !loads_a_local(&emitted),
                "{source:?}: the lookup pushes the constant, with no slot read: {emitted:?}"
            );
        }
        for emitted in emitted(source, false) {
            assert!(loads_a_local(&emitted), "{source:?}: {emitted:?}");
        }
    }
    assert_eq!(run("def a = []; [a, a + [1]]"), run("[[], [1]]"));
    assert_eq!(run("def m = {}; [m, m + {k: 1}]"), run("[{}, {k: 1}]"));
}

// --- Destructured bindings ---

/// Destructures of a known value, reading back every part they bind, and the
/// value they read.
const KNOWN_DESTRUCTURES: [(&str, &str); 6] = [
    ("def [a, b] = [1, 2]; [b, a]", "[2, 1]"),
    ("def [a, ...r] = [1, 2, 3]; [a, r]", "[1, [2, 3]]"),
    ("def [_, ...r] = [1]; r", "[]"),
    (
        "def {x, y: [z]} as m = {x: 1, y: [2]}; [x, z, m]",
        "[1, 2, {x: 1, y: [2]}]",
    ),
    ("def [[a], {b}] = [[1], {b: 2}]; [a, b]", "[1, 2]"),
    // A computed key reading an earlier part is itself known.
    (r#"def {a, [a]: b} = {a: "c", c: 3}; [a, b]"#, r#"["c", 3]"#),
];

#[test]
fn propagation_preserves_destructured_values() {
    for (source, expected) in KNOWN_DESTRUCTURES {
        assert_eq!(run(source), run(expected), "{source:?}");
    }
}

#[test]
fn each_part_of_a_known_value_propagates() {
    // A structure literal is a known value once folded.
    for (source, _) in KNOWN_DESTRUCTURES {
        for emitted in Script::new(source).code_where(|o| o.constant_propagate && o.constant_fold) {
            assert!(
                !loads_a_local(&emitted),
                "{source:?}: every lookup pushes its part's constant: {emitted:?}"
            );
        }
    }
}

#[test]
fn a_propagated_part_folds_where_it_is_used() {
    let source = "def [a, {b}] = [6, {b: 7}]; a * b";
    for emitted in Script::new(source).code_where(|o| o.constant_propagate && o.constant_fold) {
        assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(42)), 1, "{emitted:?}");
    }
    assert_eq!(run(source), Value::Int(42));
}

#[test]
fn a_part_of_a_runtime_value_is_not_propagated() {
    for source in ["def [a] = x; a", "def {k} = y; k"] {
        let script = Script::new(source).captures(&[("x", run("[1]")), ("y", run("{k: 1}"))]);
        assert_eq!(script.run(), Value::Int(1), "{source:?}");
        for emitted in script.code_where(|o| o.constant_propagate) {
            assert!(loads_a_local(&emitted), "{source:?}: {emitted:?}");
        }
    }
}

#[test]
fn a_known_value_of_the_wrong_shape_still_raises() {
    // No part is bound, so none propagates; the destructure raises as ever.
    for (source, message) in [
        ("def [a, b] = [1]; a", "exactly 2 elements"),
        ("def [a, b, ...r] = [1]; a", "at least 2 elements"),
        ("def {a} = [1]; a", "expected a Map"),
        ("def [a] = {a: 1}; a", "expected an Array"),
        ("def {a, b} = {a: 1}; a", "no value at key 'b'"),
    ] {
        let raised = raises(source);
        assert!(raised.contains(message), "{source:?}: {raised}");
    }
}
