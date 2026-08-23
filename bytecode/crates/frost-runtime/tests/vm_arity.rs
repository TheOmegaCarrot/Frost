//! Tests for `Arity::Between`: the bounded (min..=max) arity.
//!
//! Scope is the arity *check* as enforced for native functions (which receive a
//! slice, so bounded arity needs no slot seating). The closure seating path
//! (materializing absent optional slots) is separate; see the note at the bottom.

use std::collections::BTreeMap;
use std::sync::Arc;

mod common;

use common::Pop;
use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostError, FrostResult, NameEntry,
    ProgramResult, Value, Vm,
};

/// A native that returns its own argument count, so a test can prove every passed
/// arg actually reached the slice (not just that the call was accepted).
fn counting(arity: Arity) -> Value {
    Value::NativeFunction(Arc::new(frost_runtime::NativeFunction::new(
        "counter",
        arity,
        |_ctx, args: &mut [Value]| -> FrostResult { Ok(Value::Int(args.len() as i64)) },
    )))
}

/// Call `native` with `argc` integer arguments (`0..argc`), returning the run
/// result so both the success and arity-error paths are observable.
fn call_native(native: Value, argc: usize) -> Result<ProgramResult, FrostError> {
    let mut code = vec![Bytecode::Pop, Bytecode::LoadLocal(0)]; // pop own closure value, load the native (capture slot 0)
    for i in 0..argc {
        code.push(Bytecode::PushInt(i as i64));
    }
    code.push(Bytecode::Call(argc));

    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<test>".to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: vec![NameEntry {
            name: "f".to_string(),
            exported: false,
        }],
        num_captures: 1,
        arity: Arity::Exact(0),
    });
    let closure = program
        .assert_trusted()
        .close(BTreeMap::from([("f".to_string(), native)]))
        .unwrap();
    Vm::factory()
        .build(closure)
        .unwrap()
        .run()
        .map_err(|e| e.into_error())
}

#[test]
fn between_accepts_lower_bound() {
    let r = call_native(counting(Arity::Between(1, 2)), 1).unwrap();
    assert_eq!(r.tail(), &Value::Int(1));
}

#[test]
fn between_accepts_upper_bound() {
    let r = call_native(counting(Arity::Between(1, 2)), 2).unwrap();
    assert_eq!(r.tail(), &Value::Int(2));
}

#[test]
fn between_accepts_interior_count() {
    // A wider range (1..=3) must also accept a count strictly between the bounds.
    let r = call_native(counting(Arity::Between(1, 3)), 2).unwrap();
    assert_eq!(r.tail(), &Value::Int(2));
}

#[test]
fn between_below_lower_bound_is_arity_error() {
    let err = call_native(counting(Arity::Between(1, 2)), 0).unwrap_err();
    assert!(
        err.message().contains("between 1 and 2"),
        "got: {}",
        err.message()
    );
    assert!(
        err.message().contains("called with 0"),
        "got: {}",
        err.message()
    );
}

#[test]
fn between_above_upper_bound_is_arity_error() {
    let err = call_native(counting(Arity::Between(1, 2)), 3).unwrap_err();
    assert!(
        err.message().contains("between 1 and 2"),
        "got: {}",
        err.message()
    );
    assert!(
        err.message().contains("called with 3"),
        "got: {}",
        err.message()
    );
}

#[test]
fn between_equal_bounds_behaves_like_exact() {
    // A degenerate range Between(2, 2) accepts exactly 2 (the bounds are inclusive
    // on both ends) and rejects 1 and 3.
    assert_eq!(
        call_native(counting(Arity::Between(2, 2)), 2)
            .unwrap()
            .tail(),
        &Value::Int(2)
    );
    assert!(call_native(counting(Arity::Between(2, 2)), 1).is_err());
    assert!(call_native(counting(Arity::Between(2, 2)), 3).is_err());
}

// ============================================================
// Between closure seating: push_closure_frame hands a hand-rolled Between closure
// its actual `argc` on top of the args, so the prelude can branch on the real count.
// ============================================================

/// A hand-rolled `Between(0, 1)` closure that consumes the pushed `argc` to tell an
/// omitted optional from one explicitly passed as `null`: 0 args -> `Int(-1)`
/// sentinel; 1 arg -> the argument itself (so an explicit `null` stays `null`).
/// This distinction is only possible *because* `argc` is on the stack: the slot
/// value alone cannot tell "absent" from "null".
fn omitted_vs_present_probe() -> Arc<CompiledFunction> {
    use Bytecode::*;
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "probe".to_string(),
        // Stack on entry: [ closure, (x?), argc ].
        code: vec![
            PushInt(1),    //  0: [.., argc, 1]
            CompareEqual,  //  1: argc == 1 ?  -> [.., bool]
            PeekJumpIfTrue(4), //  2: -> present branch (idx 7); bool not consumed
            Pop,           //  3: absent: drop the bool
            Pop,           //  4: drop the closure value at base
            PushInt(-1),   //  5: result sentinel for "omitted"
            Jump(4),       //  6: -> end (past idx 10)
            Pop,           //  7: present: drop the bool
            DefLocal(0),   //  8: x -> slot 0
            Pop,           //  9: drop the closure value at base
            LoadLocal(0),  // 10: result is x (an explicit null stays null)
        ],
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: vec![NameEntry {
            name: "x".to_string(),
            exported: false,
        }],
        num_captures: 0,
        arity: Arity::Between(0, 1),
    })
}

#[test]
fn between_closure_distinguishes_omitted_from_explicit_null() {
    let probe = omitted_vs_present_probe();
    let run = |args: Vec<Value>| {
        Vm::factory()
            .build(probe.clone().assert_trusted().into_closure().unwrap())
            .unwrap()
            .run_with_args(args)
            .unwrap()
            .tail()
            .clone()
    };
    assert_eq!(run(vec![]), Value::Int(-1)); // omitted
    assert_eq!(run(vec![Value::Null]), Value::Null); // present, explicitly null
    assert_eq!(run(vec![Value::Int(7)]), Value::Int(7)); // present, value preserved
}

#[test]
fn between_closure_seating_works_through_call() {
    // The same probe invoked via `Call` from a wrapper (base != 0); proves the
    // argc-push is correct on the re-entrant call path, not only at the top level.
    let probe_val = Value::Closure(
        omitted_vs_present_probe()
            .assert_trusted()
            .into_closure()
            .unwrap(),
    );
    let call_probe = |arg_pushes: Vec<Bytecode>, argc: usize| -> Value {
        let mut code = vec![Bytecode::Pop, Bytecode::LoadLocal(0)]; // pop own closure, load probe (capture 0)
        code.extend(arg_pushes);
        code.push(Bytecode::Call(argc));
        let wrapper = Arc::new(CompiledFunction {
            version: FormatVersion,
            name: "wrapper".to_string(),
            code,
            child_fns: Vec::new(),
            constants: Vec::new(),
            key_constants: Vec::new(),
            name_table: vec![NameEntry {
                name: "probe".to_string(),
                exported: false,
            }],
            num_captures: 1,
            arity: Arity::Exact(0),
        });
        let closure = wrapper
            .assert_trusted()
            .close(BTreeMap::from([("probe".to_string(), probe_val.clone())]))
            .unwrap();
        Vm::factory()
            .build(closure)
            .unwrap()
            .run()
            .unwrap()
            .tail()
            .clone()
    };
    assert_eq!(call_probe(vec![], 0), Value::Int(-1)); // probe() via Call
    assert_eq!(call_probe(vec![Bytecode::PushNull], 1), Value::Null); // probe(null) via Call
}
