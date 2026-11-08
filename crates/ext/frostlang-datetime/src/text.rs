//! Time values as text: in standard forms, and by strftime patterns.

use frostlang::native::Optional;
use frostlang::{FrostError, FrostResult, FrostString, NativeCtx, Value, frost_fn};
use jiff::Zoned;
use jiff::civil::date;
use jiff::fmt::strtime::{self, BrokenDownTime};
use jiff::tz::{Offset, TimeZone};

use crate::value::{Components, TimeArg, TimelineComponents, components_at, to_microseconds};
use crate::zone::{Zone, ZoneArg, named_zone};

#[frost_fn]
pub(crate) fn to_string(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: TimeArg,
    zone: Optional<ZoneArg>,
) -> FrostResult {
    let Some(value) = value.0 else {
        return Ok(Value::Null);
    };
    let zone = zone.0.map(|ZoneArg(zone)| zone);
    let text = match value.in_zone(zone, ctx.name())? {
        Components::Date(date) => date.to_string(),
        Components::Time(time) => time.to_string(),
        Components::DateTime(datetime) => datetime.to_string(),
        Components::AtOffset(zoned) | Components::Zoned(zoned) => zoned_text(&zoned, None),
    };
    Ok(text.into())
}

#[frost_fn]
pub(crate) fn to_rfc3339(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: TimeArg,
    zone: Optional<ZoneArg>,
) -> FrostResult {
    let Some(value) = value.0 else {
        return Ok(Value::Null);
    };
    let zone = zone.0.map(|ZoneArg(zone)| zone);
    match value.on_timeline_in(zone, ctx.name())? {
        TimelineComponents::AtOffset(zoned) | TimelineComponents::Zoned(zoned) => {
            Ok(zoned_text(&zoned, None).into())
        }
    }
}

#[frost_fn]
pub(crate) fn to_rfc9557(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: TimeArg,
    zone: Optional<ZoneArg>,
) -> FrostResult {
    let Some(value) = value.0 else {
        return Ok(Value::Null);
    };
    let zone = zone.0.map(|ZoneArg(zone)| zone);
    match value.on_timeline_in(zone, ctx.name())? {
        TimelineComponents::AtOffset(zoned) => Ok(zoned_text(&zoned, None).into()),
        TimelineComponents::Zoned(zoned) => {
            Ok(zoned_text(&zoned, zoned.time_zone().iana_name()).into())
        }
    }
}

/// The RFC 3339 text of `zoned`, with RFC 9557's annotation of its zone's
/// `name`, if any.
///
/// jiff writes an offset rounded to the minute, which names another instant
/// when the offset has seconds, as some historical ones do. This writes them.
fn zoned_text(zoned: &Zoned, name: Option<&str>) -> String {
    let offset = zoned.offset().seconds();
    let sign = if offset < 0 { '-' } else { '+' };
    let (hours, minutes, seconds) = (
        offset.abs() / 3600,
        offset.abs() / 60 % 60,
        offset.abs() % 60,
    );
    let mut text = format!("{}{sign}{hours:02}:{minutes:02}", zoned.datetime());
    if seconds != 0 {
        text.push_str(&format!(":{seconds:02}"));
    }
    if let Some(name) = name {
        text.push_str(&format!("[{name}]"));
    }
    text
}

#[frost_fn]
pub(crate) fn format(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: TimeArg,
    pattern: FrostString,
    zone: Optional<ZoneArg>,
) -> FrostResult {
    let Some(value) = value.0 else {
        return Ok(Value::Null);
    };
    let zone = zone.0.map(|ZoneArg(zone)| zone);
    let broken_down = match value.in_zone(zone, ctx.name())? {
        Components::Date(date) => BrokenDownTime::from(date),
        Components::Time(time) => BrokenDownTime::from(time),
        Components::DateTime(datetime) => BrokenDownTime::from(datetime),
        Components::AtOffset(zoned) | Components::Zoned(zoned) => BrokenDownTime::from(&zoned),
    };
    broken_down
        .to_string(pattern.as_str())
        .map(Value::from)
        .map_err(|err| pattern_error(ctx.name(), &err))
}

#[frost_fn]
pub(crate) fn parse(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    text: FrostString,
    pattern: FrostString,
) -> FrostResult {
    let pattern = pattern.as_str();
    let fields = PatternFields::of(pattern).map_err(|err| pattern_error(ctx.name(), &err))?;
    if let Some(requirement) = fields.unmet_requirement() {
        return Err(FrostError::from_string(format!(
            "Function {} requires {requirement}, got {}",
            ctx.name(),
            Value::from(pattern).to_debug_string()
        )));
    }
    let Ok(parsed) = BrokenDownTime::parse(pattern, text.as_str()) else {
        return Ok(Value::Null);
    };
    Ok(fields
        .components(&parsed)
        .map_or(Value::Null, |components| components.to_map()))
}

/// The error for a call to `function` whose pattern jiff refused with `err`.
fn pattern_error(function: &str, err: &jiff::Error) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} could not use its pattern: {err}"
    ))
}

/// What a strftime pattern has when parsing.
struct PatternFields {
    /// A date.
    date: bool,
    /// A time of day.
    time: bool,
    /// An instant: `%s`.
    instant: bool,
    /// An offset or a zone.
    offset_or_zone: bool,
    /// A fraction of a second.
    fraction: bool,
}

impl PatternFields {
    /// What `pattern` has: what it parses back from a reference datetime
    /// formatted by it. Fails if it cannot format or parse that.
    fn of(pattern: &str) -> Result<Self, jiff::Error> {
        let reference = date(2001, 2, 3)
            .at(16, 5, 6, 7_008_000)
            .to_zoned(TimeZone::UTC)
            .expect("the reference datetime is in range");
        let parsed = BrokenDownTime::parse(pattern, strtime::format(pattern, &reference)?)?;
        Ok(Self {
            date: parsed.to_date().is_ok(),
            time: parsed.hour().is_some(),
            instant: parsed.timestamp().is_some(),
            offset_or_zone: parsed.offset().is_some() || parsed.iana_time_zone().is_some(),
            fraction: parsed.subsec_nanosecond().is_some(),
        })
    }

    /// What a pattern with these fields lacks to give components of a clear
    /// kind, if anything.
    fn unmet_requirement(&self) -> Option<&'static str> {
        if !self.date && !self.time && !self.instant {
            Some("a pattern with a date or a time of day")
        } else if self.offset_or_zone && !self.instant && !(self.date && self.time) {
            Some("a pattern with an offset or zone to have a date and a time of day too")
        } else if self.instant && self.fraction {
            // jiff reads `-0.5` by `%s%.f` as 0 and a half, losing the sign.
            Some("a pattern with `%s` to have no fraction of a second")
        } else {
            None
        }
    }

    /// The components of `parsed`, a text parsed by a pattern with these
    /// fields, or None if they are not a valid datetime: as with standard
    /// text, that includes an offset the zone does not have then.
    fn components(&self, parsed: &BrokenDownTime) -> Option<Components> {
        let zone = match parsed.iana_time_zone() {
            None => None,
            Some(name) => Some(Zone::Named(named_zone(name)?)),
        };
        if self.instant {
            let instant = parsed.timestamp()?;
            let zone = match (zone, parsed.offset()) {
                (Some(zone), Some(offset)) if !zone.has_offset_at(instant, offset) => return None,
                (Some(zone), _) => zone,
                (None, offset) => Zone::Fixed(offset.unwrap_or(Offset::UTC)),
            };
            return Some(zone.view(instant).into());
        }
        let date = self.date.then(|| parsed.to_date()).transpose().ok()?;
        let time = self
            .time
            .then(|| parsed.to_time())
            .transpose()
            .ok()?
            .map(to_microseconds);
        match (date, time) {
            (Some(date), Some(time)) => {
                components_at(date.to_datetime(time), parsed.offset(), zone)
            }
            (Some(date), None) => Some(Components::Date(date)),
            (None, Some(time)) => Some(Components::Time(time)),
            (None, None) => unreachable!("the pattern has a date or a time of day"),
        }
    }
}
