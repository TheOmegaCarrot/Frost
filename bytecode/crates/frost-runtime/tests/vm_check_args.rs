//! Tests for the declarative native-arg validation: `NativeFunction::check_args`
//! (the named, public guard), `Params` construction/validation and its arity,
//! and the `NativeFunction::checked` constructor that ties them together.

use std::collections::BTreeMap;
use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, EnumSet, FormatVersion, FrostError, FrostType,
    InvalidParams, NameEntry, NativeFunction, Param, Params, Value, Vm,
};

/// A throwaway native named `frob`, for testing `check_args` in isolation.
fn frob() -> NativeFunction {
    NativeFunction::new("frob", Arity::AtLeast(0), |_, _: &mut [Value]| {
        Ok(Value::Null)
    })
}

fn check(args: &[Value], params: impl IntoIterator<Item = Param>) -> Result<(), FrostError> {
    frob().check_args(args, &Params::try_new(params).unwrap())
}

// ============================================================
// check_args: the full, function-named error message
// ============================================================

#[test]
fn accepts_matching_type() {
    assert!(check(&[Value::Int(1)], [Param::of(FrostType::Int.into())]).is_ok());
}

#[test]
fn reports_function_name_type_and_position() {
    let err = check(&[Value::Int(5)], [Param::of(FrostType::Map.into())]).unwrap_err();
    assert_eq!(
        err.message(),
        "Function frob requires Map as argument 1, got Int"
    );
}

#[test]
fn lists_alternatives_and_the_label() {
    // The listed types always read in FrostType declaration order (the set is
    // canonical), regardless of the order the spec was written in.
    let params = [Param::of(FrostType::Array | FrostType::String).named("seq")];
    let err = check(&[Value::Int(5)], params).unwrap_err();
    assert_eq!(
        err.message(),
        "Function frob requires String or Array as argument 1 (seq), got Int"
    );
}

#[test]
fn named_category_sets_use_their_category_name() {
    // A set that exactly matches a named category reads as the category,
    // not the exhaustive type list.
    let err = check(&[Value::from("x")], [Param::of(FrostType::NUMERIC)]).unwrap_err();
    assert_eq!(
        err.message(),
        "Function frob requires Numeric as argument 1, got String"
    );
    let err = check(&[Value::Null], [Param::of(FrostType::STRUCTURED)]).unwrap_err();
    assert_eq!(
        err.message(),
        "Function frob requires Structured as argument 1, got Null"
    );
}

#[test]
fn reports_the_offending_position() {
    let params = [Param::any(), Param::of(FrostType::Int.into())];
    let err = check(&[Value::Null, Value::from("x")], params).unwrap_err();
    assert_eq!(
        err.message(),
        "Function frob requires Int as argument 2, got String"
    );
}

#[test]
fn any_accepts_everything() {
    for value in [Value::Int(1), Value::Null, Value::from("x")] {
        assert!(check(&[value], [Param::any()]).is_ok());
    }
}

#[test]
fn optional_absent_is_ok() {
    let params = [
        Param::of(FrostType::Int.into()),
        Param::of(FrostType::Int.into()).optional(),
    ];
    assert!(check(&[Value::Int(1)], params).is_ok());
}

#[test]
fn optional_present_is_still_checked() {
    let params = [
        Param::of(FrostType::Int.into()),
        Param::of(FrostType::Int.into()).optional(),
    ];
    let err = check(&[Value::Int(1), Value::from("x")], params).unwrap_err();
    assert_eq!(
        err.message(),
        "Function frob requires Int as argument 2, got String"
    );
}

// ============================================================
// Params construction: arity
// ============================================================

fn arity_of(params: impl IntoIterator<Item = Param>) -> Arity {
    Params::try_new(params).unwrap().arity()
}

#[test]
fn arity_all_required_is_exact() {
    let params = [Param::any(), Param::of(FrostType::Int.into())];
    assert!(matches!(arity_of(params), Arity::Exact(2)));
}

#[test]
fn arity_with_an_optional_is_between() {
    let params = [
        Param::of(FrostType::Int.into()),
        Param::of(FrostType::Int.into()).optional(),
    ];
    assert!(matches!(arity_of(params), Arity::Between(1, 2)));
}

#[test]
fn arity_empty_is_exact_zero() {
    assert!(matches!(arity_of([]), Arity::Exact(0)));
}

#[test]
fn arity_multiple_trailing_optionals() {
    let params = [
        Param::of(FrostType::Int.into()),
        Param::of(FrostType::Int.into()).optional(),
        Param::of(FrostType::Int.into()).optional(),
    ];
    assert!(matches!(arity_of(params), Arity::Between(1, 3)));
}

// ============================================================
// Params construction: validation
// ============================================================

#[test]
fn try_new_rejects_a_required_param_after_an_optional() {
    let result = Params::try_new([
        Param::of(FrostType::Int.into()),
        Param::of(FrostType::Int.into()).optional(),
        Param::of(FrostType::Int.into()), // required after optional: nonsensical
    ]);
    assert_eq!(
        result.unwrap_err(),
        InvalidParams::RequiredAfterOptional { index: 2 }
    );
}

#[test]
fn try_new_rejects_an_empty_type_set() {
    let result = Params::try_new([Param::any(), Param::of(EnumSet::empty())]);
    assert_eq!(
        result.unwrap_err(),
        InvalidParams::EmptyTypeSet { index: 1 }
    );
}

#[test]
fn invalid_params_messages_name_the_problem() {
    assert_eq!(
        InvalidParams::RequiredAfterOptional { index: 2 }.to_string(),
        "invalid param spec: required parameter at index 2 follows an optional one (optionals must be trailing)"
    );
    assert_eq!(
        InvalidParams::EmptyTypeSet { index: 1 }.to_string(),
        "invalid param spec: parameter at index 1 has an empty type set and accepts no value"
    );
}

#[test]
#[should_panic(expected = "optionals must be trailing")]
fn new_panics_on_a_required_param_after_an_optional() {
    // The slice itself is valid data; `Params::new` is where validation runs,
    // so calling it at runtime panics rather than failing to compile.
    const BAD: &[Param] = &[Param::any().optional(), Param::any()];
    let _ = Params::new(BAD);
}

#[test]
#[should_panic(expected = "type set is empty")]
fn new_panics_on_an_empty_type_set() {
    const BAD: &[Param] = &[Param::of(EnumSet::empty())];
    let _ = Params::new(BAD);
}

#[test]
fn new_in_a_const_item_validates_at_compile_time() {
    // The rejection twin of this test is the `compile_fail` doc-test on
    // `Params::new`: an invalid spec in a const item does not compile.
    const PARAMS: Params = Params::new(&[Param::any(), Param::any().optional()]);
    assert!(matches!(PARAMS.arity(), Arity::Between(1, 2)));
    assert_eq!(PARAMS.as_slice().len(), 2);
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
        version: FormatVersion,
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
    Vm::factory()
        .build(main.assert_trusted().close(captures).unwrap())
        .unwrap()
        .run()
        .map_err(|e| e.into_error())
        .map(|r| r.tail().clone())
}

#[test]
fn checked_native_emits_full_type_error() {
    // The constructor's wrapper runs `ctx.check_args` -> the function-named message.
    let frob = Value::NativeFunction(Arc::new(NativeFunction::checked(
        "frob",
        Params::try_new([Param::of(FrostType::Map.into())]).unwrap(),
        |_, _| Ok(Value::Null),
    )));
    let err = invoke(frob, vec![Value::Int(5)]).unwrap_err();
    assert_eq!(
        err.message(),
        "Function frob requires Map as argument 1, got Int"
    );
}

#[test]
fn checked_native_takes_arity_from_the_spec_and_runs_body_on_match() {
    // The static-spec path: a const item, as the globals are written.
    const PARAMS: Params = Params::new(&[Param::of(FrostType::NUMERIC)]);
    let identity = Value::NativeFunction(Arc::new(NativeFunction::checked(
        "identity",
        PARAMS,
        |_, args| Ok(args[0].clone()),
    )));
    // Correct call: spec arity (Exact(1)) and type pass, body runs.
    assert_eq!(
        invoke(identity.clone(), vec![Value::Int(7)]).unwrap(),
        Value::Int(7)
    );
    // Wrong arity is caught by the VM's arity check (from the spec), before the body.
    assert!(invoke(identity, vec![]).is_err());
}
