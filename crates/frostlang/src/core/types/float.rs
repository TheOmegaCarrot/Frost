use std::cmp::Ordering;

use crate::core::error::FrostError;

/// A validated f64 that is guaranteed to never be NaN or Infinity.
/// This makes it safe to impl Eq and Ord.
///
/// Serialized as a plain `f64`; deserialization rejects NaN and Infinity.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(into = "f64", try_from = "f64")]
pub struct FrostFloat(f64);

impl FrostFloat {
    /// Creates a new FrostFloat, returning an error if the value is NaN or Infinity.
    pub fn new(f: f64) -> Result<Self, FrostError> {
        if f.is_nan() || f.is_infinite() {
            Err("Frost Float cannot be NaN or Infinity".into())
        } else {
            Ok(Self(f))
        }
    }

    /// Returns the inner f64 value.
    pub fn get(&self) -> f64 {
        self.0
    }

    /// The Int equal to this Float, if it is a whole number within the Int range.
    pub(crate) fn to_exact_int(self) -> Option<i64> {
        // The Int range as floats: -2^63 is exact, and 2^63 is just past the end.
        const INT_START: f64 = -9_223_372_036_854_775_808.0;
        const INT_END: f64 = 9_223_372_036_854_775_808.0;
        let whole = self.0.fract() == 0.0 && (INT_START..INT_END).contains(&self.0);
        whole.then_some(self.0 as i64)
    }

    /// Compares this Float with an Int exactly, without rounding the Int to a Float.
    pub(crate) fn cmp_int(self, int: i64) -> Ordering {
        let whole = FrostFloat(self.0.trunc());
        match whole.to_exact_int() {
            // The fraction breaks a tie between the whole part and the Int.
            Some(whole_int) => whole_int
                .cmp(&int)
                .then(self.0.fract().partial_cmp(&0.0).expect("a finite fraction")),
            // Past the Int range on one side or the other.
            None if whole.0 < 0.0 => Ordering::Less,
            None => Ordering::Greater,
        }
    }
}

impl TryFrom<f64> for FrostFloat {
    type Error = FrostError;
    fn try_from(f: f64) -> Result<FrostFloat, Self::Error> {
        FrostFloat::new(f)
    }
}

impl From<FrostFloat> for f64 {
    fn from(value: FrostFloat) -> Self {
        value.0
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
