//! Time values as Frost represents them: Int microseconds, counted for an
//! instant from the Unix epoch.

use std::time::{SystemTime, UNIX_EPOCH};

/// `time` as Int microseconds since the Unix epoch, rounded toward the past,
/// or `None` if it is beyond an Int's range.
pub(crate) fn unix_micros(time: SystemTime) -> Option<i64> {
    match time.duration_since(UNIX_EPOCH) {
        Ok(since) => i64::try_from(since.as_micros()).ok(),
        Err(before) => i64::try_from(before.duration().as_nanos().div_ceil(1000))
            .ok()
            .map(|micros| -micros),
    }
}
