//! Tests for the binary arithmetic opcodes: `Add`, `Subtract`, `Multiply`, `Divide`, `Modulus`.
//!
//! These opcodes delegate to the corresponding `Value` operators, whose full semantics
//! (numeric promotion, overflow wrapping, string/array/map overloads, every type error) are covered at the value level in `binary_operators.rs`.
//! What only the VM can get wrong lives here:
//!   * each opcode delegates to the *right* operator (a representative case),
//!   * operand order -- rhs is the top of the stack, lhs below (decisive for the non-commutative `-`, `/`, `%`),
//!   * stack effect -- two operands consumed, one result pushed,
//!   * errors `?`-propagate out through the opcode.
//!
//! Operands without a `Push*` opcode (String/Array/Map) come from the constant
//! table via `LoadConst`.

use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FrostArray, FrostError, FrostFloat, MapKey, Value, Vm,
};

// ============================================================
// Helpers
// ============================================================

/// Run a constants-carrying, local-less top-level program; return its tail value
/// (or the error it raised).
fn eval(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let mut body = vec![Bytecode::Pop]; // pop the closure value the runner pushes
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        name: "<arith>".to_string(),
        code: body,
        child_fns: Vec::new(),
        constants,
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let closure = program.into_closure().unwrap();
    Vm::new(closure).unwrap().run().map(|r| r.tail().clone())
}

/// Evaluate a constant-free program, expecting success.
fn val(code: Vec<Bytecode>) -> Value {
    eval(vec![], code).unwrap()
}

fn float(x: f64) -> Bytecode {
    Bytecode::PushFloat(FrostFloat::new(x).unwrap())
}

fn float_val(x: f64) -> Value {
    Value::try_from(x).unwrap()
}

fn arr(xs: &[i64]) -> Value {
    Value::Array(FrostArray::from(
        xs.iter().copied().map(Value::Int).collect::<Vec<_>>(),
    ))
}

fn map_kv(pairs: &[(&str, i64)]) -> Value {
    Value::Map(
        pairs
            .iter()
            .map(|(k, v)| (MapKey::String(Arc::from(k.as_bytes())), Value::Int(*v)))
            .collect(),
    )
}

use Bytecode::{
    Add, Divide, LoadConst, MakeArray, Modulus, Multiply, Pop, PushInt, PushNull, Subtract,
};

// ============================================================
// Add (numeric, plus string/array/map overloads)
// ============================================================

#[test]
fn add_ints() {
    assert_eq!(val(vec![PushInt(2), PushInt(3), Add]), Value::Int(5));
}

#[test]
fn add_promotes_int_and_float() {
    // Delegates to the promoting `Value::add`: 1 + 2.5 -> 3.5.
    assert_eq!(val(vec![PushInt(1), float(2.5), Add]), float_val(3.5));
}

#[test]
fn add_concatenates_strings_in_operand_order() {
    // 'ab' + 'cd' -> 'abcd' (not 'cdab'): proves Add reaches string concat and
    // that the deeper operand is the lhs.
    let out = eval(
        vec![Value::from("ab"), Value::from("cd")],
        vec![LoadConst(0), LoadConst(1), Add],
    )
    .unwrap();
    assert_eq!(out, Value::from("abcd"));
}

#[test]
fn add_concatenates_arrays() {
    // [1, 2] + [3] -> [1, 2, 3].
    let out = eval(
        vec![arr(&[1, 2]), arr(&[3])],
        vec![LoadConst(0), LoadConst(1), Add],
    )
    .unwrap();
    assert_eq!(out, arr(&[1, 2, 3]));
}

#[test]
fn add_merges_maps() {
    // {a: 1} + {b: 2} -> {a: 1, b: 2}.
    let out = eval(
        vec![map_kv(&[("a", 1)]), map_kv(&[("b", 2)])],
        vec![LoadConst(0), LoadConst(1), Add],
    )
    .unwrap();
    assert_eq!(out, map_kv(&[("a", 1), ("b", 2)]));
}

#[test]
fn add_map_overlapping_keys_right_wins() {
    // {a: 1, b: 2} + {b: 99, c: 3} -> {a: 1, b: 99, c: 3}: the rhs value wins on a
    // key collision. (The Add opcode now merges in its own steal path, so pin it.)
    let out = eval(
        vec![
            map_kv(&[("a", 1), ("b", 2)]),
            map_kv(&[("b", 99), ("c", 3)]),
        ],
        vec![LoadConst(0), LoadConst(1), Add],
    )
    .unwrap();
    assert_eq!(out, map_kv(&[("a", 1), ("b", 99), ("c", 3)]));
}

#[test]
fn add_arrays_with_empty_operands() {
    // Empty operands concat correctly on either side.
    let lhs_empty = eval(
        vec![arr(&[]), arr(&[1, 2])],
        vec![LoadConst(0), LoadConst(1), Add],
    )
    .unwrap();
    assert_eq!(lhs_empty, arr(&[1, 2]));
    let rhs_empty = eval(
        vec![arr(&[1, 2]), arr(&[])],
        vec![LoadConst(0), LoadConst(1), Add],
    )
    .unwrap();
    assert_eq!(rhs_empty, arr(&[1, 2]));
}

#[test]
fn add_uniquely_owned_arrays_concat_via_steal_path() {
    // Operands built fresh with MakeArray are uniquely owned, so Add takes the
    // steal/move branch; the result must still be the plain concatenation.
    assert_eq!(
        val(vec![
            PushInt(1),
            MakeArray(1),
            PushInt(2),
            MakeArray(1),
            Add
        ]),
        arr(&[1, 2])
    );
}

#[test]
fn add_incompatible_types_is_error() {
    let err = eval(vec![], vec![PushInt(1), PushNull, Add]).unwrap_err();
    assert!(
        err.message.contains("incompatible types"),
        "got: {}",
        err.message
    );
}

// ============================================================
// Subtract (numeric only; non-commutative -> operand order matters)
// ============================================================

#[test]
fn subtract_respects_operand_order() {
    // 10 - 3 -> 7. A reversed pop would give 3 - 10 = -7.
    assert_eq!(val(vec![PushInt(10), PushInt(3), Subtract]), Value::Int(7));
}

#[test]
fn subtract_floats() {
    assert_eq!(val(vec![float(5.0), float(1.5), Subtract]), float_val(3.5));
}

#[test]
fn subtract_incompatible_types_is_error() {
    let err = eval(vec![], vec![PushInt(1), PushNull, Subtract]).unwrap_err();
    assert!(
        err.message.contains("incompatible types"),
        "got: {}",
        err.message
    );
}

// ============================================================
// Multiply (numeric only)
// ============================================================

#[test]
fn multiply_ints() {
    assert_eq!(val(vec![PushInt(4), PushInt(3), Multiply]), Value::Int(12));
}

#[test]
fn multiply_promotes_int_and_float() {
    assert_eq!(val(vec![PushInt(2), float(2.5), Multiply]), float_val(5.0));
}

#[test]
fn multiply_incompatible_types_is_error() {
    let err = eval(vec![], vec![PushInt(2), PushNull, Multiply]).unwrap_err();
    assert!(
        err.message.contains("incompatible types"),
        "got: {}",
        err.message
    );
}

// ============================================================
// Divide (non-commutative; integer division truncates toward zero)
// ============================================================

#[test]
fn divide_respects_operand_order_and_truncates() {
    // 7 / 2 -> 3 (truncates). Reversed would be 2 / 7 = 0.
    assert_eq!(val(vec![PushInt(7), PushInt(2), Divide]), Value::Int(3));
}

#[test]
fn divide_truncates_toward_zero_for_negatives() {
    // -7 / 2 -> -3 (toward zero, not floor -4).
    assert_eq!(val(vec![PushInt(-7), PushInt(2), Divide]), Value::Int(-3));
}

#[test]
fn divide_floats() {
    assert_eq!(val(vec![float(7.0), float(2.0), Divide]), float_val(3.5));
}

#[test]
fn divide_by_zero_is_error() {
    let err = eval(vec![], vec![PushInt(1), PushInt(0), Divide]).unwrap_err();
    assert_eq!(err.message, "Division by zero");
}

// ============================================================
// Modulus (Int-only; sign follows the dividend)
// ============================================================

#[test]
fn modulus_respects_operand_order() {
    // 7 % 3 -> 1. Reversed would be 3 % 7 = 3.
    assert_eq!(val(vec![PushInt(7), PushInt(3), Modulus]), Value::Int(1));
}

#[test]
fn modulus_sign_follows_dividend() {
    // -7 % 3 -> -1 (the result takes the dividend's sign).
    assert_eq!(val(vec![PushInt(-7), PushInt(3), Modulus]), Value::Int(-1));
}

#[test]
fn modulus_by_zero_is_error() {
    let err = eval(vec![], vec![PushInt(1), PushInt(0), Modulus]).unwrap_err();
    assert_eq!(err.message, "Modulus by zero");
}

#[test]
fn modulus_on_floats_is_type_error() {
    // Modulus is Int-only; floats are a type error, not a computation.
    let err = eval(vec![], vec![float(7.0), float(3.0), Modulus]).unwrap_err();
    assert!(
        err.message.contains("incompatible types"),
        "got: {}",
        err.message
    );
}

// ============================================================
// Stack effect
// ============================================================

#[test]
fn arithmetic_leaves_exactly_one_value() {
    // Sentinel below; after the op and a Pop, the sentinel is the tail -- proving
    // the opcode consumed two operands and pushed exactly one result.
    assert_eq!(
        val(vec![PushInt(99), PushInt(2), PushInt(3), Add, Pop]),
        Value::Int(99)
    );
}
