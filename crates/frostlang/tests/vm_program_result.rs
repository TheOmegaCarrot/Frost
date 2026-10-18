use crate::common;

use std::num::NonZeroU64;
use std::sync::Arc;

use common::{closure, entry, fn_with_locals, func, run, run_fn};
use frostlang::bytecode::{Bytecode, NameEntry};
use frostlang::{Arity, Closure, FrostArray, ProgramParts, Value, Vm, VmRuntimeConfiguration};

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
    // An exported binding defined as null reads back as Some(Null), not None:
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
// warm Vm reuse
// ============================================================

#[test]
fn a_result_can_be_recycled_into_a_runnable_vm() {
    // A spent ProgramResult yields its Vm, which builds and runs the new program.
    let result_a = run(vec![Bytecode::PushInt(1)]);
    assert_eq!(result_a.tail(), &Value::Int(1));

    let result_b = result_a
        .into_idle_vm()
        .build(closure(vec![Bytecode::PushInt(42)], vec![]))
        .run()
        .unwrap();
    assert_eq!(result_b.tail(), &Value::Int(42));
}

#[test]
fn recycling_replaces_exports_with_the_new_program() {
    // Program A exports x = 100. After recycling, Program B reuses the Vm and the same
    // slot but exports its own x = 200; the result reflects B's run, not A's.
    let result_a = run_fn(fn_with_locals(
        vec![Bytecode::PushInt(100), Bytecode::DefLocal(0)],
        vec![entry("x", true)],
    ));
    assert_eq!(result_a.get_export("x"), Some(&Value::Int(100)));

    let result_b = result_a
        .into_idle_vm()
        .build(closure(
            vec![Bytecode::PushInt(200), Bytecode::DefLocal(0)],
            vec![entry("x", true)],
        ))
        .run()
        .unwrap();
    assert_eq!(result_b.get_export("x"), Some(&Value::Int(200)));
}

// ============================================================
// into_parts
// ============================================================

/// An Array held only by the caller, to pass into a run.
fn host_array() -> Value {
    Value::Array(FrostArray::from(vec![Value::Int(1), Value::Int(2)]))
}

/// Whether `value`, an Array, is uniquely owned: nothing else holds its elements.
fn is_unique(value: Value) -> bool {
    value
        .try_into_array()
        .expect("the value is an Array")
        .try_into_vec()
        .is_ok()
}

/// A top-level closure taking one argument into its slot 0, named by `slot`,
/// then running `body`.
fn one_param_closure(slot: NameEntry, body: Vec<Bytecode>) -> Arc<Closure> {
    // The prelude seats the argument, then pops the closure's own value.
    let code = [Bytecode::DefLocal(0), Bytecode::Pop]
        .into_iter()
        .chain(body)
        .collect();
    func(code, Arity::Exact(1), vec![slot], vec![])
        .assert_well_formed()
        .into_closure()
        .expect("no captures")
}

#[test]
fn into_parts_yields_the_tail_and_exports() {
    let program = fn_with_locals(
        vec![
            Bytecode::PushInt(10),
            Bytecode::DefLocal(0),
            Bytecode::PushInt(20),
            Bytecode::DefLocal(1),
            Bytecode::PushInt(30),
            Bytecode::DefLocal(2),
            Bytecode::PushInt(7),
        ],
        vec![
            entry("first", true),
            entry("private", false),
            entry("last", true),
        ],
    );
    let ProgramParts { tail, exports, .. } = run_fn(program).into_parts();
    assert_eq!(tail, Value::Int(7), "the tail value");
    assert_eq!(
        exports,
        [
            ("first".to_string(), Value::Int(10)),
            ("last".to_string(), Value::Int(30)),
        ],
        "only the exported bindings, in the order `exports` lists them"
    );
}

#[test]
fn into_parts_yields_null_for_a_program_with_no_tail() {
    // A program of only definitions leaves no tail value, which reads as null.
    let program = fn_with_locals(
        vec![Bytecode::PushInt(1), Bytecode::DefLocal(0)],
        vec![entry("x", true)],
    );
    let ProgramParts { tail, exports, .. } = run_fn(program).into_parts();
    assert_eq!(tail, Value::Null);
    assert_eq!(exports, [("x".to_string(), Value::Int(1))]);
}

#[test]
fn into_parts_keeps_an_exported_null() {
    // A present null export is still listed, as `exports` lists it.
    let program = fn_with_locals(
        vec![Bytecode::PushNull, Bytecode::DefLocal(0)],
        vec![entry("maybe", true)],
    );
    let ProgramParts { exports, .. } = run_fn(program).into_parts();
    assert_eq!(exports, [("maybe".to_string(), Value::Null)]);
}

#[test]
fn a_tail_copied_out_is_shared_while_the_result_lives() {
    // The control for the uniqueness tests below: the tail is a copy of the
    // non-exported local `state`, so a clone through `tail` shares it with the Vm.
    let result = Vm::factory()
        .build(one_param_closure(
            entry("state", false),
            vec![Bytecode::LoadLocal(0)],
        ))
        .run_with_args([host_array()])
        .unwrap();
    let tail = result.tail().clone();
    assert!(
        !is_unique(tail),
        "a tail cloned out of a live result is shared with it"
    );
}

#[test]
fn into_parts_hands_over_a_uniquely_owned_tail() {
    // The tail is a copy of the non-exported local `state`. Taking the result
    // apart must release that local, leaving the tail uniquely owned.
    let result = Vm::factory()
        .build(one_param_closure(
            entry("state", false),
            vec![Bytecode::LoadLocal(0)],
        ))
        .run_with_args([host_array()])
        .unwrap();
    let ProgramParts { tail, idle_vm, .. } = result.into_parts();
    assert!(
        is_unique(tail),
        "the tail is uniquely owned, even while the idle Vm lives"
    );
    drop(idle_vm);
}

#[test]
fn into_parts_hands_over_a_uniquely_owned_export() {
    let result = Vm::factory()
        .build(one_param_closure(
            entry("state", true),
            vec![Bytecode::PushNull],
        ))
        .run_with_args([host_array()])
        .unwrap();
    let ProgramParts {
        mut exports,
        idle_vm,
        ..
    } = result.into_parts();
    let (name, value) = exports.pop().expect("`state` is exported");
    assert_eq!(name, "state");
    assert!(
        is_unique(value),
        "the export is uniquely owned, even while the idle Vm lives"
    );
    drop(idle_vm);
}

#[test]
fn into_parts_shares_a_value_only_between_the_tail_and_an_export() {
    // The tail is the exported local `state` itself, so the two parts share it,
    // and nothing else does: once the export is dropped, the tail is unique.
    let result = Vm::factory()
        .build(one_param_closure(
            entry("state", true),
            vec![Bytecode::LoadLocal(0)],
        ))
        .run_with_args([host_array()])
        .unwrap();
    let ProgramParts {
        tail,
        exports,
        idle_vm,
    } = result.into_parts();
    assert_eq!(exports, [("state".to_string(), tail.clone())]);
    drop(exports);
    assert!(
        is_unique(tail),
        "with the export dropped, only the tail holds the value"
    );
    drop(idle_vm);
}

#[test]
fn the_idle_vm_from_into_parts_runs_the_next_program() {
    // The tail of one run is passed into the next on the recycled Vm: the hand-off
    // a host keeping state between runs makes.
    let step = one_param_closure(entry("state", false), vec![Bytecode::LoadLocal(0)]);
    let first = Vm::factory()
        .build(Arc::clone(&step))
        .run_with_args([host_array()])
        .unwrap()
        .into_parts();
    let second = first
        .idle_vm
        .build(step)
        .run_with_args([first.tail])
        .unwrap()
        .into_parts();
    assert!(
        is_unique(second.tail),
        "the state stays uniquely owned across runs"
    );
}

#[test]
fn recycling_after_into_parts_replaces_exports_with_the_new_program() {
    // Program A exports x = 100. Program B, on the Vm A was taken apart from,
    // exports its own x = 200 from the same slot.
    let ProgramParts { idle_vm, .. } = run_fn(fn_with_locals(
        vec![Bytecode::PushInt(100), Bytecode::DefLocal(0)],
        vec![entry("x", true)],
    ))
    .into_parts();
    let result_b = idle_vm
        .build(closure(
            vec![Bytecode::PushInt(200), Bytecode::DefLocal(0)],
            vec![entry("x", true)],
        ))
        .run()
        .unwrap();
    assert_eq!(result_b.get_export("x"), Some(&Value::Int(200)));
}

// ============================================================
// builder / configuration
// ============================================================

#[test]
fn builder_accepts_configuration_and_builds_a_runnable_vm() {
    // The configured limits are well above what this program needs, so it runs
    // normally; their enforcement is covered in vm_limits.rs.
    let config = VmRuntimeConfiguration::default()
        .with_max_call_depth(NonZeroU64::new(64))
        .with_fuel(NonZeroU64::new(10_000))
        .with_max_import_depth(NonZeroU64::new(8));
    let result = Vm::factory()
        .configuration(config)
        .build(closure(vec![Bytecode::PushInt(7)], vec![]))
        .run()
        .unwrap();
    assert_eq!(result.tail(), &Value::Int(7));
}

// ============================================================
// RunError: a failed run keeps the warm Vm
// ============================================================

#[test]
fn a_failed_run_surfaces_its_error_and_recycles_the_vm() {
    // Program A divides by zero: a failed run. Its `RunError` carries the error and
    // still owns the warm Vm, which `into_idle_vm` recycles to run a fresh program B.
    let failed = Vm::factory()
        .build(closure(
            vec![Bytecode::PushInt(1), Bytecode::PushInt(0), Bytecode::Divide],
            vec![],
        ))
        .run()
        .unwrap_err();

    assert_eq!(failed.error().message(), "Division by zero");
    assert_eq!(failed.fuel_consumed(), 0); // the failing program made no calls

    // The Vm survives the failure: rebuild it onto program B and run to success.
    let recovered = failed
        .into_idle_vm()
        .build(closure(vec![Bytecode::PushInt(42)], vec![]))
        .run()
        .unwrap();
    assert_eq!(recovered.tail(), &Value::Int(42));
}

#[test]
fn a_failed_run_comes_apart_into_its_error_and_the_vm() {
    let failed = Vm::factory()
        .build(closure(
            vec![Bytecode::PushInt(1), Bytecode::PushInt(0), Bytecode::Divide],
            vec![],
        ))
        .run()
        .unwrap_err();

    let (error, idle_vm) = failed.into_parts();
    assert_eq!(error.message(), "Division by zero");
    let recovered = idle_vm
        .build(closure(vec![Bytecode::PushInt(42)], vec![]))
        .run()
        .unwrap();
    assert_eq!(
        recovered.tail(),
        &Value::Int(42),
        "the Vm runs a fresh program"
    );
}
