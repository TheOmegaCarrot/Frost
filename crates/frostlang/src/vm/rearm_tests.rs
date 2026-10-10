//! White-box tests for invariants not observable through the public API: a
//! failed run leaves no marks or local slots behind, and a recycled Vm starts
//! with none.
//!
//! Leftover marks would sit below every mark the next program saves, and
//! leftover slots below every frame's, so neither changes the next program's
//! behavior; only a direct check of the private state can see them. (Discarding
//! what frames abandoned by a *caught* error held is observable, and is covered
//! black-box in `tests/vm_errors.rs` and `tests/vm_local_slots.rs`.)

use super::*;

/// A top-level closure running `code`, after popping its own fn value.
fn program(code: Vec<Bytecode>) -> Arc<Closure> {
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        origin: None,
        code: [Bytecode::Pop].into_iter().chain(code).collect(),
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: vec![NameEntry {
            name: "x".to_string(),
            exported: false,
        }],
        num_captures: 0,
        arity: Arity::Exact(0),
    })
    .assert_trusted()
    .into_closure()
    .expect("captures nothing")
}

/// A run that fails holding two marks and a defined local.
fn failed_run() -> RunError {
    let failing = program(vec![
        Bytecode::PushInt(1),
        Bytecode::DefLocal(0),
        Bytecode::MarkStack,
        Bytecode::MarkStack,
        Bytecode::PushInt(1),
        Bytecode::ProduceError,
    ]);
    Vm::factory().build(failing).run().unwrap_err()
}

/// A failed run, with marks and slots left over as a faulty unwind would leave them.
fn dirty_failed_run() -> RunError {
    let mut failed = failed_run();
    failed.vm.marks.extend([1, 2]);
    failed.vm.slots.extend([Some(Value::Int(1)), None]);
    failed
}

fn assert_clean(vm: &Vm) {
    assert!(vm.marks.is_empty(), "marks left over: {:?}", vm.marks);
    assert!(vm.slots.is_empty(), "slots left over: {:?}", vm.slots);
}

#[test]
fn a_failed_run_leaves_no_marks_or_slots() {
    assert_clean(&failed_run().vm);
}

#[test]
fn rebuilding_an_idle_vm_clears_its_marks_and_slots() {
    let vm = dirty_failed_run()
        .into_idle_vm()
        .build(program(vec![Bytecode::PushNull]));
    assert_clean(&vm);
}
