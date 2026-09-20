//! Constant propagation, end to end: a binding whose value is compile-time known
//! is loaded directly at each use, so a lookup needs no local slot read.
//!
//! Propagation must never change a program's result, only its code, so each case
//! runs both with the option on and off and expects the same value; a companion
//! assertion on the emitted code confirms the lookup was actually propagated.

use frost_compile::{CompilerOptions, OptimizationOptions, compile_program};
use frost_runtime::{Bytecode, Value, Vm};

fn options(constant_propagate: bool) -> CompilerOptions {
    CompilerOptions {
        optimization_options: OptimizationOptions {
            constant_fold: false,
            constant_propagate,
        },
    }
}

/// Compile `source` and run it, returning the tail value.
fn run(source: &str, propagate: bool) -> Value {
    let output =
        compile_program("test.frst", source, options(propagate)).expect("source should compile");
    let closure = output
        .code
        .into_closure()
        .expect("top level needs no captures");
    Vm::factory()
        .build(closure)
        .expect("closure builds")
        .run()
        .map_err(frost_runtime::RunError::into_error)
        .expect("program should run")
        .tail()
        .clone()
}

/// The top-level function's emitted bytecode.
fn code(source: &str, propagate: bool) -> Vec<Bytecode> {
    let output =
        compile_program("test.frst", source, options(propagate)).expect("source should compile");
    output
        .code
        .into_closure()
        .expect("top level needs no captures")
        .inner_fn()
        .code
        .clone()
}

#[test]
fn propagation_preserves_a_scalar_binding_value() {
    for propagate in [false, true] {
        assert_eq!(run("def x = 5\nx", propagate), Value::Int(5));
    }
}

#[test]
fn propagation_preserves_a_pooled_binding_value() {
    for propagate in [false, true] {
        assert_eq!(run("def s = \"hello\"\ns", propagate), Value::from("hello"));
    }
}

#[test]
fn a_propagated_lookup_loads_the_constant_not_the_local() {
    // With propagation off, the lookup reads the binding's slot.
    assert!(
        code("def x = 5\nx", false).contains(&Bytecode::LoadLocal(0)),
        "without propagation the lookup is a local load"
    );
    // With it on, the lookup pushes the constant directly: no slot read.
    assert!(
        !code("def x = 5\nx", true)
            .iter()
            .any(|op| matches!(op, Bytecode::LoadLocal(_))),
        "a propagated lookup emits no LoadLocal"
    );
}

#[test]
fn a_propagated_pooled_lookup_loads_a_constant() {
    let with = code("def s = \"hello\"\ns", true);
    assert!(
        !with.iter().any(|op| matches!(op, Bytecode::LoadLocal(_))),
        "a propagated String lookup emits no LoadLocal"
    );
    assert!(
        with.iter().any(|op| matches!(op, Bytecode::LoadConst(_))),
        "the propagated String is loaded from the constant pool"
    );
}
