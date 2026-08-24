//! Type checking, conversion, and value serialization.

use std::num::NonZeroUsize;

use enumset::{EnumSet, enum_set};

use crate::{Arity, Bytecode, FrostType, Param, Params, Value};

/// The shared one-any-argument spec (`Exact(1)`).
const ONE_ANY: Params = Params::new(&[Param::any()]);

/// The spec for the numeric parsers: the types a number can be read from.
/// A value of any other type is a type error, which is distinct from a value of the
/// right type whose content will not convert: that returns Null.
const NUMERIC_OR_STRING: EnumSet<FrostType> =
    enum_set!(FrostType::Int | FrostType::Float | FrostType::String);
const ONE_NUMERIC_OR_STRING: Params = Params::new(&[Param::of(NUMERIC_OR_STRING)]);

/// A one-argument type predicate `is_X(v)`, testing whether v's type is in `set`.
/// Compiles to `DropBelow(1); TypeTest(set)`, identical to `Value::fits(set)`.
fn type_predicate(name: &'static str, set: EnumSet<FrostType>) -> Value {
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

pub(super) fn type_global() -> Value {
    Value::checked_native("type", ONE_ANY, |_, args| {
        Ok(Value::from(args[0].type_name()))
    })
}

pub(super) fn to_string_global() -> Value {
    // Concat(1) stringifies its single operand via `to_frost_string`: identical.
    super::bytecode_global(
        "to_string",
        Arity::Exact(1),
        vec![
            Bytecode::DropBelow(1),
            Bytecode::Concat(NonZeroUsize::new(1).unwrap()),
        ],
    )
}

pub(super) fn pretty_global() -> Value {
    Value::native("pretty", Arity::Exact(1), |_, args| {
        Ok(Value::from(args[0].to_pretty_string()))
    })
}

pub(super) fn to_int_global() -> Value {
    Value::checked_native("to_int", ONE_NUMERIC_OR_STRING, |_, args| {
        Ok(args[0].to_frost_int())
    })
}

pub(super) fn to_float_global() -> Value {
    Value::checked_native("to_float", ONE_NUMERIC_OR_STRING, |_, args| {
        Ok(args[0].to_frost_float())
    })
}

pub(super) fn to_bytes_global() -> Value {
    super::stub("to_bytes")
}

pub(super) fn from_utf8_global() -> Value {
    super::stub("from_utf8")
}
