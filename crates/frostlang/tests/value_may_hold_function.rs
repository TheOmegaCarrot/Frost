//! `Value::may_hold_function`: whether a Function is, or may be, reachable from
//! a value.

use crate::script::Script;

use frostlang::compile::OptimizationOptions;
use frostlang::{FrostOpaque, SpecialFloat, Value, frost_closure};

/// An Opaque payload with no drop glue, which provably holds no Value.
#[derive(Debug)]
struct Plain(#[allow(dead_code)] u64);

impl FrostOpaque for Plain {
    fn type_name() -> &'static str {
        "Plain"
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

/// An Opaque payload owning heap data, which might be a Value.
#[derive(Debug)]
struct Owning(#[allow(dead_code)] Vec<u8>);

impl FrostOpaque for Owning {
    fn type_name() -> &'static str {
        "Owning"
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

/// A Frost closure, made by a script. One run only: closures from separate
/// runs are never equal.
fn closure() -> Value {
    Script::new("fn -> 1").run_under(OptimizationOptions::ALL)
}

/// A native Function.
fn native() -> Value {
    frost_closure!("native", || 1)
}

#[test]
fn a_function_is_one() {
    assert!(closure().may_hold_function(), "a closure");
    assert!(native().may_hold_function(), "a native");
}

#[test]
fn a_value_holding_none_holds_none() {
    for value in [
        Value::Null,
        Value::Bool(true),
        Value::Int(1),
        Value::from_f64(1.5),
        Value::from("text"),
        Value::from(&b"bytes"[..]),
        Value::array([Value::Int(1), Value::array([Value::from("deep")])]),
        Value::map([("a", Value::map([("b", Value::Null)]))]),
        Value::opaque(Plain(1)),
        Value::opaque(SpecialFloat::new(f64::NAN).unwrap()),
    ] {
        assert!(!value.may_hold_function(), "{value:?}");
    }
}

#[test]
fn a_function_held_at_any_depth_is_found() {
    for function in [closure(), native()] {
        for value in [
            Value::array([function.clone()]),
            Value::array([Value::Int(1), Value::array([Value::Null, function.clone()])]),
            Value::map([("a", function.clone())]),
            Value::map([("a", Value::array([Value::map([("b", function.clone())])]))]),
        ] {
            assert!(value.may_hold_function(), "{value:?}");
        }
    }
}

#[test]
fn an_opaque_that_might_hold_a_value_counts() {
    assert!(Value::opaque(Owning(vec![1])).may_hold_function());
    assert!(
        Value::array([Value::map([("a", Value::opaque(Owning(Vec::new())))])]).may_hold_function()
    );
}

#[test]
fn a_value_sharing_its_contents_is_searched_whole() {
    // Each level holds the one below twice, so a search revisiting shared
    // contents would take 2^64 steps.
    let deep = (0..64).fold(Value::Int(1), |inner, _| {
        Value::array([inner.clone(), inner])
    });
    assert!(!deep.may_hold_function());
    let with_function = Value::array([deep, native()]);
    assert!(with_function.may_hold_function());
}
