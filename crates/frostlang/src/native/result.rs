//! Returning Rust types from a native as values.

use serde::Serialize;

use crate::{
    FrostArray, FrostBytes, FrostError, FrostFloat, FrostMap, FrostResult, FrostString, Value,
    to_value,
};

/// A Rust type a native can return, converted to the value the native
/// returns.
///
/// A `Vec` returns an Array, as it is taken from one by
/// [`FromArg`](super::FromArg). So a `Vec<u8>` returns an Array of Ints,
/// unlike `Value::from`, which makes Bytes of it: return Bytes as
/// [`FrostBytes`], or return a [`Value`] to choose the value exactly.
pub trait IntoNativeResult {
    /// This result as the value returned by the native named `function`, or
    /// the error it raises. The conversion fails only for a result no value
    /// can hold, such as a non-finite `f64`.
    fn into_result(self, function: &str) -> FrostResult;
}

impl<T: IntoNativeResult> IntoNativeResult for Result<T, FrostError> {
    fn into_result(self, function: &str) -> FrostResult {
        self?.into_result(function)
    }
}

/// `None` returns Null.
impl<T: IntoNativeResult> IntoNativeResult for Option<T> {
    fn into_result(self, function: &str) -> FrostResult {
        self.map_or(Ok(Value::Null), |result| result.into_result(function))
    }
}

/// Returns Null.
impl IntoNativeResult for () {
    fn into_result(self, _: &str) -> FrostResult {
        Ok(Value::Null)
    }
}

/// Implements [`IntoNativeResult`] for each type that converts into a [`Value`]
/// as is.
macro_rules! into_value {
    ($($type:ty),*) => {$(
        impl IntoNativeResult for $type {
            fn into_result(self, _: &str) -> FrostResult {
                Ok(Value::from(self))
            }
        }
    )*};
}

into_value!(
    Value,
    bool,
    FrostFloat,
    String,
    &str,
    FrostString,
    FrostBytes,
    FrostArray,
    FrostMap
);

/// A non-finite `f64` is an error: a Float is always finite.
impl IntoNativeResult for f64 {
    fn into_result(self, function: &str) -> FrostResult {
        FrostFloat::new(self).map(Value::Float).map_err(|_| {
            FrostError::from_string(format!("Function {function} has no finite result"))
        })
    }
}

/// Implements [`IntoNativeResult`] for each integer type. A result outside the
/// Int range is an error.
macro_rules! into_int {
    ($($int:ty),*) => {$(
        impl IntoNativeResult for $int {
            fn into_result(self, function: &str) -> FrostResult {
                i64::try_from(self).map(Value::Int).map_err(|_| {
                    FrostError::from_string(format!(
                        "Function {function} has no Int result: {self} is out of range"
                    ))
                })
            }
        }
    )*};
}

into_int!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

/// An Array of each element's result.
impl<T: IntoNativeResult> IntoNativeResult for Vec<T> {
    fn into_result(self, function: &str) -> FrostResult {
        self.into_iter()
            .map(|element| element.into_result(function))
            .collect::<Result<Vec<Value>, FrostError>>()
            .map(Value::from)
    }
}

/// A result serialized into a value with [`to_value`], such as a struct into
/// a Map.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Ser<T>(pub T);

impl<T: Serialize> IntoNativeResult for Ser<T> {
    fn into_result(self, function: &str) -> FrostResult {
        to_value(&self.0).map_err(|err| {
            FrostError::from_string(format!(
                "Function {function} has a result no value can hold: {}",
                err.message()
            ))
        })
    }
}
