//! Tests for `Value::frost_type`, `FrostType::name`, and `Value::fits_category`:
//! the type-classification core that `type_name` and the `is_*` predicates delegate to.
//!
//! The `value_type_checks` suite covers the `is_*` predicates over the common variants;
//! this file covers what it doesn't: the `frost_type` / `name` / `fits_category` surface directly,
//! the `Function` (native *and* closure) and `Opaque` variants, and every `FrostTypeCategory`,
//! with concrete hand-authored expectations (not re-derived from the implementation), so an inverted category would be caught.

mod common;

use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, FrostArray, FrostMap, FrostType, FrostTypeCategory, NativeFunction, Value,
};

type Ft = FrostType;
type Ftc = FrostTypeCategory;

// ============================================================
// One value of each runtime variant
// ============================================================

fn float(x: f64) -> Value {
    Value::try_from(x).unwrap()
}

fn array() -> Value {
    Value::Array(FrostArray::empty())
}

fn map() -> Value {
    Value::Map(FrostMap::empty())
}

fn native() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction::new(
        "f",
        Arity::Exact(0),
        |_ctx, _args| Ok(Value::Null),
    )))
}

/// A real `Closure` value, only obtainable by running `CreateClosure`.
fn closure() -> Value {
    let child = common::func(
        vec![Bytecode::Pop, Bytecode::PushInt(1)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let program = common::func(
        vec![Bytecode::CreateClosure {
            num_captures: 0,
            function: 0,
        }],
        Arity::Exact(0),
        vec![],
        vec![child],
    );
    common::run_fn(program).tail().clone()
}

fn opaque() -> Value {
    let data: Arc<dyn std::any::Any + Send + Sync> = Arc::new(0u32);
    Value::Opaque(data)
}

/// Every runtime variant paired with its expected `FrostType`. Note both function
/// variants map to `Function`.
fn samples() -> Vec<(Value, FrostType)> {
    vec![
        (Value::Null, Ft::Null),
        (Value::Bool(true), Ft::Bool),
        (Value::Int(1), Ft::Int),
        (float(1.5), Ft::Float),
        (Value::from("x"), Ft::String),
        (array(), Ft::Array),
        (map(), Ft::Map),
        (native(), Ft::Function),
        (closure(), Ft::Function),
        (opaque(), Ft::Opaque),
    ]
}

// ============================================================
// frost_type
// ============================================================

#[test]
fn frost_type_classifies_every_variant() {
    for (v, expected) in samples() {
        assert_eq!(v.frost_type(), expected, "{}", v.type_name());
    }
}

#[test]
fn both_function_variants_are_function() {
    // A closure is a Function, not just a native function.
    assert_eq!(native().frost_type(), Ft::Function);
    assert_eq!(closure().frost_type(), Ft::Function);
    assert!(native().is_function());
    assert!(closure().is_function());
}

// ============================================================
// FrostType::name (and type_name delegation)
// ============================================================

#[test]
fn frost_type_name_strings() {
    assert_eq!(Ft::Null.name(), "Null");
    assert_eq!(Ft::Bool.name(), "Bool");
    assert_eq!(Ft::Int.name(), "Int");
    assert_eq!(Ft::Float.name(), "Float");
    assert_eq!(Ft::String.name(), "String");
    assert_eq!(Ft::Array.name(), "Array");
    assert_eq!(Ft::Map.name(), "Map");
    assert_eq!(Ft::Function.name(), "Function");
    assert_eq!(Ft::Opaque.name(), "Opaque");
}

#[test]
fn type_name_delegates_to_frost_type_name() {
    for (v, _) in samples() {
        assert_eq!(v.type_name(), v.frost_type().name());
    }
    // Both function variants stringify as "Function".
    assert_eq!(native().type_name(), "Function");
    assert_eq!(closure().type_name(), "Function");
}

// ============================================================
// fits_category: Exact
// ============================================================

#[test]
fn exact_matches_own_type() {
    for (v, t) in samples() {
        assert!(
            v.fits_category(Ftc::Exact(t)),
            "{} is Exact({:?})",
            v.type_name(),
            t
        );
    }
}

#[test]
fn exact_rejects_foreign_type() {
    assert!(!Value::Int(1).fits_category(Ftc::Exact(Ft::String)));
    assert!(!Value::from("x").fits_category(Ftc::Exact(Ft::Int)));
    assert!(!Value::Null.fits_category(Ftc::Exact(Ft::Bool)));
    assert!(!array().fits_category(Ftc::Exact(Ft::Map)));
    assert!(!native().fits_category(Ftc::Exact(Ft::Opaque)));
}

// ============================================================
// fits_category: categories (concrete expectations)
// ============================================================

/// Assert each variant's membership in `category` against a hand-written truth.
fn assert_membership(category: Ftc, expected_true: &[Ft]) {
    for (v, t) in samples() {
        let expected = expected_true.contains(&t);
        assert_eq!(
            v.fits_category(category),
            expected,
            "{} in {:?}",
            v.type_name(),
            category
        );
    }
}

#[test]
fn numeric_category() {
    // Int and Float only.
    assert_membership(Ftc::Numeric, &[Ft::Int, Ft::Float]);
}

#[test]
fn primitive_category() {
    // Null counts as primitive; Array/Map/Function/Opaque do not.
    assert_membership(
        Ftc::Primitive,
        &[Ft::Null, Ft::Bool, Ft::Int, Ft::Float, Ft::String],
    );
}

#[test]
fn structured_category() {
    // Array and Map only.
    assert_membership(Ftc::Structured, &[Ft::Array, Ft::Map]);
}

#[test]
fn nonnull_category() {
    // Everything except Null.
    assert_membership(
        Ftc::NonNull,
        &[
            Ft::Bool,
            Ft::Int,
            Ft::Float,
            Ft::String,
            Ft::Array,
            Ft::Map,
            Ft::Function,
            Ft::Opaque,
        ],
    );
}

#[test]
fn null_is_primitive_but_not_nonnull() {
    // The two subtle cases worth stating outright.
    assert!(Value::Null.fits_category(Ftc::Primitive));
    assert!(!Value::Null.fits_category(Ftc::NonNull));
}
