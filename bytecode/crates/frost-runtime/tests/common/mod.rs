//! Shared helpers for the VM integration tests.
//!
//! Lives in `common/mod.rs` (not `common.rs`) so Cargo does not compile it as its own test binary.
//! `allow(dead_code)` because each test binary pulls in the whole module but uses only the helpers it needs.
#![allow(dead_code)]

use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, Closure, CompiledFunction, GLOBAL_NAMES, NameEntry, ProgramResult, Vm,
};

/// The `LoadGlobal` slot index of a predefined global, by name. Panics if `name` is not
/// a predefined global. (The runtime exposes only the ordered [`GLOBAL_NAMES`]; a slot is
/// just its position.)
pub fn global_slot(name: &str) -> usize {
    GLOBAL_NAMES
        .iter()
        .position(|&n| n == name)
        .unwrap_or_else(|| panic!("`{name}` is not a predefined global"))
}

/// A nameless compiled function with no locals, constants, or child functions.
/// Arity `Exact(0)` -- suitable for a top-level / thunk.
pub fn empty_fn(code: Vec<Bytecode>) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: "<test>".to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    })
}

/// A nameless compiled function with an explicit name table.
/// Each entry's position in `names` is its slot index.
/// Arity `Exact(0)`, no captures.
pub fn fn_with_locals(code: Vec<Bytecode>, names: Vec<NameEntry>) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: "<test>".to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: names,
        num_captures: 0,
        arity: Arity::Exact(0),
    })
}

/// A fully-specified compiled function, for call tests: explicit `arity`,
/// a `names` table sizing the local slots, and `child_fns` reachable by `CreateClosure`.
/// No captures (capture-bearing closures are produced at runtime by `CreateClosure`).
pub fn func(
    code: Vec<Bytecode>,
    arity: Arity,
    names: Vec<NameEntry>,
    child_fns: Vec<Arc<CompiledFunction>>,
) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: "<test>".to_string(),
        code,
        child_fns,
        constants: Vec::new(),
        name_table: names,
        num_captures: 0,
        arity,
    })
}

/// Build a name-table entry.
/// Its index in the slice passed to `fn_with_locals` / `func` is its slot.
pub fn entry(name: &str, exported: bool) -> NameEntry {
    NameEntry {
        name: name.to_string(),
        exported,
    }
}

/// Run a nameless, local-less program to completion.
pub fn run(code: Vec<Bytecode>) -> ProgramResult {
    run_fn(empty_fn(code))
}

/// Run a pre-built top-level program (e.g. one carrying `child_fns`) to completion.
///
/// The top-level is invoked like any other closure, so the runner pushes its closure
/// value and the body must `Pop` it first.
/// Test programs are written without that leading `Pop`, so it is spliced in here.
pub fn run_fn(program: Arc<CompiledFunction>) -> ProgramResult {
    let top = CompiledFunction {
        code: std::iter::once(Bytecode::Pop)
            .chain(program.code.iter().copied())
            .collect(),
        ..(*program).clone()
    };
    let closure = Arc::new(top)
        .assert_trusted()
        .into_closure()
        .expect("test top-level captures nothing");
    Vm::factory().build(closure).unwrap().run().unwrap()
}

/// Build a runnable top-level [`Closure`] (no captures) from `code` plus a name
/// table, splicing in the leading fn-value `Pop`.
/// For tests that need the closure itself -- e.g. `reset`, or a direct `Vm::new`.
pub fn closure(code: Vec<Bytecode>, names: Vec<NameEntry>) -> Arc<Closure> {
    let mut body = vec![Bytecode::Pop];
    body.extend(code);
    Arc::new(CompiledFunction {
        name: "<test>".to_string(),
        code: body,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: names,
        num_captures: 0,
        arity: Arity::Exact(0),
    })
    .assert_trusted()
    .into_closure()
    .expect("no captures")
}
