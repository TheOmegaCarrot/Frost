//! [`Uint64`]: an unsigned 64-bit integer beyond the Int range, shared by the
//! formats that have such integers.
//!
//! Scripts handle these with the functions of `ext.formats.u64`, installed
//! with any such format:
//!
//! - `from_string(text)`: the unsigned integer that `text`, decimal digits
//!   optionally after a `+`, spells, as [`to_value`] gives it; Null if `text`
//!   spells no unsigned 64-bit integer.
//! - `to_string(n)`: the decimal text of `n`, an unsigned integer as
//!   [`from_value`] takes it; anything else is an error.
//! - `to_float(n)`: the Float nearest `n`, which need not equal it; anything
//!   but an unsigned integer is an error.

mod natives;

pub(crate) use natives::module;

use std::fmt;

use frostlang::{FrostOpaque, Value};

/// An unsigned integer beyond the Int range, which Frost has no type for.
///
/// Every format with a native unsigned 64-bit integer, such as MessagePack,
/// decodes one beyond the Int range to this Opaque, which encodes back as the
/// same integer; within the Int range, it is an Int. The Opaque renders as its
/// decimal text, as in `<Uint64: 18446744073709551615>`, and is `==` to
/// another of the same value. [`Display`](fmt::Display) writes that text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Uint64(u64);

impl Uint64 {
    /// `n`, or `None` if it is within the Int range, where it is an Int.
    pub fn new(n: u64) -> Option<Self> {
        match i64::try_from(n) {
            Ok(_) => None,
            Err(_) => Some(Self(n)),
        }
    }

    /// The integer.
    pub fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for Uint64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FrostOpaque for Uint64 {
    fn type_name() -> &'static str {
        "Uint64"
    }

    fn try_to_string(&self) -> Option<String> {
        Some(self.to_string())
    }

    fn equals(&self, other: &dyn FrostOpaque) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }
}

/// The Frost value of the unsigned integer `n`: an Int if it fits one, else a
/// [`Uint64`] Opaque.
pub fn to_value(n: u64) -> Value {
    match i64::try_from(n) {
        Ok(n) => Value::Int(n),
        Err(_) => Value::opaque(Uint64(n)),
    }
}

/// The unsigned integer `value` holds, if it is an Int that is not negative,
/// or a [`Uint64`] Opaque.
pub fn from_value(value: &Value) -> Option<u64> {
    match value {
        Value::Int(i) => u64::try_from(*i).ok(),
        _ => value.downcast_opaque::<Uint64>().map(|n| n.get()),
    }
}
