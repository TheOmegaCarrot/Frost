use std::{collections::BTreeMap, sync::Arc};

use crate::{FrostArray, FrostError, FrostFloat, FrostMap, Value};

impl Value {
    /// Frost `+` operator: numeric addition, string/array concatenation, or map merge.
    pub fn add(&self, rhs: &Value) -> Result<Value, FrostError> {
        match (self, rhs) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::from(l.wrapping_add(*r))),

            (Value::Float(l), Value::Float(r)) => {
                Ok(Value::from(FrostFloat::try_from(l.get() + r.get())?))
            }

            (Value::Int(l), Value::Float(r)) => {
                Ok(Value::from(FrostFloat::try_from(*l as f64 + r.get())?))
            }

            (Value::Float(l), Value::Int(r)) => {
                Ok(Value::from(FrostFloat::try_from(l.get() + *r as f64)?))
            }

            (Value::String(l), Value::String(r)) => {
                Ok(Value::from(Arc::from([l.as_ref(), r.as_ref()].concat())))
            }

            (Value::Array(l), Value::Array(r)) => Ok(Value::from(FrostArray::from(
                [l.as_slice(), r.as_slice()].concat(),
            ))),

            (Value::Map(l), Value::Map(r)) => Ok(Value::Map(
                l.iter()
                    .chain(r.iter())
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            )),

            _ => Err(binop_type_error("add", "+", self, rhs)),
        }
    }

    /// Frost `-` operator: numeric subtraction.
    pub fn subtract(&self, rhs: &Value) -> Result<Value, FrostError> {
        match (self, rhs) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::from(l.wrapping_sub(*r))),

            (Value::Float(l), Value::Float(r)) => {
                Ok(Value::from(FrostFloat::try_from(l.get() - r.get())?))
            }

            (Value::Int(l), Value::Float(r)) => {
                Ok(Value::from(FrostFloat::try_from(*l as f64 - r.get())?))
            }

            (Value::Float(l), Value::Int(r)) => {
                Ok(Value::from(FrostFloat::try_from(l.get() - *r as f64)?))
            }

            _ => Err(binop_type_error("subtract", "-", self, rhs)),
        }
    }

    /// Frost `*` operator: numeric multiplication.
    pub fn multiply(&self, rhs: &Value) -> Result<Value, FrostError> {
        match (self, rhs) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::from(l.wrapping_mul(*r))),

            (Value::Float(l), Value::Float(r)) => {
                Ok(Value::from(FrostFloat::try_from(l.get() * r.get())?))
            }

            (Value::Int(l), Value::Float(r)) => {
                Ok(Value::from(FrostFloat::try_from(*l as f64 * r.get())?))
            }

            (Value::Float(l), Value::Int(r)) => {
                Ok(Value::from(FrostFloat::try_from(l.get() * *r as f64)?))
            }

            _ => Err(binop_type_error("multiply", "*", self, rhs)),
        }
    }

    /// Frost `/` operator: numeric division. Integer division truncates toward zero.
    pub fn divide(&self, rhs: &Value) -> Result<Value, FrostError> {
        match (self, rhs) {
            (Value::Int(_) | Value::Float(_), Value::Int(0)) => {
                Err(FrostError::Recoverable("Division by zero".into()))
            }

            (Value::Int(_) | Value::Float(_), Value::Float(f)) if f.get() == 0.0 => {
                Err(FrostError::Recoverable("Division by zero".into()))
            }

            (Value::Int(l), Value::Int(r)) => Ok(Value::from(l.wrapping_div(*r))),

            (Value::Float(l), Value::Float(r)) => {
                Ok(Value::from(FrostFloat::try_from(l.get() / r.get())?))
            }

            (Value::Int(l), Value::Float(r)) => {
                Ok(Value::from(FrostFloat::try_from(*l as f64 / r.get())?))
            }

            (Value::Float(l), Value::Int(r)) => {
                Ok(Value::from(FrostFloat::try_from(l.get() / *r as f64)?))
            }

            _ => Err(binop_type_error("divide", "/", self, rhs)),
        }
    }

    /// Frost `%` operator: integer-only modulus.
    pub fn modulus(&self, rhs: &Value) -> Result<Value, FrostError> {
        match (self, rhs) {
            (Value::Int(_), Value::Int(0)) => {
                Err(FrostError::Recoverable("Modulus by zero".into()))
            }

            (Value::Int(l), Value::Int(r)) => Ok(Value::from(l.wrapping_rem(*r))),

            _ => Err(binop_type_error("modulus", "%", self, rhs)),
        }
    }
}

fn binop_type_error(verb: &str, glyph: &str, lhs: &Value, rhs: &Value) -> FrostError {
    FrostError::Recoverable(format!(
        "Cannot {verb} incompatible types: {} {glyph} {}",
        lhs.type_name(),
        rhs.type_name()
    ))
}
