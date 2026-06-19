//! Shared helpers for the VM integration tests.
//!
//! Lives in `common/mod.rs` (not `common.rs`) so Cargo does not compile it as
//! its own test binary. `allow(dead_code)` because each test binary pulls in
//! the whole module but uses only the helpers it needs.
#![allow(dead_code)]

use std::sync::Arc;

use frost_runtime::{Bytecode, CompiledFunction, NameEntry, ProgramResult, Vm};

/// A nameless compiled function with no locals, constants, or child functions.
pub fn empty_fn(code: Vec<Bytecode>) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: None,
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: Vec::new(),
    })
}

/// A nameless compiled function with an explicit name table. Each entry's
/// position in `names` is its slot index.
pub fn fn_with_locals(code: Vec<Bytecode>, names: Vec<NameEntry>) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: None,
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: names,
    })
}

/// Build a name-table entry. Its index in the `Vec` passed to `fn_with_locals`
/// is its slot.
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
