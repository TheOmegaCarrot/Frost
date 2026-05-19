use std::sync::Arc;

use crate::{FrostArray, FrostMap, Value};

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Int(l), Value::Int(r)) => l == r,
            (Value::Float(l), Value::Float(r)) => l == r,
            (Value::Bool(l), Value::Bool(r)) => l == r,
            (Value::String(l), Value::String(r)) => l == r,
            (Value::Array(l), Value::Array(r)) => array_eq(l, r),
            (Value::Map(l), Value::Map(r)) => map_eq(l, r),
            (Value::Function(l), Value::Function(r)) => Arc::ptr_eq(l, r),
            (Value::Opaque(l), Value::Opaque(r)) => Arc::ptr_eq(l, r),
            _ => false,
        }
    }
}

impl Eq for Value {}

fn array_eq(l: &FrostArray, r: &FrostArray) -> bool {
    l.len() == r.len() && l.iter().zip(r.iter()).all(|(l_, r_)| l_ == r_)
}

fn map_eq(l: &FrostMap, r: &FrostMap) -> bool {
    l.len() == r.len()
        && l.iter()
            .zip(r.iter())
            .all(|((lk, lv), (rk, rv))| lk == rk && lv == rv)
}
