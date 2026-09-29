//! White-box tests for invariants not observable through the public API: a
//! failed run leaves no marks behind, and a recycled Vm starts with none.
//!
//! Leftover marks would sit below every mark the next program saves, so they never
//! change its behavior; only a direct check of the private mark stack can see
//! them. (Discarding the marks of frames abandoned by a *caught* error is
//! observable, and is covered black-box in `tests/vm_errors.rs`.)

use super::*;

/// A top-level closure running `code`, after popping its own fn value.
fn program(code: Vec<Bytecode>) -> Arc<Closure> {
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        code: [Bytecode::Pop].into_iter().chain(code).collect(),
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    })
    .assert_trusted()
    .into_closure()
    .expect("captures nothing")
}

/// A run that fails with two marks saved.
fn failed_run() -> RunError {
    let failing = program(vec![
        Bytecode::MarkStack,
        Bytecode::MarkStack,
        Bytecode::PushInt(1),
        Bytecode::ProduceError,
    ]);
    Vm::factory().build(failing).unwrap().run().unwrap_err()
}

#[test]
fn a_failed_run_leaves_no_marks() {
    let failed = failed_run();
    assert!(
        failed.vm.marks.is_empty(),
        "left over: {:?}",
        failed.vm.marks
    );
}

#[test]
fn resetting_a_vm_clears_its_marks() {
    let mut failed = failed_run();
    failed.vm.marks.extend([1, 2]);
    let vm = failed.reset(program(vec![Bytecode::PushNull]));
    assert!(vm.marks.is_empty(), "left over: {:?}", vm.marks);
}

#[test]
fn rebuilding_an_idle_vm_clears_its_marks() {
    let mut failed = failed_run();
    failed.vm.marks.extend([1, 2]);
    let vm = failed
        .into_idle_vm()
        .build(program(vec![Bytecode::PushNull]));
    assert!(vm.marks.is_empty(), "left over: {:?}", vm.marks);
}
