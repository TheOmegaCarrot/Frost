//! The in-memory host bridge (`to_value` / `from_value`): a `Value`-typed field is a true
//! catch-all that round-trips any Value, Functions and Opaques included. A foreign format
//! still refuses Functions, even nested, and treats the bridge's marker as a transparent
//! newtype so ordinary data is unaffected.

mod common;

use std::borrow::Cow;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use frost_runtime::{Arity, FrostOpaque, NativeFunction, Value, from_value, to_value};

fn native_fn() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction::new(
        "f",
        Arity::Exact(0),
        |_ctx, _args| Ok(Value::Null),
    )))
}

#[derive(Debug)]
struct Widget;

impl FrostOpaque for Widget {
    fn type_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Widget")
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

#[derive(Serialize, Deserialize)]
struct Holder {
    foo: i64,
    bar: Value,
}

#[derive(Serialize, Deserialize)]
struct OneValue {
    bar: Value,
}

// -- The catch-all field round-trips anything --

#[test]
fn value_field_round_trips_a_function() {
    // The law's example: {foo: 42, bar: <fn>} through a struct with a Value field and back
    // to an equal Map. Function values compare by identity, and the value flows through by
    // Arc clone, so the round-trip is equal.
    let map = Value::map([("foo", Value::Int(42)), ("bar", native_fn())]);

    let holder: Holder = from_value(map.clone()).unwrap();
    assert_eq!(holder.foo, 42);
    assert!(holder.bar.is_function());

    assert_eq!(to_value(&holder).unwrap(), map);
}

#[test]
fn value_field_round_trips_a_closure() {
    // L9 separates Closure from NativeFunction; both must survive the bridge. The value is
    // carried, never run, so a bare compiled function suffices.
    let map = Value::map([("bar", Value::Closure(common::closure(vec![], vec![])))]);

    let holder: OneValue = from_value(map.clone()).unwrap();
    assert!(holder.bar.is_function());

    assert_eq!(to_value(&holder).unwrap(), map);
}

#[test]
fn value_field_round_trips_an_opaque() {
    let map = Value::map([("bar", Value::opaque(Widget))]);

    let holder: OneValue = from_value(map.clone()).unwrap();
    assert!(holder.bar.is_opaque());

    assert_eq!(to_value(&holder).unwrap(), map);
}

#[test]
fn value_field_round_trips_a_function_nested_in_a_structure() {
    // The slot carries the whole Value, so a function buried inside a structured value in
    // the field survives, not only a function that is the field itself.
    let inner = Value::map([("fn", native_fn()), ("n", Value::Int(1))]);
    let map = Value::map([("bar", inner)]);

    let holder: OneValue = from_value(map.clone()).unwrap();
    assert!(holder.bar.is_map());

    assert_eq!(to_value(&holder).unwrap(), map);
}

#[test]
fn value_field_round_trips_ordinary_data() {
    // The common case must be unchanged by the marker path.
    let map = Value::map([("foo", Value::Int(1)), ("bar", Value::from("hi"))]);

    let holder: Holder = from_value(map.clone()).unwrap();
    assert_eq!(holder.bar, Value::from("hi"));

    assert_eq!(to_value(&holder).unwrap(), map);
}

// -- Ignored fields are skipped without inspection --

#[derive(Deserialize)]
struct DataOnly {
    foo: i64,
}

#[test]
fn an_unlisted_function_field_is_skipped() {
    // A target that does not name the function-valued field skips it, rather than failing
    // to deserialize the fields it does want.
    let map = Value::map([("foo", Value::Int(7)), ("extra", native_fn())]);

    let data: DataOnly = from_value(map).unwrap();
    assert_eq!(data.foo, 7);
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // `foo` exists to shape the derive; this case only asserts rejection.
struct StrictData {
    foo: i64,
}

#[test]
fn deny_unknown_fields_rejects_an_extra_function_field() {
    // Unlike the skip case, `deny_unknown_fields` rejects on the field name, before the
    // function value is ever inspected.
    let map = Value::map([("foo", Value::Int(1)), ("extra", native_fn())]);

    assert!(from_value::<StrictData>(map).is_err());
}

// -- A foreign format is unaffected, and still refuses Functions --

#[test]
fn foreign_format_serializes_data_transparently() {
    // The marker newtype adds no wrapper: a data Value is JSON like any other value.
    assert_eq!(serde_json::to_string(&Value::Int(42)).unwrap(), "42");
    assert_eq!(
        serde_json::from_str::<Value>("[1,2,3]").unwrap(),
        Value::array([Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn foreign_format_refuses_a_function() {
    assert!(serde_json::to_string(&native_fn()).is_err());
}

#[test]
fn foreign_format_refuses_a_nested_function() {
    // The external-format rule holds at any nesting depth.
    let nested = Value::array([native_fn()]);
    assert!(serde_json::to_string(&nested).is_err());
}

#[test]
fn foreign_format_refuses_a_function_in_a_map_value() {
    // A Map value goes through a different serializer path than an Array element; it must
    // refuse a Function just the same.
    let map = Value::map([("fn", native_fn())]);
    assert!(serde_json::to_string(&map).is_err());
}
