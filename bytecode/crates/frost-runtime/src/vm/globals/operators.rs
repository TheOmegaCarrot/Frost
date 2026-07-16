//! Arithmetic and comparison operators as first-class functions.
//!
//! Operators sidestep the `Param` type-check (`Value::native`, not `checked_native`):
//! their validity is a *relation* between the two args (`Int + Int` is fine but
//! `Int + String` is not, and `String + String` / `Array + Array` are also fine),
//! which a per-parameter spec can't express. The underlying `Value` method raises
//! the type error instead.

use std::cmp::Ordering;

use crate::core::FrostResult;
use crate::{Arity, Value};

/// A two-argument native that forwards to an infix `Value` operator method.
fn binary(name: &'static str, op: fn(&Value, &Value) -> FrostResult) -> Value {
    Value::native(name, Arity::Exact(2), move |_, args| op(&args[0], &args[1]))
}

/// A two-argument comparison native: orders the args and maps the `Ordering` to a Bool.
fn comparison(name: &'static str, accept: fn(Ordering) -> bool) -> Value {
    Value::native(name, Arity::Exact(2), move |_, args| {
        Ok(accept(args[0].compare(&args[1])?).into())
    })
}

pub(super) fn plus_global() -> Value {
    // `+` concatenates Arrays and merges Maps. Consume the args so those cases
    // steal their backing storage instead of cloning, mirroring `Vm::do_add`;
    // every other case forwards to the scalar `Value::add`.
    Value::native("plus", Arity::Exact(2), |_, args| {
        let lhs = args[0].take();
        let rhs = args[1].take();
        Ok(match (lhs, rhs) {
            (Value::Array(l), Value::Array(r)) => {
                let mut elems = l.to_owned();
                elems.extend(r.to_owned());
                Value::Array(elems.into())
            }
            (Value::Map(l), Value::Map(r)) => {
                let mut entries = l.to_owned();
                entries.extend(r.to_owned());
                Value::Map(entries.into())
            }
            (lhs, rhs) => lhs.add(&rhs)?,
        })
    })
}

pub(super) fn minus_global() -> Value {
    binary("minus", Value::subtract)
}

pub(super) fn times_global() -> Value {
    binary("times", Value::multiply)
}

pub(super) fn divide_global() -> Value {
    binary("divide", Value::divide)
}

pub(super) fn mod_global() -> Value {
    binary("mod", Value::modulus)
}

pub(super) fn equal_global() -> Value {
    Value::native("equal", Arity::Exact(2), |_, args| {
        Ok((args[0] == args[1]).into())
    })
}

pub(super) fn not_equal_global() -> Value {
    Value::native("not_equal", Arity::Exact(2), |_, args| {
        Ok((args[0] != args[1]).into())
    })
}

pub(super) fn less_than_global() -> Value {
    comparison("less_than", |o| o == Ordering::Less)
}

pub(super) fn less_than_or_equal_global() -> Value {
    comparison("less_than_or_equal", |o| o != Ordering::Greater)
}

pub(super) fn greater_than_global() -> Value {
    comparison("greater_than", |o| o == Ordering::Greater)
}

pub(super) fn greater_than_or_equal_global() -> Value {
    comparison("greater_than_or_equal", |o| o != Ordering::Less)
}
