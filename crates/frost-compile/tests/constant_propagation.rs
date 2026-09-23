//! Constant propagation, end to end: a binding whose value is compile-time known
//! is loaded directly at each use, so a lookup needs no local slot read.
//!
//! The harness runs each behavioral case under every optimization permutation,
//! so propagation is checked never to change a result; the code assertions
//! confirm a lookup was actually propagated.

mod common;

use common::{Emitted, Script, run};
use frost_runtime::{Bytecode, Value};

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
