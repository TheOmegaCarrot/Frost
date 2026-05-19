use std::sync::Arc;

use crate::Value;

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(l), Value::Bool(r)) => l == r,
            (Value::Int(l), Value::Int(r)) => l == r,
            (Value::Float(l), Value::Float(r)) => l == r,
            (Value::String(l), Value::String(r)) => l == r,
            (Value::Array(l), Value::Array(r)) => l == r,
            (Value::Map(l), Value::Map(r)) => l == r,
            (Value::Function(l), Value::Function(r)) => Arc::ptr_eq(l, r),
            (Value::Opaque(l), Value::Opaque(r)) => Arc::ptr_eq(l, r),
            _ => false,
        }
    }
}

impl Eq for Value {}
