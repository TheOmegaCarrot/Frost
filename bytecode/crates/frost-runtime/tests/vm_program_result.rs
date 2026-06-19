mod common;

use common::{empty_fn, entry, fn_with_locals, run};
use frost_runtime::{Bytecode, Value, Vm};

// ============================================================
// get_export / exports
// ============================================================

#[test]
fn get_export_finds_exported_value() {
    let locals = vec![entry("x", true)];
    let program = fn_with_locals(vec![Bytecode::PushInt(42), Bytecode::DefLocal(0)], locals);
    let result = Vm::new(program).unwrap().run().unwrap();
    assert_eq!(result.get_export("x"), Some(&Value::Int(42)));
}

#[test]
fn get_export_returns_none_for_unknown_name() {
    let result = run(vec![]);
    assert_eq!(result.get_export("nothing"), None);
}

#[test]
fn get_export_hides_non_exported_local() {
    // A defined but non-exported top-level local is invisible to get_export:
    // only `export`ed names are part of a program's result surface.
    let locals = vec![entry("secret", false)];
    let program = fn_with_locals(vec![Bytecode::PushInt(7), Bytecode::DefLocal(0)], locals);
    let result = Vm::new(program).unwrap().run().unwrap();
    assert_eq!(result.get_export("secret"), None);
}

#[test]
fn get_export_returns_exported_null_as_some() {
    // An exported binding defined as null reads back as Some(Null), not None --
    // a present null is distinct from an absent name.
    let locals = vec![entry("maybe", true)];
    let program = fn_with_locals(vec![Bytecode::PushNull, Bytecode::DefLocal(0)], locals);
    let result = Vm::new(program).unwrap().run().unwrap();
    assert_eq!(result.get_export("maybe"), Some(&Value::Null));
}

#[test]
fn exports_returns_only_exported() {
    let locals = vec![entry("public_val", true), entry("private_val", false)];
    let program = fn_with_locals(
        vec![
            Bytecode::PushInt(10),
            Bytecode::DefLocal(0),
            Bytecode::PushInt(20),
            Bytecode::DefLocal(1),
        ],
        locals,
    );
    let result = Vm::new(program).unwrap().run().unwrap();
    let exports: Vec<_> = result.exports().collect();
    assert_eq!(exports.len(), 1);
    assert_eq!(exports[0].0, "public_val");
    assert_eq!(exports[0].1, &Value::Int(10));
}

// ============================================================
// reset / warm Vm reuse
// ============================================================

#[test]
fn reset_recycles_into_a_runnable_vm() {
    // A spent ProgramResult re-arms via reset(program) and runs the new program.
    let result_a = Vm::new(empty_fn(vec![Bytecode::PushInt(1)]))
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(result_a.tail(), &Value::Int(1));

    let vm_b = result_a.reset(empty_fn(vec![Bytecode::PushInt(42)]));
    let result_b = vm_b.run().unwrap();
    assert_eq!(result_b.tail(), &Value::Int(42));
}

#[test]
fn reset_does_not_leak_bindings() {
    // Program A loads an exported `x` that the host binds to 100.
    let a_locals = vec![entry("x", true)];
    let mut vm = Vm::new(fn_with_locals(vec![Bytecode::LoadLocal(0)], a_locals)).unwrap();
    assert!(vm.set_binding("x", Value::from(100i64)));
    let result_a = vm.run().unwrap();
    assert_eq!(result_a.get_export("x"), Some(&Value::Int(100)));

    // Recycle into Program B, which has the same slot `x` but is never bound.
    let b_locals = vec![entry("x", true)];
    let result_b = result_a
        .reset(fn_with_locals(vec![], b_locals))
        .run()
        .unwrap();
    // The 100 from A must NOT survive the reset: fresh slots, no carryover.
    assert_eq!(result_b.get_export("x"), None);
}
