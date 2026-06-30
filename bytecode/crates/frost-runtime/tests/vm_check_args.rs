//! Tests for the declarative native-arg validation: `NativeFunction::check_args`
//! (the named, public guard), the `[Param]::arity()` derivation, and the
//! `NativeFunction::checked` constructor that ties them together.

use std::collections::BTreeMap;
use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FrostError, FrostType, NameEntry, NativeFunction, Param,
    ParamSpec, Value, Vm,
};

/// A throwaway native named `frob`, for testing `check_args` in isolation.
fn frob() -> NativeFunction {
    NativeFunction::new(
        "frob",
        Arity::AtLeast(0),
        |_, _: &mut [Value]| Ok(Value::Null),
    )
}

fn check(args: &[Value], params: &[Param]) -> Result<(), FrostError> {
    frob().check_args(args, params)
}

// ============================================================
// check_args: the full, function-named error message
// ============================================================

#[test]
fn accepts_matching_type() {
    assert!(check(&[Value::Int(1)], &[Param::of(&[FrostType::Int])]).is_ok());
}

#[test]
fn reports_function_name_type_and_position() {
    let err = check(&[Value::Int(5)], &[Param::of(&[FrostType::Map])]).unwrap_err();
    assert_eq!(
        err.message,
        "Function frob requires Map as argument 1, got Int"
    );
}

#[test]
fn lists_alternatives_and_the_label() {
    let params = &[Param::of(&[FrostType::Array, FrostType::String]).named("seq")];
    let err = check(&[Value::Int(5)], params).unwrap_err();
    assert_eq!(
        err.message,
        "Function frob requires Array or String as argument 1 (seq), got Int"
    );
}

#[test]
fn reports_the_offending_position() {
    let params = &[Param::any(), Param::of(&[FrostType::Int])];
    let err = check(&[Value::Null, Value::from("x")], params).unwrap_err();
    assert_eq!(
        err.message,
        "Function frob requires Int as argument 2, got String"
    );
}

#[test]
fn any_accepts_everything() {
    let params = &[Param::any()];
    assert!(check(&[Value::Int(1)], params).is_ok());
    assert!(check(&[Value::Null], params).is_ok());
    assert!(check(&[Value::from("x")], params).is_ok());
}

#[test]
fn optional_absent_is_ok() {
    let params = &[
        Param::of(&[FrostType::Int]),
        Param::of(&[FrostType::Int]).optional(),
    ];
    assert!(check(&[Value::Int(1)], params).is_ok());
}

#[test]
fn optional_present_is_still_checked() {
    let params = &[
        Param::of(&[FrostType::Int]),
        Param::of(&[FrostType::Int]).optional(),
    ];
    let err = check(&[Value::Int(1), Value::from("x")], params).unwrap_err();
    assert_eq!(
        err.message,
        "Function frob requires Int as argument 2, got String"
    );
}

// ============================================================
// Arity derivation from the spec
// ============================================================

#[test]
fn arity_all_required_is_exact() {
    let params = [Param::any(), Param::of(&[FrostType::Int])];
    assert!(matches!(params.arity(), Arity::Exact(2)));
}

#[test]
fn arity_with_an_optional_is_between() {
    let params = [
        Param::of(&[FrostType::Int]),
        Param::of(&[FrostType::Int]).optional(),
    ];
    assert!(matches!(params.arity(), Arity::Between(1, 2)));
}

#[test]
fn arity_empty_is_exact_zero() {
    let params: &[Param] = &[];
    assert!(matches!(params.arity(), Arity::Exact(0)));
}

#[test]
fn arity_multiple_trailing_optionals() {
    let params = [
        Param::of(&[FrostType::Int]),
        Param::of(&[FrostType::Int]).optional(),
        Param::of(&[FrostType::Int]).optional(),
    ];
    assert!(matches!(params.arity(), Arity::Between(1, 3)));
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "optionals must be trailing")]
fn arity_rejects_a_required_param_after_an_optional() {
    let params = [
        Param::of(&[FrostType::Int]),
        Param::of(&[FrostType::Int]).optional(),
        Param::of(&[FrostType::Int]), // required after optional -- nonsensical
    ];
    let _ = params.arity();
}

// ============================================================
// NativeFunction::checked end-to-end (through the VM)
// ============================================================

fn entry(name: &str) -> NameEntry {
    NameEntry {
        name: name.to_string(),
        exported: false,
    }
}

/// Invoke `native` with `args` through the VM (both seated as captures).
fn invoke(native: Value, args: Vec<Value>) -> Result<Value, FrostError> {
    let argc = args.len();
    let mut code = vec![Bytecode::Pop, Bytecode::LoadLocal(0)]; // pop own value, load the native
    code.extend((1..=argc).map(Bytecode::LoadLocal)); // args at capture slots 1..=argc
    code.push(Bytecode::Call(argc));

    let mut name_table = vec![entry("f")];
    name_table.extend((0..argc).map(|i| entry(&format!("a{i}"))));

    let main = Arc::new(CompiledFunction {
        name: "main".to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table,
        num_captures: 1 + argc,
        arity: Arity::Exact(0),
    });

    let mut captures = BTreeMap::new();
    captures.insert("f".to_string(), native);
    for (i, v) in args.into_iter().enumerate() {
        captures.insert(format!("a{i}"), v);
    }
    Vm::new(main.close(captures).unwrap())
        .unwrap()
        .run()
        .map(|r| r.tail().clone())
}

#[test]
fn checked_native_emits_full_type_error() {
    // The constructor's wrapper runs `ctx.check_args` -> the function-named message.
    let frob = Value::NativeFunction(Arc::new(NativeFunction::checked(
        "frob",
        [Param::of(&[FrostType::Map])],
        |_, _| Ok(Value::Null),
    )));
    let err = invoke(frob, vec![Value::Int(5)]).unwrap_err();
    assert_eq!(
        err.message,
        "Function frob requires Map as argument 1, got Int"
    );
}

#[test]
fn checked_native_derives_arity_and_runs_body_on_match() {
    let identity = Value::NativeFunction(Arc::new(NativeFunction::checked(
        "identity",
        [Param::of(&[FrostType::Int])],
        |_, args| Ok(args[0].clone()),
    )));
    // Correct call: derived arity (Exact(1)) and type pass, body runs.
    assert_eq!(
        invoke(identity.clone(), vec![Value::Int(7)]).unwrap(),
        Value::Int(7)
    );
    // Wrong arity is caught by the VM's arity check (derived from the spec), before the body.
    assert!(invoke(identity, vec![]).is_err());
}
