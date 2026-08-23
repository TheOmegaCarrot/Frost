//! Tests for the `Concat(N)` opcode.
//!
//! `Concat(N)` `( x1 ... xN -- s )` stringifies the top N stack items with the
//! compact Frost `to_string` form (top-level Strings unquoted) and concatenates
//! them deepest-first, so the top of the stack becomes the last component. It
//! consumes exactly N items and leaves anything below them untouched. N is nonzero.

use std::num::NonZeroUsize;
use std::sync::Arc;

use frost_runtime::{Arity, Bytecode, CompiledFunction, FormatVersion, FrostError, Value, Vm};

/// Run a constants-carrying, local-less top-level program and return its tail value.
fn eval(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<concat>".to_string(),
        code: body,
        child_fns: Vec::new(),
        constants,
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let closure = program.assert_trusted().into_closure().unwrap();
    Vm::factory()
        .build(closure)
        .unwrap()
        .run()
        .map_err(|e| e.into_error())
        .map(|r| r.tail().clone())
}

fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}

// ---- Ordering and the stack effect ----

#[test]
fn concatenates_deepest_first_and_leaves_lower_untouched() {
    // [1, 2, 3, 4, 5], Concat(3) -> [1, 2, "345"]: top three joined deepest-first
    // (top last), the two below untouched. MakeArray makes the whole result observable.
    let r = eval(
        vec![],
        vec![
            Bytecode::PushInt(1),
            Bytecode::PushInt(2),
            Bytecode::PushInt(3),
            Bytecode::PushInt(4),
            Bytecode::PushInt(5),
            Bytecode::Concat(nz(3)),
            Bytecode::MakeArray(3),
        ],
    );
    assert_eq!(
        r.unwrap(),
        Value::array([Value::from(1i64), Value::from(2i64), Value::from("345")])
    );
}

#[test]
fn concat_one_stringifies_a_single_value() {
    let r = eval(vec![], vec![Bytecode::PushInt(42), Bytecode::Concat(nz(1))]);
    assert_eq!(r.unwrap(), Value::from("42"));
}

#[test]
fn concat_one_leaves_the_rest_of_the_stack() {
    // [10, 20, 30], Concat(1) -> [10, 20, "30"].
    let r = eval(
        vec![],
        vec![
            Bytecode::PushInt(10),
            Bytecode::PushInt(20),
            Bytecode::PushInt(30),
            Bytecode::Concat(nz(1)),
            Bytecode::MakeArray(3),
        ],
    );
    assert_eq!(
        r.unwrap(),
        Value::array([Value::from(10i64), Value::from(20i64), Value::from("30")])
    );
}

#[test]
fn concat_consumes_the_whole_stack() {
    let r = eval(
        vec![],
        vec![
            Bytecode::PushInt(1),
            Bytecode::PushInt(2),
            Bytecode::PushInt(3),
            Bytecode::Concat(nz(3)),
        ],
    );
    assert_eq!(r.unwrap(), Value::from("123"));
}

// ---- Stringification conventions ----

#[test]
fn strings_concatenate_unquoted() {
    let r = eval(
        vec![Value::from("foo"), Value::from("bar")],
        vec![
            Bytecode::LoadConst(0),
            Bytecode::LoadConst(1),
            Bytecode::Concat(nz(2)),
        ],
    );
    assert_eq!(r.unwrap(), Value::from("foobar"));
}

#[test]
fn mixed_types_use_compact_forms() {
    // "a" (unquoted), 1, 2.5 (float form), true -> "a12.5true".
    let float: Value = 2.5.try_into().unwrap();
    let r = eval(
        vec![Value::from("a"), float],
        vec![
            Bytecode::LoadConst(0), // "a"
            Bytecode::PushInt(1),
            Bytecode::LoadConst(1), // 2.5
            Bytecode::PushTrue,
            Bytecode::Concat(nz(4)),
        ],
    );
    assert_eq!(r.unwrap(), Value::from("a12.5true"));
}
