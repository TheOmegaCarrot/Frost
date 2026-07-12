//! Tests for the structure opcodes: `MakeArray`, `MakeMap`, and `ExplodeArray`.
//!
//!   * `MakeArray(n)` takes the top `n` values; the topmost becomes the *back*.
//!   * `MakeMap(n)` takes the top `2n` values as `k1, v1, k2, v2, ...` (key deeper than its value).
//!     Keys must be non-null primitives, so it is fallible; duplicate keys keep the last value (right wins).
//!   * `ExplodeArray` is the inverse of `MakeArray`: it consumes one Array and pushes its elements (the back ends up on top).
//!     Its operand is compiler-guaranteed to be an Array (emitted only in compiled destructuring / pattern matching),
//!     so a non-array is an IMPOSSIBLE state -- it panics and is not tested here.
//!
//! Operands without a `Push*` opcode (String/Array/Map) come from the constant
//! table via `LoadConst`.

use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostArray, FrostError, MapKey, Value, Vm,
};

// ============================================================
// Helpers
// ============================================================

fn eval(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<structures>".to_string(),
        code: body,
        child_fns: Vec::new(),
        constants,
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

fn val(code: Vec<Bytecode>) -> Value {
    eval(vec![], code).unwrap()
}

fn array(vs: Vec<Value>) -> Value {
    Value::Array(FrostArray::from(vs))
}

fn skey(s: &str) -> MapKey {
    MapKey::String(Arc::from(s.as_bytes()))
}

fn map(pairs: Vec<(MapKey, Value)>) -> Value {
    Value::Map(pairs.into_iter().collect())
}

use Bytecode::{DropBelow, ExplodeArray, LoadConst, MakeArray, MakeMap, Pop, PushInt, PushNull};

// ============================================================
// MakeArray
// ============================================================

#[test]
fn make_array_empty() {
    assert_eq!(val(vec![MakeArray(0)]), array(vec![]));
}

#[test]
fn make_array_single() {
    assert_eq!(
        val(vec![PushInt(1), MakeArray(1)]),
        array(vec![Value::Int(1)])
    );
}

#[test]
fn make_array_preserves_push_order_top_is_back() {
    // 1, 2, 3 pushed; the topmost (3) is the back of the array.
    assert_eq!(
        val(vec![PushInt(1), PushInt(2), PushInt(3), MakeArray(3)]),
        array(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn make_array_holds_mixed_types() {
    let out = eval(
        vec![Value::from("x")],
        vec![PushInt(1), LoadConst(0), PushNull, MakeArray(3)],
    )
    .unwrap();
    assert_eq!(
        out,
        array(vec![Value::Int(1), Value::from("x"), Value::Null])
    );
}

#[test]
fn make_array_consumes_exactly_n() {
    // Sentinel below; MakeArray(2) takes only the top two, then Pop drops the
    // array, revealing the sentinel.
    assert_eq!(
        val(vec![PushInt(99), PushInt(1), PushInt(2), MakeArray(2), Pop]),
        Value::Int(99)
    );
}

#[test]
fn make_array_nests() {
    // [[1]] -- an array whose only element is itself an array.
    assert_eq!(
        val(vec![PushInt(1), MakeArray(1), MakeArray(1)]),
        array(vec![array(vec![Value::Int(1)])])
    );
}

// ============================================================
// MakeMap
// ============================================================

#[test]
fn make_map_empty() {
    assert_eq!(val(vec![MakeMap(0)]), map(vec![]));
}

#[test]
fn make_map_single_pair() {
    // {a: 1} -- key is deeper, value on top.
    let out = eval(
        vec![Value::from("a")],
        vec![LoadConst(0), PushInt(1), MakeMap(1)],
    )
    .unwrap();
    assert_eq!(out, map(vec![(skey("a"), Value::Int(1))]));
}

#[test]
fn make_map_does_not_swap_key_and_value() {
    // Distinguishes {a: 1} from {1: "a"} -- proves key (deeper) and value (top)
    // are not transposed.
    let out = eval(
        vec![Value::from("a")],
        vec![LoadConst(0), PushInt(1), MakeMap(1)],
    )
    .unwrap();
    let m = out.as_map().unwrap();
    assert_eq!(m.get_str("a"), Some(&Value::Int(1)));
    assert!(m.get(&MapKey::Int(1)).is_none());
}

#[test]
fn make_map_multiple_pairs() {
    let out = eval(
        vec![Value::from("a"), Value::from("b")],
        vec![
            LoadConst(0),
            PushInt(1),
            LoadConst(1),
            PushInt(2),
            MakeMap(2),
        ],
    )
    .unwrap();
    assert_eq!(
        out,
        map(vec![(skey("a"), Value::Int(1)), (skey("b"), Value::Int(2))])
    );
}

#[test]
fn make_map_duplicate_key_keeps_last() {
    // {a: 1, a: 2} -> {a: 2} (right/last wins).
    let out = eval(
        vec![Value::from("a")],
        vec![
            LoadConst(0),
            PushInt(1),
            LoadConst(0),
            PushInt(2),
            MakeMap(2),
        ],
    )
    .unwrap();
    assert_eq!(out, map(vec![(skey("a"), Value::Int(2))]));
}

#[test]
fn make_map_accepts_int_key() {
    // {1: "one"} -- Int is a valid (primitive) key.
    let out = eval(
        vec![Value::from("one")],
        vec![PushInt(1), LoadConst(0), MakeMap(1)],
    )
    .unwrap();
    assert_eq!(out, map(vec![(MapKey::Int(1), Value::from("one"))]));
}

#[test]
fn make_map_value_may_be_null() {
    // Only keys are restricted; a null *value* is fine: {a: null}.
    let out = eval(
        vec![Value::from("a")],
        vec![LoadConst(0), PushNull, MakeMap(1)],
    )
    .unwrap();
    assert_eq!(out, map(vec![(skey("a"), Value::Null)]));
}

#[test]
fn make_map_null_key_is_error() {
    let err = eval(vec![], vec![PushNull, PushInt(1), MakeMap(1)]).unwrap_err();
    assert!(err.message.contains("Map key"), "got: {}", err.message);
}

#[test]
fn make_map_structured_key_is_error() {
    // An array is not a primitive, so it cannot be a key.
    let err = eval(
        vec![Value::Array(FrostArray::empty())],
        vec![LoadConst(0), PushInt(1), MakeMap(1)],
    )
    .unwrap_err();
    assert!(err.message.contains("Map key"), "got: {}", err.message);
}

#[test]
fn make_map_consumes_exactly_two_per_pair() {
    // Sentinel below a single pair; Pop drops the map, revealing the sentinel.
    let out = eval(
        vec![Value::from("a")],
        vec![PushInt(99), LoadConst(0), PushInt(1), MakeMap(1), Pop],
    )
    .unwrap();
    assert_eq!(out, Value::Int(99));
}

// ============================================================
// ExplodeArray
// ============================================================

#[test]
fn explode_empty_pushes_nothing() {
    // Sentinel below an empty array: exploding consumes the array and pushes
    // nothing, so the sentinel remains the tail.
    let out = eval(
        vec![array(vec![])],
        vec![PushInt(99), LoadConst(0), ExplodeArray],
    )
    .unwrap();
    assert_eq!(out, Value::Int(99));
}

#[test]
fn explode_single_element() {
    let out = eval(
        vec![array(vec![Value::Int(7)])],
        vec![LoadConst(0), ExplodeArray],
    )
    .unwrap();
    assert_eq!(out, Value::Int(7));
}

#[test]
fn explode_puts_array_back_on_top() {
    // [1, 2, 3] explodes so the back element (3) lands on top; drop the two below it
    // to isolate the top and prove it is 3.
    let out = eval(
        vec![array(vec![Value::Int(1), Value::Int(2), Value::Int(3)])],
        vec![LoadConst(0), ExplodeArray, DropBelow(1), DropBelow(1)],
    )
    .unwrap();
    assert_eq!(out, Value::Int(3));
}

#[test]
fn explode_then_make_array_round_trips() {
    // ExplodeArray followed by MakeArray(n) is the identity -- proves element
    // order is preserved. (Clone path: the constant table still references it.)
    let original = array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    let out = eval(
        vec![original.clone()],
        vec![LoadConst(0), ExplodeArray, MakeArray(3)],
    )
    .unwrap();
    assert_eq!(out, original);
}

#[test]
fn explode_consumes_array_and_pushes_each_element() {
    // Sentinel below; explode a 2-element array, re-collect exactly 2, then Pop
    // the rebuilt array -- the sentinel proves explode pushed exactly two values.
    let out = eval(
        vec![array(vec![Value::Int(1), Value::Int(2)])],
        vec![PushInt(99), LoadConst(0), ExplodeArray, MakeArray(2), Pop],
    )
    .unwrap();
    assert_eq!(out, Value::Int(99));
}

#[test]
fn explode_uniquely_owned_array_round_trips() {
    // Build the array with MakeArray (uniquely owned -> try_extract moves), then
    // explode and rebuild. Exercises the zero-copy steal path.
    assert_eq!(
        val(vec![
            PushInt(1),
            PushInt(2),
            MakeArray(2),
            ExplodeArray,
            MakeArray(2)
        ]),
        array(vec![Value::Int(1), Value::Int(2)])
    );
}

#[test]
fn explode_mixed_types_round_trips() {
    let original = array(vec![Value::Int(1), Value::from("x"), Value::Null]);
    let out = eval(
        vec![original.clone()],
        vec![LoadConst(0), ExplodeArray, MakeArray(3)],
    )
    .unwrap();
    assert_eq!(out, original);
}

#[test]
fn explode_is_shallow() {
    // Exploding [[1], [2]] pushes the two inner arrays as single values, not their
    // contents -- re-collecting yields the original nested structure.
    let original = array(vec![array(vec![Value::Int(1)]), array(vec![Value::Int(2)])]);
    let out = eval(
        vec![original.clone()],
        vec![LoadConst(0), ExplodeArray, MakeArray(2)],
    )
    .unwrap();
    assert_eq!(out, original);
}
