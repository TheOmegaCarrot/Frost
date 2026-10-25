//! [`SpecialFloat`]: a NaN or infinite float, carried as an Opaque.

use crate::core::FrostOpaque;

/// A NaN or infinite `f64`, which [`FrostFloat`](crate::FrostFloat) cannot hold.
///
/// [`Value::from_f64`](crate::Value::from_f64), or deserializing a `Value` from any
/// format, turns a NaN or infinite float into an Opaque holding a `SpecialFloat`. A
/// `Value` serializes it back out as the same float, bit for bit, so data passing
/// through Frost keeps it.
///
/// Anywhere else, an `f64` or `f32` is a finite number: [`to_value`](crate::to_value)
/// and a native's result refuse a NaN or infinity, and
/// [`from_value`](crate::from_value) and a native's argument refuse a `SpecialFloat`
/// in place of one.
///
/// The exception is input that serde buffers while deserializing: untagged and
/// internally or adjacently tagged enums, and `#[serde(flatten)]` fields. There, an
/// `f64` or `f32` takes a `SpecialFloat` as the float it holds, and an `f32` rounds a
/// Float beyond its range to an infinity.
#[derive(Clone, Copy, Debug)]
pub struct SpecialFloat(f64);

impl SpecialFloat {
    /// Wraps `f`, or returns `None` if it is finite.
    pub fn new(f: f64) -> Option<Self> {
        (!f.is_finite()).then_some(Self(f))
    }

    /// The wrapped float: NaN, or positive or negative infinity.
    pub fn get(self) -> f64 {
        self.0
    }
}

impl FrostOpaque for SpecialFloat {
    fn type_name() -> &'static str {
        "SpecialFloat"
    }

    fn try_to_string(&self) -> Option<String> {
        Some(self.0.to_string())
    }

    /// Equal when the floats are bit for bit the same: unlike IEEE comparison,
    /// a NaN equals itself.
    fn equals(&self, other: &dyn FrostOpaque) -> bool {
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| self.0.to_bits() == other.0.to_bits())
    }
}
