//! Shared helpers for the VM integration tests.
//!
//! Lives in `common/mod.rs` (not `common.rs`) so Cargo does not compile it as
//! its own test binary. `allow(dead_code)` because each test binary pulls in
//! the whole module but uses only the helpers it needs.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::Arc;

use frost_runtime::{Bytecode, CompiledFunction, NameTableEntry, ProgramResult, Vm};

/// A nameless compiled function with no locals, constants, or child functions.
pub fn empty_fn(code: Vec<Bytecode>) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: None,
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: BTreeMap::new(),
    })
}

/// A nameless compiled function with an explicit local name table.
pub fn fn_with_locals(
    code: Vec<Bytecode>,
    locals: BTreeMap<String, NameTableEntry>,
) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: None,
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: locals,
    })
}

/// Build a single name-table entry, for use with `BTreeMap::from([...])`.
pub fn slot(name: &str, slot: usize, exported: bool) -> (String, NameTableEntry) {
    (name.to_string(), NameTableEntry { slot, exported })
}

/// Run a nameless, local-less program to completion.
pub fn run(code: Vec<Bytecode>) -> ProgramResult {
    Vm::new(empty_fn(code)).unwrap().run().unwrap()
}
