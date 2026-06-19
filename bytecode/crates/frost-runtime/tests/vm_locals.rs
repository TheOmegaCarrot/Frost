mod common;

use common::{empty_fn, entry, fn_with_locals};
use frost_runtime::{Bytecode, Value, Vm};

// ============================================================
// Local slots
// ============================================================

#[test]
fn def_local_stores_and_load_local_retrieves() {
    let locals = vec![entry("x", false)];
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
    let locals = vec![entry("x", false)];
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
    // LoadLocal should copy (clone) -- loading twice works
    let locals = vec![entry("x", false)];
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
    // stack: 5, 5 -- tail = 5
    assert_eq!(result.tail(), &Value::Int(5));
}

// ============================================================
// set_binding
// ============================================================

#[test]
fn set_binding_fills_slot() {
    let locals = vec![entry("greeting", false)];
    let program = fn_with_locals(vec![Bytecode::LoadLocal(0)], locals);
    let mut vm = Vm::new(program).unwrap();
    assert!(vm.set_binding("greeting", Value::from("hello")));
    let result = vm.run().unwrap();
    assert_eq!(result.tail().as_str(), Some("hello"));
}

#[test]
fn set_binding_unknown_name_returns_false() {
    let program = empty_fn(vec![]);
    let mut vm = Vm::new(program).unwrap();
    assert!(!vm.set_binding("nonexistent", Value::from(1i64)));
}

#[test]
fn set_binding_override() {
    let locals = vec![entry("x", false)];
    let program = fn_with_locals(vec![Bytecode::LoadLocal(0)], locals);
    let mut vm = Vm::new(program).unwrap();
    vm.set_binding("x", Value::from(1i64));
    vm.set_binding("x", Value::from(2i64)); // override
    let result = vm.run().unwrap();
    assert_eq!(result.tail(), &Value::Int(2));
}
