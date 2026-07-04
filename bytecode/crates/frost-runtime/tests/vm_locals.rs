mod common;

use common::{entry, fn_with_locals, run_fn};
use frost_runtime::{Bytecode, Value};

// ============================================================
// Local slots
// ============================================================

#[test]
fn def_local_stores_and_load_local_retrieves() {
    let program = fn_with_locals(
        vec![
            Bytecode::PushInt(99),
            Bytecode::DefLocal(0),
            Bytecode::LoadLocal(0),
        ],
        vec![entry("x", false)],
    );
    assert_eq!(run_fn(program).tail(), &Value::Int(99));
}

#[test]
fn def_local_moves_off_stack() {
    // After DefLocal, the value is no longer on the stack -- only the 1 remains.
    let program = fn_with_locals(
        vec![
            Bytecode::PushInt(1),
            Bytecode::PushInt(2),
            Bytecode::DefLocal(0),
        ],
        vec![entry("x", false)],
    );
    assert_eq!(run_fn(program).tail(), &Value::Int(1));
}

#[test]
fn load_local_copies_not_moves() {
    // LoadLocal copies (clones), so loading the same slot twice works: 5 + 5 = 10
    // proves both loads read the value (a move would have emptied the slot).
    let program = fn_with_locals(
        vec![
            Bytecode::PushInt(5),
            Bytecode::DefLocal(0),
            Bytecode::LoadLocal(0),
            Bytecode::LoadLocal(0),
            Bytecode::Add,
        ],
        vec![entry("x", false)],
    );
    assert_eq!(run_fn(program).tail(), &Value::Int(10));
}
