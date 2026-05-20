use crate::{FrostError, FrostFloat, Value};

impl Value {
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
}

fn binop_type_error(verb: &str, glyph: &str, lhs: &Value, rhs: &Value) -> FrostError {
    FrostError::Recoverable(format!(
        "Cannot {verb} incompatible types: {} {glyph} {}",
        lhs.type_name(),
        rhs.type_name()
    ))
}
