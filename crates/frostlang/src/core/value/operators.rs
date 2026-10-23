use std::sync::Arc;

use crate::core::{FrostError, FrostFloat, Value};

impl Value {
    /// Frost `+` operator: numeric addition, String/Bytes/Array concatenation,
    /// or Map merge (the right operand wins on a key collision).
    pub fn add(&self, rhs: &Value) -> Result<Value, FrostError> {
        match (self, rhs) {
            (Value::Int(l), Value::Int(r)) => int_result(l.checked_add(*r), l, "+", r),

            (Value::Float(l), Value::Float(r)) => {
                Ok(Value::from(FrostFloat::try_from(l.get() + r.get())?))
            }

            (Value::Int(l), Value::Float(r)) => {
                Ok(Value::from(FrostFloat::try_from(*l as f64 + r.get())?))
            }

            (Value::Float(l), Value::Int(r)) => {
                Ok(Value::from(FrostFloat::try_from(l.get() + *r as f64)?))
            }

            // Concatenation is same-type: text joins text, bytes join bytes.
            // Mixing them falls to the type error below rather than picking a side.
            (Value::String(l), Value::String(r)) => Ok(Value::from(format!("{l}{r}"))),

            (Value::Bytes(l), Value::Bytes(r)) => {
                Ok(Value::from([l.as_slice(), r.as_slice()].concat()))
            }

            // Structural `+` has one home: `add_owned`. The clones are Arc bumps,
            // and the now-shared storage makes it copy exactly as a borrowing merge would.
            (Value::Array(_), Value::Array(_)) | (Value::Map(_), Value::Map(_)) => {
                Self::add_owned(self.clone(), rhs.clone())
            }

            _ => Err(binop_type_error("add", "+", self, rhs)),
        }
    }

    /// Frost `+` on owned operands.
    ///
    /// Same semantics as [`add`](Self::add),
    /// but `Array + Array` and `Map + Map` reuse an operand's storage instead of copying when it is not shared.
    /// Prefer this form when the operands are owned and can be given up;
    /// use [`add`](Self::add) when only references are available.
    pub fn add_owned(lhs: Value, rhs: Value) -> Result<Value, FrostError> {
        match (lhs, rhs) {
            // `make_mut` extends a uniquely-owned lhs where it is, allocation and all,
            // and copies it only when shared.
            (Value::Array(mut l), Value::Array(r)) => {
                Arc::make_mut(&mut l.inner).extend(r.into_vec());
                Ok(Value::Array(l))
            }

            (Value::Map(mut l), Value::Map(r)) => {
                Arc::make_mut(&mut l.inner).extend(r.into_map()); // on key collision rhs wins
                Ok(Value::Map(l))
            }

            // Scalars, strings, and type errors have nothing worth stealing.
            (lhs, rhs) => lhs.add(&rhs),
        }
    }

    /// Frost `-` operator: numeric subtraction.
    pub fn subtract(&self, rhs: &Value) -> Result<Value, FrostError> {
        match (self, rhs) {
            (Value::Int(l), Value::Int(r)) => int_result(l.checked_sub(*r), l, "-", r),

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
            (Value::Int(l), Value::Int(r)) => int_result(l.checked_mul(*r), l, "*", r),

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
            (Value::Int(_) | Value::Float(_), Value::Int(0)) => Err("Division by zero".into()),

            (Value::Int(_) | Value::Float(_), Value::Float(f)) if f.get() == 0.0 => {
                Err("Division by zero".into())
            }

            // Only `Int::MIN / -1` overflows; division by zero is handled above.
            (Value::Int(l), Value::Int(r)) => int_result(l.checked_div(*r), l, "/", r),

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
            (Value::Int(_), Value::Int(0)) => Err("Modulus by zero".into()),

            // The true remainder always fits, so this never overflows:
            // `Int::MIN % -1` is 0, which `wrapping_rem` gives exactly.
            (Value::Int(l), Value::Int(r)) => Ok(Value::from(l.wrapping_rem(*r))),

            _ => Err(binop_type_error("modulus", "%", self, rhs)),
        }
    }

    /// Frost unary `-`: numeric negation. A non-numeric operand is a type error.
    pub fn negate(&self) -> Result<Value, FrostError> {
        match self {
            Value::Int(i) => i
                .checked_neg()
                .map(Value::from)
                .ok_or_else(|| FrostError::from_string(format!("Int overflow: -({i})"))),
            // Negating a float that is already non-NaN and finite cannot produce
            // NaN or Infinity, so the unwrap cannot fail.
            Value::Float(f) => Ok(Value::Float(FrostFloat::new(-f.get()).unwrap())),
            _ => Err(FrostError::from_string(format!(
                "Cannot negate value of type {}",
                self.type_name()
            ))),
        }
    }
}

/// The Int result of `lhs glyph rhs`, `checked` by the matching `checked_*`
/// operation: an overflow error when the true result is outside the Int range.
fn int_result(
    checked: Option<i64>,
    lhs: &i64,
    glyph: &str,
    rhs: &i64,
) -> Result<Value, FrostError> {
    checked
        .map(Value::from)
        .ok_or_else(|| FrostError::from_string(format!("Int overflow: {lhs} {glyph} {rhs}")))
}

fn binop_type_error(verb: &str, glyph: &str, lhs: &Value, rhs: &Value) -> FrostError {
    format!(
        "Cannot {verb} incompatible types: {} {glyph} {}",
        lhs.type_name(),
        rhs.type_name()
    )
    .into()
}
