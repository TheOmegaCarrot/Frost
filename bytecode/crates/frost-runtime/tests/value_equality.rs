use std::borrow::Cow;
use std::sync::Arc;

use frost_runtime::{
    Arity, Closure, CompiledFunction, FormatVersion, FrostArray, FrostMap, FrostOpaque, MapKey,
    NativeFunction, Value,
};

/// A minimal opaque payload for identity-equality tests.
#[derive(Debug)]
struct Marker(i64);

impl FrostOpaque for Marker {
    fn type_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Marker")
    }

    fn try_to_string(&self) -> Option<String> {
        Some(format!("marker {}", self.0))
    }
}

fn str_key(s: &str) -> MapKey {
    MapKey::String(Arc::from(s))
}

// -- Same-type equality --

#[test]
fn null_equals_null() {
    assert_eq!(Value::Null, Value::Null);
}

#[test]
fn bool_equality() {
    assert_eq!(Value::from(true), Value::from(true));
    assert_eq!(Value::from(false), Value::from(false));
    assert_ne!(Value::from(true), Value::from(false));
}

#[test]
fn int_equality() {
    assert_eq!(Value::from(42i64), Value::from(42i64));
    assert_ne!(Value::from(42i64), Value::from(43i64));
}

#[test]
fn float_equality() {
    let a: Value = 3.14.try_into().unwrap();
    let b: Value = 3.14.try_into().unwrap();
    let c: Value = 2.71.try_into().unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn string_equality() {
    assert_eq!(Value::from("hello"), Value::from("hello"));
    assert_ne!(Value::from("hello"), Value::from("world"));
}

#[test]
fn empty_string_equals_empty_string() {
    assert_eq!(Value::from(""), Value::from(""));
}

// -- Cross-type inequality --

#[test]
fn int_not_equal_to_float() {
    let i = Value::from(3i64);
    let f: Value = 3.0.try_into().unwrap();
    assert_ne!(i, f);
}

#[test]
fn null_not_equal_to_false() {
    assert_ne!(Value::Null, Value::from(false));
}

#[test]
fn null_not_equal_to_zero() {
    assert_ne!(Value::Null, Value::from(0i64));
}

#[test]
fn null_not_equal_to_empty_string() {
    assert_ne!(Value::Null, Value::from(""));
}

#[test]
fn bool_not_equal_to_int() {
    assert_ne!(Value::from(true), Value::from(1i64));
    assert_ne!(Value::from(false), Value::from(0i64));
}

// -- Array equality --

#[test]
fn empty_arrays_equal() {
    let a = Value::from(FrostArray::from(vec![]));
    let b = Value::from(FrostArray::from(vec![]));
    assert_eq!(a, b);
}

#[test]
fn arrays_same_elements() {
    let a = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]));
    let b = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]));
    assert_eq!(a, b);
}

#[test]
fn arrays_different_elements() {
    let a = Value::from(FrostArray::from(vec![Value::from(1i64)]));
    let b = Value::from(FrostArray::from(vec![Value::from(2i64)]));
    assert_ne!(a, b);
}

#[test]
fn arrays_different_lengths() {
    let a = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]));
    let b = Value::from(FrostArray::from(vec![Value::from(1i64)]));
    assert_ne!(a, b);
}

#[test]
fn nested_array_equality() {
    let inner = FrostArray::from(vec![Value::from(1i64)]);
    let a = Value::from(FrostArray::from(vec![Value::from(inner.clone())]));
    let b = Value::from(FrostArray::from(vec![Value::from(inner)]));
    assert_eq!(a, b);
}

// -- Map equality --

#[test]
fn empty_maps_equal() {
    assert_eq!(
        Value::from(FrostMap::empty()),
        Value::from(FrostMap::empty())
    );
}

#[test]
fn maps_same_entries() {
    let a: FrostMap = vec![(str_key("x"), Value::from(1i64))]
        .into_iter()
        .collect();
    let b: FrostMap = vec![(str_key("x"), Value::from(1i64))]
        .into_iter()
        .collect();
    assert_eq!(Value::from(a), Value::from(b));
}

#[test]
fn maps_different_values() {
    let a: FrostMap = vec![(str_key("x"), Value::from(1i64))]
        .into_iter()
        .collect();
    let b: FrostMap = vec![(str_key("x"), Value::from(2i64))]
        .into_iter()
        .collect();
    assert_ne!(Value::from(a), Value::from(b));
}

#[test]
fn maps_different_keys() {
    let a: FrostMap = vec![(str_key("x"), Value::from(1i64))]
        .into_iter()
        .collect();
    let b: FrostMap = vec![(str_key("y"), Value::from(1i64))]
        .into_iter()
        .collect();
    assert_ne!(Value::from(a), Value::from(b));
}

#[test]
fn maps_different_sizes() {
    let a: FrostMap = vec![
        (str_key("x"), Value::from(1i64)),
        (str_key("y"), Value::from(2i64)),
    ]
    .into_iter()
    .collect();
    let b: FrostMap = vec![(str_key("x"), Value::from(1i64))]
        .into_iter()
        .collect();
    assert_ne!(Value::from(a), Value::from(b));
}

// -- Arc short-circuit --

#[test]
fn same_arc_array_is_equal() {
    let arr = FrostArray::from(vec![Value::from(1i64)]);
    let a = Value::from(arr.clone());
    let b = Value::from(arr);
    assert_eq!(a, b);
}

#[test]
fn same_arc_map_is_equal() {
    let map: FrostMap = vec![(str_key("k"), Value::from(1i64))]
        .into_iter()
        .collect();
    let a = Value::from(map.clone());
    let b = Value::from(map);
    assert_eq!(a, b);
}

// -- Function / opaque identity (compared by Arc identity, never structurally) --

fn a_native() -> Arc<NativeFunction> {
    Arc::new(NativeFunction::new(
        "f",
        Arity::Exact(0),
        |_, _: &mut [Value]| Ok(Value::Null),
    ))
}

fn a_closure() -> Arc<Closure> {
    let f = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "f".to_string(),
        code: Vec::new(),
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    f.assert_trusted().into_closure().unwrap()
}

#[test]
fn native_functions_compare_by_identity() {
    let n = a_native();
    assert_eq!(Value::NativeFunction(n.clone()), Value::NativeFunction(n));
    assert_ne!(
        Value::NativeFunction(a_native()),
        Value::NativeFunction(a_native())
    );
}

#[test]
fn closures_compare_by_identity() {
    let c = a_closure();
    // The same closure equals itself.
    assert_eq!(Value::Closure(c.clone()), Value::Closure(c));
    // Distinct closures, even structurally identical, are never equal.
    assert_ne!(Value::Closure(a_closure()), Value::Closure(a_closure()));
}

#[test]
fn opaques_compare_by_identity() {
    let o: Arc<dyn FrostOpaque> = Arc::new(Marker(42));
    assert_eq!(Value::Opaque(o.clone()), Value::Opaque(o));
    // Distinct instances, even with equal payloads, are never equal.
    assert_ne!(Value::opaque(Marker(1)), Value::opaque(Marker(1)));
}

#[test]
fn a_function_never_equals_a_non_function() {
    assert_ne!(Value::Closure(a_closure()), Value::Null);
    assert_ne!(Value::NativeFunction(a_native()), Value::from(0i64));
}

// -- Negative zero --

#[test]
fn negative_zero_equals_positive_zero() {
    let pos: Value = 0.0.try_into().unwrap();
    let neg: Value = (-0.0f64).try_into().unwrap();
    assert_eq!(pos, neg);
}
