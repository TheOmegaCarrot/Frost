use std::{cmp::Ordering, sync::Arc};

use crate::core::{FrostFloat, Value};

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

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Value::Null, Value::Null) => None,
            (Value::Bool(_), Value::Bool(_)) => None,
            (Value::Int(l), Value::Int(r)) => l.partial_cmp(r),
            (Value::Float(l), Value::Float(r)) => l.partial_cmp(r),
            (Value::String(l), Value::String(r)) => l.partial_cmp(r),
            (Value::Array(l), Value::Array(r)) => l.partial_cmp(r),
            (Value::Map(l), Value::Map(r)) => None,
            (Value::Function(l), Value::Function(r)) => None,
            (Value::Opaque(l), Value::Opaque(r)) => None,

            (Value::Int(l), Value::Float(r)) => FrostFloat::from(*l).partial_cmp(r),
            (Value::Float(l), Value::Int(r)) => l.partial_cmp(&FrostFloat::from(*r)),
            _ => None,
        }
    }
}
