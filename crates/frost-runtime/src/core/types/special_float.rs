//! [`SpecialFloat`]: a NaN or infinite float, carried as an Opaque.

use std::borrow::Cow;

use crate::core::FrostOpaque;

/// A NaN or infinite `f64`, which [`FrostFloat`](crate::FrostFloat) cannot hold.
///
/// Converting into a [`Value`](crate::Value) with [`to_value`](crate::to_value), or
/// deserializing one from any format, turns a NaN or infinite float into an Opaque
/// holding a `SpecialFloat`. It converts back out as the same float, bit for bit, so
/// data passing through Frost keeps it.
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
    fn type_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("SpecialFloat")
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
