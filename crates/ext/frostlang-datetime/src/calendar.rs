//! Converting time values between instants and components, and reading the
//! calendar.

use std::cmp::Ordering;

use frostlang::native::Optional;
use frostlang::{FrostResult, NativeCtx, Value, frost_fn};

use crate::value::{StrictTimeArg, TimeArg, TimeValue, date_required_error};
use crate::zone::ZoneArg;

#[frost_fn]
pub(crate) fn components(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: TimeArg,
    zone: Optional<ZoneArg>,
) -> FrostResult {
    let Some(value) = value.0 else {
        return Ok(Value::Null);
    };
    let zone = zone.0.map(|ZoneArg(zone)| zone);
    Ok(value.in_zone(zone, ctx.name())?.to_map())
}

#[frost_fn]
pub(crate) fn instant(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: TimeArg,
    zone: Optional<ZoneArg>,
) -> FrostResult {
    let Some(value) = value.0 else {
        return Ok(Value::Null);
    };
    let zone = zone.0.map(|ZoneArg(zone)| zone);
    let instant = value.instant(zone, ctx.name())?;
    Ok(Value::Int(instant.as_microsecond()))
}

#[frost_fn]
pub(crate) fn weekday(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: TimeArg,
    zone: Optional<ZoneArg>,
) -> FrostResult {
    let Some(value) = value.0 else {
        return Ok(Value::Null);
    };
    let zone = zone.0.map(|ZoneArg(zone)| zone);
    let components = match value {
        // A zone converts a value on the timeline, but a wall-clock value's date
        // is its own: pinning it could move it, as into a day a zone skipped.
        TimeValue::Components(components) if components.instant_if_on_timeline().is_none() => {
            components
        }
        on_timeline => on_timeline.in_zone(zone, ctx.name())?,
    };
    let date = components
        .date()
        .ok_or_else(|| date_required_error(ctx.name()))?;
    Ok(Value::Int(i64::from(date.weekday().to_monday_one_offset())))
}

#[frost_fn]
pub(crate) fn before(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    a: StrictTimeArg,
    b: StrictTimeArg,
) -> FrostResult {
    Ok(Value::Bool(a.0.order(b.0, ctx.name())? == Ordering::Less))
}
