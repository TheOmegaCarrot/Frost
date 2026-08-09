//! Type checking, conversion, and value serialization.

use enumset::{EnumSet, enum_set};

use crate::{Arity, FrostType, Param, Params, Value};

/// The shared one-any-argument spec (`Exact(1)`).
const ONE_ANY: Params = Params::new(&[Param::any()]);

/// The spec for the numeric parsers: the types a number can be read from.
/// A value of any other type is a type error, which is distinct from a value of the
/// right type whose content will not convert: that returns Null.
const NUMERIC_OR_STRING: EnumSet<FrostType> =
    enum_set!(FrostType::Int | FrostType::Float | FrostType::String);
const ONE_NUMERIC_OR_STRING: Params = Params::new(&[Param::of(NUMERIC_OR_STRING)]);

/// A one-argument predicate native accepting any value.
fn predicate(name: &'static str, pred: fn(&Value) -> bool) -> Value {
    Value::checked_native(name, ONE_ANY, move |_, args| Ok(pred(&args[0]).into()))
}

pub(super) fn is_null_global() -> Value {
    predicate("is_null", Value::is_null)
}

pub(super) fn is_int_global() -> Value {
    predicate("is_int", Value::is_int)
}

pub(super) fn is_float_global() -> Value {
    predicate("is_float", Value::is_float)
}

pub(super) fn is_bool_global() -> Value {
    predicate("is_bool", Value::is_bool)
}

pub(super) fn is_string_global() -> Value {
    predicate("is_string", Value::is_string)
}

pub(super) fn is_bytes_global() -> Value {
    predicate("is_bytes", Value::is_bytes)
}

pub(super) fn is_array_global() -> Value {
    predicate("is_array", Value::is_array)
}

pub(super) fn is_map_global() -> Value {
    predicate("is_map", Value::is_map)
}

pub(super) fn is_function_global() -> Value {
    predicate("is_function", Value::is_function)
}

pub(super) fn is_nonnull_global() -> Value {
    predicate("is_nonnull", Value::is_nonnull)
}

pub(super) fn is_numeric_global() -> Value {
    predicate("is_numeric", Value::is_numeric)
}

pub(super) fn is_primitive_global() -> Value {
    predicate("is_primitive", Value::is_primitive)
}

pub(super) fn is_structured_global() -> Value {
    predicate("is_structured", Value::is_structured)
}

pub(super) fn is_flat_global() -> Value {
    predicate("is_flat", Value::is_flat)
}

pub(super) fn type_global() -> Value {
    Value::checked_native("type", ONE_ANY, |_, args| {
        Ok(Value::from(args[0].type_name()))
    })
}

pub(super) fn to_string_global() -> Value {
    Value::native("to_string", Arity::Exact(1), |_, args| {
        Ok(Value::from(args[0].to_frost_string()))
    })
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
