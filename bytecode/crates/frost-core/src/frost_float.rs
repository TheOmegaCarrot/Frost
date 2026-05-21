use std::{cmp::Ordering, ops::Deref};

use crate::error::FrostError;

/// A validated f64 that is guaranteed to never be NaN or Infinity.
/// This makes it safe to impl Eq and Ord.
#[derive(Clone, Copy, Debug)]
pub struct FrostFloat(f64);

impl FrostFloat {
    /// Creates a new FrostFloat, returning an error if the value is NaN or Infinity.
    pub fn new(f: f64) -> Result<Self, FrostError> {
        if f.is_nan() || f.is_infinite() {
            Err(FrostError::Recoverable(
                "Frost Float cannot be NaN or Infinity".into(),
            ))
        } else {
            Ok(Self(f))
        }
    }

    /// Returns the inner f64 value.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for FrostFloat {
    type Error = FrostError;
    fn try_from(f: f64) -> Result<FrostFloat, Self::Error> {
        FrostFloat::new(f)
    }
}

impl From<i64> for FrostFloat {
    fn from(i: i64) -> FrostFloat {
        FrostFloat::new(i as f64).expect("IMPOSSIBLE: Integer conversion to FrostFloat failed")
    }
}

impl From<FrostFloat> for f64 {
    fn from(value: FrostFloat) -> Self {
        value.0
    }
}

impl Deref for FrostFloat {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for FrostFloat {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for FrostFloat {}

impl PartialOrd for FrostFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FrostFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap()
    }
}
