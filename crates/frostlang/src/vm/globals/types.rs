//! Type checking, conversion, and value serialization.

use std::num::NonZeroU64;

use crate::bytecode::Bytecode;
use crate::native::Flat;
use crate::{Arity, FrostBytes, FrostType, TypeSet, Value, frost_fn};

/// A one-argument type predicate `is_X(v)`, testing whether v's type is in `set`.
/// Compiles to `DropBelow(1); TypeTest(set)`, identical to `Value::fits(set)`.
fn type_predicate(name: &'static str, set: TypeSet) -> Value {
    super::bytecode_global(
        name,
        Arity::Exact(1),
        vec![Bytecode::DropBelow(1), Bytecode::TypeTest(set)],
    )
}

pub(super) fn is_null_global() -> Value {
    type_predicate("is_null", FrostType::NULL)
}

pub(super) fn is_int_global() -> Value {
    type_predicate("is_int", FrostType::INT)
}

pub(super) fn is_float_global() -> Value {
    type_predicate("is_float", FrostType::FLOAT)
}

pub(super) fn is_bool_global() -> Value {
    type_predicate("is_bool", FrostType::BOOL)
}

pub(super) fn is_string_global() -> Value {
    type_predicate("is_string", FrostType::STRING)
}

pub(super) fn is_bytes_global() -> Value {
    type_predicate("is_bytes", FrostType::BYTES)
}

pub(super) fn is_array_global() -> Value {
    type_predicate("is_array", FrostType::ARRAY)
}

pub(super) fn is_map_global() -> Value {
    type_predicate("is_map", FrostType::MAP)
}

pub(super) fn is_function_global() -> Value {
    type_predicate("is_function", FrostType::FUNCTION)
}

pub(super) fn is_opaque_global() -> Value {
    type_predicate("is_opaque", FrostType::OPAQUE)
}

pub(super) fn is_nonnull_global() -> Value {
    type_predicate("is_nonnull", FrostType::NONNULL)
}

pub(super) fn is_numeric_global() -> Value {
    type_predicate("is_numeric", FrostType::NUMERIC)
}

pub(super) fn is_primitive_global() -> Value {
    type_predicate("is_primitive", FrostType::PRIMITIVE)
}

pub(super) fn is_structured_global() -> Value {
    type_predicate("is_structured", FrostType::STRUCTURED)
}

pub(super) fn is_flat_global() -> Value {
    type_predicate("is_flat", FrostType::FLAT)
}

#[frost_fn]
pub(super) fn r#type(#[frost(unnamed)] value: Value) -> &'static str {
    value.type_name()
}

pub(super) fn to_string_global() -> Value {
    // Concat(1) stringifies its single operand via `to_frost_string`: identical.
    super::bytecode_global(
        "to_string",
        Arity::Exact(1),
        vec![
            Bytecode::DropBelow(1),
            Bytecode::Concat(NonZeroU64::new(1).unwrap()),
        ],
    )
}

#[frost_fn]
pub(super) fn pretty(#[frost(unnamed)] value: Value) -> String {
    value.to_pretty_string()
}

// The numeric parsers take the types a number can be read from. A value of any
// other type is a type error, which is distinct from a value of the right type
// whose content will not convert: that returns Null.

#[frost_fn]
pub(super) fn to_int(#[frost(unnamed, types = Numeric | String)] value: Value) -> Value {
    value.to_frost_int()
}

#[frost_fn]
pub(super) fn to_float(#[frost(unnamed, types = Numeric | String)] value: Value) -> Value {
    value.to_frost_float()
}

#[frost_fn]
pub(super) fn to_bytes(#[frost(unnamed)] value: Flat) -> Value {
    Value::from(value).to_frost_bytes()
}

#[frost_fn]
pub(super) fn from_utf8(#[frost(unnamed)] bytes: FrostBytes) -> Value {
    Value::Bytes(bytes).decode_utf8()
}
