//! Arithmetic and comparison operators as first-class functions.
//!
//! Each is a hand-rolled bytecode closure over the single opcode that implements
//! the operator, which dispatches to the same `Value` method a native would call.
//! Operators do no per-parameter type-check: their validity is a *relation* between
//! the two args (`Int + Int` is fine but `Int + String` is not, and `String + String`
//! / `Array + Array` are also fine), so the opcode's `Value` method raises the type
//! error when the operands do not combine.

use crate::{Arity, Bytecode, Value};

/// A two-argument operator: `DropBelow(2)` drops the closure's own value, leaving
/// `( lhs rhs )` for the single binary `op`.
fn binary_op(name: &'static str, op: Bytecode) -> Value {
    super::bytecode_global(name, Arity::Exact(2), vec![Bytecode::DropBelow(2), op])
}

pub(super) fn plus_global() -> Value {
    binary_op("plus", Bytecode::Add)
}

pub(super) fn minus_global() -> Value {
    binary_op("minus", Bytecode::Subtract)
}

pub(super) fn times_global() -> Value {
    binary_op("times", Bytecode::Multiply)
}

pub(super) fn divide_global() -> Value {
    binary_op("divide", Bytecode::Divide)
}

pub(super) fn mod_global() -> Value {
    binary_op("mod", Bytecode::Modulus)
}

pub(super) fn equal_global() -> Value {
    binary_op("equal", Bytecode::CompareEqual)
}

pub(super) fn not_equal_global() -> Value {
    binary_op("not_equal", Bytecode::CompareNotEqual)
}

pub(super) fn less_than_global() -> Value {
    binary_op("less_than", Bytecode::CompareLessThan)
}

pub(super) fn less_than_or_equal_global() -> Value {
    binary_op("less_than_or_equal", Bytecode::CompareLessThanOrEqual)
}

pub(super) fn greater_than_global() -> Value {
    binary_op("greater_than", Bytecode::CompareGreaterThan)
}

pub(super) fn greater_than_or_equal_global() -> Value {
    binary_op("greater_than_or_equal", Bytecode::CompareGreaterThanOrEqual)
}
