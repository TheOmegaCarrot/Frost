use crate::common;

use std::collections::BTreeMap;
use std::sync::Arc;

use common::{entry, fn_with_locals, func, run_fn};
use frostlang::bytecode::{Bytecode, CompiledFunction, FormatVersion};
use frostlang::{Arity, Value, Vm};

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
    // After DefLocal, the value is no longer on the stack; only the 1 remains.
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

#[test]
fn consume_local_pushes_the_slots_value() {
    let program = fn_with_locals(
        vec![
            Bytecode::PushInt(7),
            Bytecode::DefLocal(0),
            Bytecode::ConsumeLocal(0),
        ],
        vec![entry("x", false)],
    );
    assert_eq!(run_fn(program).tail(), &Value::Int(7));
}

#[test]
fn a_consumed_slot_may_be_defined_and_read_again() {
    let program = fn_with_locals(
        vec![
            Bytecode::PushInt(1),
            Bytecode::DefLocal(0),
            Bytecode::ConsumeLocal(0),
            Bytecode::PushInt(2),
            Bytecode::DefLocal(0),
            Bytecode::LoadLocal(0),
            Bytecode::Add,
        ],
        vec![entry("x", false)],
    );
    assert_eq!(run_fn(program).tail(), &Value::Int(3));
}

/// Store an Array in a local, then pass it, loaded with `load`, to a native
/// that reports whether it holds the Array's only reference.
fn passed_unshared(load: Bytecode) -> Value {
    let probe = Value::native("probe", Arity::Exact(1), |_, args| {
        let array = args[0].take().try_into_array().expect("an Array");
        Ok(Value::Bool(array.try_into_vec().is_ok()))
    });
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<test>".to_string(),
        origin: None,
        code: vec![
            Bytecode::Pop,
            Bytecode::PushInt(1),
            Bytecode::MakeArray(1),
            Bytecode::DefLocal(1),
            Bytecode::LoadLocal(0),
            load,
            Bytecode::Call(1),
        ],
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: vec![entry("probe", false), entry("xs", false)],
        num_captures: 1,
        arity: Arity::Exact(0),
    });
    let closure = program
        .assert_trusted()
        .close(BTreeMap::from([("probe".to_string(), probe)]))
        .unwrap();
    Vm::factory().build(closure).run().unwrap().tail().clone()
}

#[test]
fn a_consumed_value_is_no_longer_shared_with_its_slot() {
    assert_eq!(
        passed_unshared(Bytecode::ConsumeLocal(1)),
        Value::Bool(true),
        "consumed: the slot gives up its reference"
    );
    assert_eq!(
        passed_unshared(Bytecode::LoadLocal(1)),
        Value::Bool(false),
        "loaded: the slot keeps its reference"
    );
}

#[test]
#[should_panic(expected = "local value is undefined")]
fn a_call_never_sees_a_finished_calls_locals() {
    // `define` fills its slot 0 and returns; `read` then runs in the same place
    // and reads its own slot 0, which it never defined. It must find it empty,
    // not holding `define`'s 5.
    let define = func(
        vec![
            Bytecode::Pop,
            Bytecode::PushInt(5),
            Bytecode::DefLocal(0),
            Bytecode::PushNull,
        ],
        Arity::Exact(0),
        vec![entry("x", false)],
        vec![],
    );
    let read = func(
        vec![Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::Exact(0),
        vec![entry("x", false)],
        vec![],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure(0),
            Bytecode::Call(0),
            Bytecode::Pop,
            Bytecode::CreateClosure(1),
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![define, read],
    );
    run_fn(program);
}
