//! Constant propagation, end to end: a binding whose value is compile-time known
//! is loaded directly at each use, so a lookup needs no local slot read.
//!
//! The harness runs each behavioral case under every optimization permutation,
//! so propagation is checked never to change a result; the code assertions
//! confirm a lookup was actually propagated.

mod common;

use common::{Emitted, Script, run};
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
