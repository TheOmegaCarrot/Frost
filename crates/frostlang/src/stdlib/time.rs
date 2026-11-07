//! `std.time`: durations in human units.
//!
//! Frost represents every duration and instant as Int microseconds; an
//! instant counts from the Unix epoch.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::native::Numeric;
use crate::{FrostError, FrostFloat, StdlibModule, Value, frost_closure};

const MILLISECOND: i64 = 1_000;
const SECOND: i64 = 1_000 * MILLISECOND;
const MINUTE: i64 = 60 * SECOND;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;

/// The `std.time` module: functions converting between Int microseconds and
/// human units of time, such as `seconds(5)` and `as_seconds(duration)`.
///
/// It reads no clock: [`os`](crate::stdlib::os) does.
pub fn time() -> StdlibModule {
    StdlibModule::builder("time")
        .value(
            "milliseconds",
            unit_constructor("std.time.milliseconds", MILLISECOND),
        )
        .value("seconds", unit_constructor("std.time.seconds", SECOND))
        .value("minutes", unit_constructor("std.time.minutes", MINUTE))
        .value("hours", unit_constructor("std.time.hours", HOUR))
        .value("days", unit_constructor("std.time.days", DAY))
        .value("weeks", unit_constructor("std.time.weeks", WEEK))
        .value(
            "as_milliseconds",
            unit_inverse("std.time.as_milliseconds", MILLISECOND),
        )
        .value("as_seconds", unit_inverse("std.time.as_seconds", SECOND))
        .value("as_minutes", unit_inverse("std.time.as_minutes", MINUTE))
        .value("as_hours", unit_inverse("std.time.as_hours", HOUR))
        .value("as_days", unit_inverse("std.time.as_days", DAY))
        .value("as_weeks", unit_inverse("std.time.as_weeks", WEEK))
        .build()
}

/// The function taking a count of a unit `unit` microseconds long to Int
/// microseconds, rounded to the nearest.
fn unit_constructor(name: &'static str, unit: i64) -> Value {
    frost_closure!(name, move |#[frost(unnamed)] count: Numeric| {
        let micros = match count {
            Numeric::Int(count) => count.checked_mul(unit),
            // An infinite product is past the Int range too.
            Numeric::Float(count) => FrostFloat::new((count.get() * unit as f64).round())
                .ok()
                .and_then(FrostFloat::to_exact_int),
        };
        micros.map(Value::Int).ok_or_else(|| {
            FrostError::from_string(format!(
                "Function {name} has no Int result for {}",
                Value::from(count).to_frost_string()
            ))
        })
    })
}

/// The function taking Int microseconds to a Float count of a unit `unit`
/// microseconds long.
fn unit_inverse(name: &'static str, unit: i64) -> Value {
    frost_closure!(name, move |#[frost(unnamed)] micros: i64| {
        micros as f64 / unit as f64
    })
}

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
