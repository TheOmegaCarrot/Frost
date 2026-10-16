use std::sync::Arc;

use crate::core::Value;

/// Frost's `==`, which never fails.
///
/// Values of different variants are never equal, so an `Int` never equals a
/// `Float`. Data compares by content: an `Array` or `Map` element by element. A
/// Function, native or closure, equals only itself: each evaluation of a lambda
/// expression makes a new Function, unequal to any other, even one with the
/// same code and captures. An `Opaque` compares as
/// [`FrostOpaque::equals`](crate::FrostOpaque::equals) describes.
///
/// Which values are the same Function or Opaque handle is decided by the
/// source, never by compiler options: no optimization merges two into one or
/// splits one into several.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(l), Value::Bool(r)) => l == r,
            (Value::Int(l), Value::Int(r)) => l == r,
            (Value::Float(l), Value::Float(r)) => l == r,
            (Value::String(l), Value::String(r)) => l == r,
            (Value::Bytes(l), Value::Bytes(r)) => l == r,
            (Value::Array(l), Value::Array(r)) => l == r,
            (Value::Map(l), Value::Map(r)) => l == r,
            (Value::NativeFunction(l), Value::NativeFunction(r)) => Arc::ptr_eq(l, r),
            (Value::Closure(l), Value::Closure(r)) => Arc::ptr_eq(l, r),
            (Value::Opaque(l), Value::Opaque(r)) => l.frost_eq(r),
            _ => false,
        }
    }
}

impl Eq for Value {}
