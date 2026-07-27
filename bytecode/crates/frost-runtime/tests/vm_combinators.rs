//! Tests for the `error`, `and_then`, and `or_else` globals.
//!
//! `error(payload)` raises: a String payload becomes the error's message; any other
//! value is carried as-is and surfaces intact through `try_call`'s `error` field.
//! `and_then`/`or_else` are the null-propagation combinators; presence means
//! `not null`, so falsy values like `false` count as present.
//!
//! (`call` and `try_call` are covered in `vm_call.rs` and `vm_errors.rs`.)

use std::collections::BTreeMap;
use std::sync::Arc;

mod common;

use common::{Pop, global_slot};
use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostError, FrostResult, MapKey, NameEntry,
    NativeCtx, NativeFunction, Value, Vm,
};

use Bytecode::*;

// ============================================================
// Helpers
// ============================================================

fn entry(name: &str) -> NameEntry {
    NameEntry {
        name: name.to_string(),
        exported: false,
    }
}

/// A standalone (no-capture) closure value.
fn func(name: &str, code: Vec<Bytecode>, arity: Arity, names: &[&str]) -> Value {
    let f = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: name.to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: names.iter().map(|n| entry(n)).collect(),
        num_captures: 0,
        arity,
    });
    Value::Closure(f.assert_trusted().into_closure().unwrap())
}

/// A native function `Value`.
fn native(
    name: &'static str,
    arity: Arity,
    f: impl Fn(NativeCtx<'_>, &mut [Value]) -> FrostResult + Send + Sync + 'static,
) -> Value {
    Value::NativeFunction(Arc::new(NativeFunction::new(name, arity, f)))
}

/// A native that raises if it is ever invoked; the "must not be called" sentinel.
fn boom() -> Value {
    native("boom", Arity::AtLeast(0), |_, _| {
        Err(FrostError::from_static("boom!"))
    })
}

/// `fn x -> x + 1` as a native. `Exact(1)` doubles as proof that the caller
/// passed exactly one argument.
fn plus_one() -> Value {
    native("plus_one", Arity::Exact(1), |_, args| {
        Ok(Value::Int(
            args[0].as_int().expect("plus_one wants an Int") + 1,
        ))
    })
}

fn fmap(pairs: Vec<(&str, Value)>) -> Value {
    Value::Map(
        pairs
            .into_iter()
            .map(|(k, v)| (MapKey::String(Arc::from(k.as_bytes())), v))
            .collect(),
    )
}

/// Build and run a top-level "main" that, after popping its own value, runs `body`.
/// `caps` are seated as captures (slot `i` is the i-th entry).
fn run_main(caps: Vec<(&str, Value)>, body: Vec<Bytecode>) -> Result<Value, FrostError> {
    let names: Vec<NameEntry> = caps.iter().map(|(n, _)| entry(n)).collect();
    let mut code = vec![Pop];
    code.extend(body);
    let main = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: names,
        num_captures: caps.len(),
        arity: Arity::Exact(0),
    });
    let map: BTreeMap<String, Value> = caps.into_iter().map(|(n, v)| (n.to_string(), v)).collect();
    Vm::factory()
        .build(main.assert_trusted().close(map).unwrap())
        .unwrap()
        .run()
        .map_err(|e| e.into_error())
        .map(|r| r.tail().clone())
}

/// `global(a, b)` through the full `LoadGlobal` + `Call` path.
fn binary(global: &str, a: Value, b: Value) -> Result<Value, FrostError> {
    run_main(
        vec![("a", a), ("b", b)],
        vec![
            LoadGlobal(global_slot(global)),
            LoadLocal(0),
            LoadLocal(1),
            Call(2),
        ],
    )
}

/// `error(payload)`, returning the raised error.
fn raise(payload: Value) -> FrostError {
    run_main(
        vec![("payload", payload)],
        vec![LoadGlobal(global_slot("error")), LoadLocal(0), Call(1)],
    )
    .unwrap_err()
}

/// `try_call(error, [payload])`: raise and catch in one program, returning the result map.
fn catch_raise(payload: Value) -> Value {
    run_main(
        vec![("payload", payload)],
        vec![
            LoadGlobal(global_slot("try_call")),
            LoadGlobal(global_slot("error")),
            LoadLocal(0),
            MakeArray(1),
            Call(2),
        ],
    )
    .unwrap()
}

// ============================================================
// error
// ============================================================

#[test]
fn error_raises_with_the_string_as_message() {
    let err = raise(Value::from("boom"));
    assert_eq!(err.message(), "boom");
}

#[test]
fn error_contributes_its_frame_to_the_backtrace() {
    let err = raise(Value::from("boom"));
    let frames: Vec<&str> = err.backtrace().iter().map(String::as_str).collect();
    assert_eq!(frames, vec!["error", "main"]);
}

#[test]
fn error_accepts_a_non_string_payload() {
    // Any value may be thrown; the message is its rendering.
    let err = raise(Value::Int(42));
    assert_eq!(err.message(), "42");
}

#[test]
fn error_string_payload_round_trips_through_try_call() {
    let result = catch_raise(Value::from("kaput"));
    let map = result.as_map().expect("try_call returns a Map");
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(false)));
    assert_eq!(map.get_str("error"), Some(&Value::from("kaput")));
}

#[test]
fn error_value_payload_survives_try_call_intact() {
    // A non-string payload reaches the catcher as the value itself, not a rendering.
    let payload = fmap(vec![("code", Value::Int(404))]);
    let result = catch_raise(payload.clone());
    let map = result.as_map().expect("try_call returns a Map");
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(false)));
    assert_eq!(map.get_str("error"), Some(&payload));
}

// ============================================================
// and_then
// ============================================================

#[test]
fn and_then_null_short_circuits_without_calling_f() {
    assert_eq!(
        binary("and_then", Value::Null, boom()).unwrap(),
        Value::Null
    );
}

#[test]
fn and_then_passes_the_value_to_f_and_returns_its_result() {
    assert_eq!(
        binary("and_then", Value::Int(41), plus_one()).unwrap(),
        Value::Int(42)
    );
}

#[test]
fn and_then_treats_false_as_present() {
    // Presence is `not null`, not truthiness.
    let k = native("konst", Arity::Exact(1), |_, _| Ok(Value::from("ran")));
    assert_eq!(
        binary("and_then", Value::Bool(false), k).unwrap(),
        Value::from("ran")
    );
}

#[test]
fn and_then_reenters_a_closure_callback() {
    // fn x -> x + x, as compiled code: proves the combinator re-enters the Vm.
    let double = func(
        "double",
        vec![DefLocal(0), Pop, LoadLocal(0), LoadLocal(0), Add],
        Arity::Exact(1),
        &["x"],
    );
    assert_eq!(
        binary("and_then", Value::Int(21), double).unwrap(),
        Value::Int(42)
    );
}

#[test]
fn and_then_propagates_an_error_from_f() {
    let err = binary("and_then", Value::Int(1), boom()).unwrap_err();
    assert_eq!(err.message(), "boom!");
}

#[test]
fn and_then_requires_a_function_second_arg() {
    // Checked even when the value is null and f would never run.
    let err = binary("and_then", Value::Null, Value::Int(2)).unwrap_err();
    assert!(
        err.message()
            .contains("and_then requires Function as argument 2"),
        "got: {}",
        err.message()
    );
}

// ============================================================
// or_else
// ============================================================

#[test]
fn or_else_returns_a_nonnull_value_without_calling_f() {
    assert_eq!(
        binary("or_else", Value::Int(5), boom()).unwrap(),
        Value::Int(5)
    );
}

#[test]
fn or_else_keeps_falsy_nonnull_values() {
    // Presence is `not null`: false is not replaced by the fallback.
    assert_eq!(
        binary("or_else", Value::Bool(false), boom()).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn or_else_calls_f_with_no_args_when_null() {
    // Exact(0) doubles as proof that the fallback is invoked with no arguments.
    let fallback = native("fallback", Arity::Exact(0), |_, _| Ok(Value::Int(7)));
    assert_eq!(
        binary("or_else", Value::Null, fallback).unwrap(),
        Value::Int(7)
    );
}

#[test]
fn or_else_fallback_may_return_null() {
    let fallback = native("fallback", Arity::Exact(0), |_, _| Ok(Value::Null));
    assert_eq!(
        binary("or_else", Value::Null, fallback).unwrap(),
        Value::Null
    );
}

#[test]
fn or_else_reenters_a_closure_fallback() {
    let const99 = func("const99", vec![Pop, PushInt(99)], Arity::Exact(0), &[]);
    assert_eq!(
        binary("or_else", Value::Null, const99).unwrap(),
        Value::Int(99)
    );
}

#[test]
fn or_else_propagates_an_error_from_f() {
    let err = binary("or_else", Value::Null, boom()).unwrap_err();
    assert_eq!(err.message(), "boom!");
}

#[test]
fn or_else_requires_a_function_second_arg() {
    // Checked even when the value is present and the fallback would never run.
    let err = binary("or_else", Value::Int(1), Value::Int(2)).unwrap_err();
    assert!(
        err.message()
            .contains("or_else requires Function as argument 2"),
        "got: {}",
        err.message()
    );
}

// ============================================================
// Arity
// ============================================================

#[test]
fn combinator_globals_enforce_arity() {
    // error: Exact(1)
    assert!(run_main(vec![], vec![LoadGlobal(global_slot("error")), Call(0)]).is_err());
    assert!(binary("error", Value::Int(1), Value::Int(2)).is_err());

    // and_then / or_else: Exact(2)
    for name in ["and_then", "or_else"] {
        assert!(
            run_main(vec![], vec![LoadGlobal(global_slot(name)), Call(0)]).is_err(),
            "{name} with 0 args"
        );
        assert!(
            run_main(
                vec![("a", Value::Int(1))],
                vec![LoadGlobal(global_slot(name)), LoadLocal(0), Call(1)],
            )
            .is_err(),
            "{name} with 1 arg"
        );
        assert!(
            run_main(
                vec![
                    ("a", Value::Int(1)),
                    ("b", plus_one()),
                    ("c", Value::Int(3)),
                ],
                vec![
                    LoadGlobal(global_slot(name)),
                    LoadLocal(0),
                    LoadLocal(1),
                    LoadLocal(2),
                    Call(3),
                ],
            )
            .is_err(),
            "{name} with 3 args"
        );
    }
}
