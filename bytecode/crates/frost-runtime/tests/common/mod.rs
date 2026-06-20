//! Shared helpers for the VM integration tests.
//!
//! Lives in `common/mod.rs` (not `common.rs`) so Cargo does not compile it as
//! its own test binary. `allow(dead_code)` because each test binary pulls in
//! the whole module but uses only the helpers it needs.
#![allow(dead_code)]

use std::sync::Arc;

use frost_runtime::{Arity, Bytecode, CompiledFunction, NameEntry, ProgramResult, Vm};

/// A nameless compiled function with no locals, constants, or child functions.
/// Arity `Exact(0)` -- suitable for a top-level / thunk.
pub fn empty_fn(code: Vec<Bytecode>) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: None,
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: Vec::new(),
        arity: Arity::Exact(0),
    })
}

/// A nameless compiled function with an explicit name table. Each entry's
/// position in `names` is its slot index. Arity `Exact(0)`.
pub fn fn_with_locals(code: Vec<Bytecode>, names: Vec<NameEntry>) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: None,
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: names,
        arity: Arity::Exact(0),
    })
}

/// A fully-specified compiled function, for call tests: explicit `arity`, a
/// `names` table sizing the local slots, and `child_fns` reachable by
/// `CreateClosure`.
pub fn func(
    code: Vec<Bytecode>,
    arity: Arity,
    names: Vec<NameEntry>,
    child_fns: Vec<Arc<CompiledFunction>>,
) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: None,
        code,
        child_fns,
        constants: Vec::new(),
        name_table: names,
        arity,
    })
}

/// Build a name-table entry. Its index in the `Vec` passed to `fn_with_locals`
/// / `func` is its slot.
pub fn entry(name: &str, exported: bool) -> NameEntry {
    NameEntry {
        name: name.to_string(),
        exported,
    }
}

/// Run a nameless, local-less program to completion.
pub fn run(code: Vec<Bytecode>) -> ProgramResult {
    Vm::new(empty_fn(code)).unwrap().run().unwrap()
}

/// Run a pre-built program (e.g. one carrying `child_fns`) to completion.
pub fn run_fn(program: Arc<CompiledFunction>) -> ProgramResult {
    Vm::new(program).unwrap().run().unwrap()
}
