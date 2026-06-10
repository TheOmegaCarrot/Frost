#![allow(clippy::approx_constant)]

use std::collections::BTreeMap;
use std::sync::Arc;

use frost_runtime::{CompiledFunction, FrostFloat, NameTableEntry, Value, Vm};
use frost_runtime::Bytecode;

fn empty_fn(code: Vec<Bytecode>) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: None,
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: BTreeMap::new(),
    })
}

fn fn_with_locals(code: Vec<Bytecode>, locals: BTreeMap<String, NameTableEntry>) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        name: None,
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table: locals,
    })
}

fn slot(name: &str, slot: usize, exported: bool) -> (String, NameTableEntry) {
    (name.to_string(), NameTableEntry { slot, exported })
}

fn run(code: Vec<Bytecode>) -> frost_runtime::ProgramResult {
    let program = empty_fn(code);
    Vm::new(program).unwrap().run().unwrap()
}

// ============================================================
// Empty program
// ============================================================

#[test]
fn empty_program_returns_null_tail() {
    let result = run(vec![]);
    assert_eq!(result.tail(), &Value::Null);
}

// ============================================================
// Push constants
// ============================================================

#[test]
fn push_null() {
    let result = run(vec![Bytecode::PushNull]);
    assert_eq!(result.tail(), &Value::Null);
}

#[test]
fn push_true() {
    let result = run(vec![Bytecode::PushTrue]);
    assert_eq!(result.tail(), &Value::Bool(true));
}

#[test]
fn push_false() {
    let result = run(vec![Bytecode::PushFalse]);
    assert_eq!(result.tail(), &Value::Bool(false));
}

#[test]
fn push_int() {
    let result = run(vec![Bytecode::PushInt(42)]);
    assert_eq!(result.tail(), &Value::Int(42));
}

#[test]
fn push_float() {
    let f = FrostFloat::new(3.14).unwrap();
    let result = run(vec![Bytecode::PushFloat(f)]);
    assert_eq!(result.tail(), &Value::Float(f));
}

#[test]
fn tail_is_top_of_stack() {
    let result = run(vec![Bytecode::PushInt(1), Bytecode::PushInt(2), Bytecode::PushInt(3)]);
    assert_eq!(result.tail(), &Value::Int(3));
}

// ============================================================
// Stack manipulation
// ============================================================

#[test]
fn pop_discards_top() {
    let result = run(vec![Bytecode::PushInt(1), Bytecode::PushInt(2), Bytecode::Pop]);
    assert_eq!(result.tail(), &Value::Int(1));
}

#[test]
fn dup_copies_top() {
    let result = run(vec![
        Bytecode::PushInt(42),
        Bytecode::Dup,
        Bytecode::Pop, // pop the dup
    ]);
    assert_eq!(result.tail(), &Value::Int(42));
}

#[test]
fn dup_leaves_two_copies() {
    let result = run(vec![Bytecode::PushInt(7), Bytecode::Dup]);
    // stack: 7, 7 — tail is top = 7
    assert_eq!(result.tail(), &Value::Int(7));
}

#[test]
fn peek_down_1_is_dup() {
    // PeekDown(1) copies the top item (same as Dup)
    let result = run(vec![
        Bytecode::PushInt(10),
        Bytecode::PushInt(20),
        Bytecode::PeekDown(1), // copies 20 (top)
        Bytecode::Pop,         // pop the copy
    ]);
    assert_eq!(result.tail(), &Value::Int(20));
}

#[test]
fn peek_down_copies_deeper() {
    // PeekDown(2) copies the item below the top
    let result = run(vec![
        Bytecode::PushInt(10),
        Bytecode::PushInt(20),
        Bytecode::PeekDown(2), // copies 10 (below top)
    ]);
    assert_eq!(result.tail(), &Value::Int(10));
}

// ============================================================
// Local slots
// ============================================================

#[test]
fn def_local_stores_and_load_local_retrieves() {
    let locals = BTreeMap::from([slot("x", 0, false)]);
    let program = fn_with_locals(
        vec![
            Bytecode::PushInt(99),
            Bytecode::DefLocal(0),
            Bytecode::LoadLocal(0),
        ],
        locals,
    );
    let result = Vm::new(program).unwrap().run().unwrap();
    assert_eq!(result.tail(), &Value::Int(99));
}

#[test]
fn def_local_moves_off_stack() {
    // After DefLocal, the value is no longer on the stack
    let locals = BTreeMap::from([slot("x", 0, false)]);
    let program = fn_with_locals(
        vec![
            Bytecode::PushInt(1),
            Bytecode::PushInt(2),
            Bytecode::DefLocal(0), // moves 2 into slot 0
            // stack now has just 1
        ],
        locals,
    );
    let result = Vm::new(program).unwrap().run().unwrap();
    assert_eq!(result.tail(), &Value::Int(1));
}

#[test]
fn load_local_copies_not_moves() {
    // LoadLocal should copy (clone) — loading twice works
    let locals = BTreeMap::from([slot("x", 0, false)]);
    let program = fn_with_locals(
        vec![
            Bytecode::PushInt(5),
            Bytecode::DefLocal(0),
            Bytecode::LoadLocal(0),
            Bytecode::LoadLocal(0),
        ],
        locals,
    );
    let result = Vm::new(program).unwrap().run().unwrap();
    // stack: 5, 5 — tail = 5
    assert_eq!(result.tail(), &Value::Int(5));
}

// ============================================================
// set_global
// ============================================================

#[test]
fn set_global_fills_slot() {
    let locals = BTreeMap::from([slot("greeting", 0, false)]);
    let program = fn_with_locals(vec![Bytecode::LoadLocal(0)], locals);
    let mut vm = Vm::new(program).unwrap();
    assert!(vm.set_global("greeting", Value::from("hello")));
    let result = vm.run().unwrap();
    assert_eq!(result.tail().as_str(), Some("hello"));
}

#[test]
fn set_global_unknown_name_returns_false() {
    let program = empty_fn(vec![]);
    let mut vm = Vm::new(program).unwrap();
    assert!(!vm.set_global("nonexistent", Value::from(1i64)));
}

#[test]
fn set_global_override() {
    let locals = BTreeMap::from([slot("x", 0, false)]);
    let program = fn_with_locals(vec![Bytecode::LoadLocal(0)], locals);
    let mut vm = Vm::new(program).unwrap();
    vm.set_global("x", Value::from(1i64));
    vm.set_global("x", Value::from(2i64)); // override
    let result = vm.run().unwrap();
    assert_eq!(result.tail(), &Value::Int(2));
}

// ============================================================
// ProgramResult
// ============================================================

#[test]
fn lookup_finds_defined_value() {
    let locals = BTreeMap::from([slot("x", 0, false)]);
    let program = fn_with_locals(vec![Bytecode::PushInt(42), Bytecode::DefLocal(0)], locals);
    let result = Vm::new(program).unwrap().run().unwrap();
    assert_eq!(result.lookup("x"), Some(&Value::Int(42)));
}

#[test]
fn lookup_returns_none_for_unknown_name() {
    let result = run(vec![]);
    assert_eq!(result.lookup("nothing"), None);
}

#[test]
fn lookup_distinguishes_null_from_undefined() {
    let locals = BTreeMap::from([slot("defined_null", 0, false), slot("never_set", 1, false)]);
    let program = fn_with_locals(
        vec![Bytecode::PushNull, Bytecode::DefLocal(0)],
        locals,
    );
    let result = Vm::new(program).unwrap().run().unwrap();
    assert_eq!(result.lookup("defined_null"), Some(&Value::Null));
    assert_eq!(result.lookup("never_set"), None);
}

#[test]
fn exports_returns_only_exported() {
    let locals = BTreeMap::from([
        slot("public_val", 0, true),
        slot("private_val", 1, false),
    ]);
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
