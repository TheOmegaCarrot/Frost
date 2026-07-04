mod common;

use common::{closure, entry, fn_with_locals, run, run_fn};
use frost_runtime::{Bytecode, Value};

// ============================================================
// get_export / exports
// ============================================================

#[test]
fn get_export_finds_exported_value() {
    let program = fn_with_locals(
        vec![Bytecode::PushInt(42), Bytecode::DefLocal(0)],
        vec![entry("x", true)],
    );
    let result = run_fn(program);
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
    let program = fn_with_locals(
        vec![Bytecode::PushInt(7), Bytecode::DefLocal(0)],
        vec![entry("secret", false)],
    );
    let result = run_fn(program);
    assert_eq!(result.get_export("secret"), None);
}

#[test]
fn get_export_returns_exported_null_as_some() {
    // An exported binding defined as null reads back as Some(Null), not None --
    // a present null is distinct from an absent name.
    let program = fn_with_locals(
        vec![Bytecode::PushNull, Bytecode::DefLocal(0)],
        vec![entry("maybe", true)],
    );
    let result = run_fn(program);
    assert_eq!(result.get_export("maybe"), Some(&Value::Null));
}

#[test]
fn exports_returns_only_exported() {
    let program = fn_with_locals(
        vec![
            Bytecode::PushInt(10),
            Bytecode::DefLocal(0),
            Bytecode::PushInt(20),
            Bytecode::DefLocal(1),
        ],
        vec![entry("public_val", true), entry("private_val", false)],
    );
    let result = run_fn(program);
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
    // A spent ProgramResult re-arms via reset(closure) and runs the new program.
    let result_a = run(vec![Bytecode::PushInt(1)]);
    assert_eq!(result_a.tail(), &Value::Int(1));

    let result_b = result_a
        .reset(closure(vec![Bytecode::PushInt(42)], vec![]))
        .run()
        .unwrap();
    assert_eq!(result_b.tail(), &Value::Int(42));
}

#[test]
fn reset_replaces_exports_with_the_new_program() {
    // Program A exports x = 100. After reset, Program B reuses the Vm and the same
    // slot but exports its own x = 200 -- the result reflects B's run, not A's.
    let result_a = run_fn(fn_with_locals(
        vec![Bytecode::PushInt(100), Bytecode::DefLocal(0)],
        vec![entry("x", true)],
    ));
    assert_eq!(result_a.get_export("x"), Some(&Value::Int(100)));

    let result_b = result_a
        .reset(closure(
            vec![Bytecode::PushInt(200), Bytecode::DefLocal(0)],
            vec![entry("x", true)],
        ))
        .run()
        .unwrap();
    assert_eq!(result_b.get_export("x"), Some(&Value::Int(200)));
}
