//! Time values as scripts give them: Int microseconds, components Maps, and
//! text.

use std::cmp::Ordering;
use std::ops::RangeInclusive;

use frostlang::native::{ArgSite, FromArg};
use frostlang::{FrostError, FrostType, MapKey, TypeSet, Value, from_value};
use jiff::civil::{Date, DateTime, Time};
use jiff::fmt::temporal::{
    DateTimeParser, Pieces, PiecesOffset, TimeZoneAnnotation, TimeZoneAnnotationKind,
};
use jiff::tz::{Offset, TimeZone};
use jiff::{Timestamp, Zoned};
use serde::Deserialize;

use crate::zone::{OFFSET_REQUIREMENT, Zone, named_zone, offset_of_micros};

/// The years components may have, for error messages.
const YEARS: &str = "the years -9999 to 9999";

/// A time value: an instant, or calendar components.
#[derive(Clone, Debug)]
pub(crate) enum TimeValue {
    /// An instant, given as Int microseconds since the Unix epoch. It is on the
    /// timeline but has no calendar view until it is seen in a zone.
    Instant(Timestamp),
    /// Calendar components.
    Components(Components),
}

/// Calendar components, of one kind each.
#[derive(Clone, Debug)]
pub(crate) enum Components {
    /// A date.
    Date(Date),
    /// A time of day.
    Time(Time),
    /// A wall-clock datetime, in no zone.
    DateTime(DateTime),
    /// A datetime at a fixed offset, as [`Zone::Fixed`] gives it, and so on the
    /// timeline.
    AtOffset(Zoned),
    /// A datetime in a time zone, as [`Zone::Named`] gives it, and so on the
    /// timeline.
    Zoned(Zoned),
}

/// Components on the timeline: the kinds of [`Components`] a zone gives.
#[derive(Clone, Debug)]
pub(crate) enum TimelineComponents {
    /// As [`Components::AtOffset`].
    AtOffset(Zoned),
    /// As [`Components::Zoned`].
    Zoned(Zoned),
}

impl TimelineComponents {
    /// The instant these components are.
    pub(crate) fn instant(&self) -> Timestamp {
        match self {
            TimelineComponents::AtOffset(zoned) | TimelineComponents::Zoned(zoned) => {
                zoned.timestamp()
            }
        }
    }
}

impl From<TimelineComponents> for Components {
    fn from(components: TimelineComponents) -> Self {
        match components {
            TimelineComponents::AtOffset(zoned) => Components::AtOffset(zoned),
            TimelineComponents::Zoned(zoned) => Components::Zoned(zoned),
        }
    }
}

/// A time value argument: an Int, a components Map, or text. Text that is not
/// a time value holds `None`.
pub(crate) struct TimeArg(pub(crate) Option<TimeValue>);

impl FromArg for TimeArg {
    const TYPES: TypeSet = FrostType::Int.or(FrostType::String).or(FrostType::Map);

    fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError> {
        match value {
            Value::Int(micros) => {
                let Some(instant) = instant_of_micros(micros) else {
                    return Err(instant_range_error(site, micros));
                };
                Ok(Self(Some(TimeValue::Instant(instant))))
            }
            Value::String(text) => Ok(Self(read_text(text.as_str()).map(TimeValue::Components))),
            map => read_map(map)
                .map(|components| Self(Some(TimeValue::Components(components))))
                .map_err(|problem| site.requires(format_args!("valid components: {problem}"))),
        }
    }
}

/// A time value argument that must be one: text that is not a time value
/// raises.
pub(crate) struct StrictTimeArg(pub(crate) TimeValue);

impl FromArg for StrictTimeArg {
    const TYPES: TypeSet = TimeArg::TYPES;

    fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError> {
        match TimeArg::from_arg(value.clone(), site)? {
            TimeArg(Some(time)) => Ok(Self(time)),
            TimeArg(None) => Err(site.requires(format_args!(
                "a time value, got {}",
                value.to_debug_string()
            ))),
        }
    }
}

/// The instant `micros` microseconds after the Unix epoch, if jiff has it.
fn instant_of_micros(micros: i64) -> Option<Timestamp> {
    // Unlike `Timestamp::from_microsecond`, this reaches the microseconds of
    // the last second jiff has.
    let nanosecond = i32::try_from(micros.rem_euclid(1_000_000) * 1000)
        .expect("a fraction of a second in nanoseconds fits an i32");
    Timestamp::new(micros.div_euclid(1_000_000), nanosecond).ok()
}

/// The error for `micros` at `site`, an instant jiff does not have.
fn instant_range_error(site: &ArgSite<'_>, micros: i64) -> FrostError {
    let last = instant_of_micros(Timestamp::MAX.as_microsecond())
        .expect("the last whole microsecond is in range");
    site.requires(format_args!(
        "an instant from {} to {last}, got {micros}",
        Timestamp::MIN
    ))
}

/// The error for a call to `function` with a value that cannot be placed on
/// the timeline, or seen in a zone, within the years jiff supports.
pub(crate) fn beyond_years_error(function: &str) -> FrostError {
    FrostError::from_string(format!("Function {function} has no result within {YEARS}"))
}

/// The error for a call to `function` with a time of day where it needs a
/// date.
pub(crate) fn date_required_error(function: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires a date, got a time of day"
    ))
}

impl TimeValue {
    /// This value's components in `zone`: a value on the timeline is converted
    /// to it, and a wall-clock value pinned there. A date pins to the start of
    /// its day; a time of day alone cannot be pinned.
    ///
    /// Without `zone`, an instant is seen in the local zone, and components stay
    /// as they are.
    pub(crate) fn in_zone(
        self,
        zone: Option<Zone>,
        function: &str,
    ) -> Result<Components, FrostError> {
        match (self, zone) {
            (TimeValue::Components(components), None) => Ok(components),
            (value, zone) => Ok(value.on_timeline_in(zone, function)?.into()),
        }
    }

    /// This value on the timeline, in `zone`: as [`in_zone`](Self::in_zone),
    /// but without `zone` a wall-clock value is pinned to the local zone.
    pub(crate) fn on_timeline_in(
        self,
        zone: Option<Zone>,
        function: &str,
    ) -> Result<TimelineComponents, FrostError> {
        let components = match self {
            TimeValue::Instant(instant) => {
                return Ok(zone.unwrap_or_else(Zone::local).view(instant));
            }
            TimeValue::Components(components) => components,
        };
        match (components, zone) {
            (Components::AtOffset(zoned), None) => Ok(TimelineComponents::AtOffset(zoned)),
            (Components::Zoned(zoned), None) => Ok(TimelineComponents::Zoned(zoned)),
            (Components::AtOffset(zoned) | Components::Zoned(zoned), Some(zone)) => {
                Ok(zone.view(zoned.timestamp()))
            }
            (Components::DateTime(datetime), zone) => zone
                .unwrap_or_else(Zone::local)
                .pin(datetime)
                .ok_or_else(|| beyond_years_error(function)),
            (Components::Date(date), zone) => zone
                .unwrap_or_else(Zone::local)
                .pin(date.to_datetime(Time::midnight()))
                .ok_or_else(|| beyond_years_error(function)),
            (Components::Time(_), _) => Err(date_required_error(function)),
        }
    }

    /// The instant this value is. A wall-clock value is pinned to `zone`, or
    /// without one the local zone.
    pub(crate) fn instant(
        self,
        zone: Option<Zone>,
        function: &str,
    ) -> Result<Timestamp, FrostError> {
        Ok(self.on_timeline_in(zone, function)?.instant())
    }

    /// The instant this value is, if it is on the timeline; a wall-clock value
    /// is not pinned.
    fn instant_if_on_timeline(&self) -> Option<Timestamp> {
        match self {
            TimeValue::Instant(instant) => Some(*instant),
            TimeValue::Components(components) => components.instant_if_on_timeline(),
        }
    }

    /// This kind of value, for an error message.
    fn kind(&self) -> &'static str {
        match self {
            TimeValue::Instant(_) => "an instant",
            TimeValue::Components(components) => components.kind(),
        }
    }

    /// How this value orders against `other`: as instants if both are on the
    /// timeline, or as components if both are wall-clock values of one kind.
    ///
    /// A wall-clock value raises against one on the timeline. Pinning it would
    /// give no consistent order: a skipped time moves forward past wall-clock
    /// times later than it.
    pub(crate) fn order(self, other: TimeValue, function: &str) -> Result<Ordering, FrostError> {
        if let (Some(this), Some(other)) = (
            self.instant_if_on_timeline(),
            other.instant_if_on_timeline(),
        ) {
            return Ok(this.cmp(&other));
        }
        let kinds = (self.kind(), other.kind());
        match (self, other) {
            (
                TimeValue::Components(Components::Date(a)),
                TimeValue::Components(Components::Date(b)),
            ) => Ok(a.cmp(&b)),
            (
                TimeValue::Components(Components::Time(a)),
                TimeValue::Components(Components::Time(b)),
            ) => Ok(a.cmp(&b)),
            (
                TimeValue::Components(Components::DateTime(a)),
                TimeValue::Components(Components::DateTime(b)),
            ) => Ok(a.cmp(&b)),
            _ => Err(FrostError::from_string(format!(
                "Function {function} cannot order {} against {}",
                kinds.0, kinds.1
            ))),
        }
    }
}

impl Components {
    /// The instant these components are, if they are on the timeline; a
    /// wall-clock value is not pinned.
    pub(crate) fn instant_if_on_timeline(&self) -> Option<Timestamp> {
        match self {
            Components::AtOffset(zoned) | Components::Zoned(zoned) => Some(zoned.timestamp()),
            _ => None,
        }
    }

    /// The date these components hold, if any.
    pub(crate) fn date(&self) -> Option<Date> {
        match self {
            Components::Date(date) => Some(*date),
            Components::Time(_) => None,
            Components::DateTime(datetime) => Some(datetime.date()),
            Components::AtOffset(zoned) | Components::Zoned(zoned) => Some(zoned.date()),
        }
    }

    /// The time of day these components hold, if any.
    fn time(&self) -> Option<Time> {
        match self {
            Components::Date(_) => None,
            Components::Time(time) => Some(*time),
            Components::DateTime(datetime) => Some(datetime.time()),
            Components::AtOffset(zoned) | Components::Zoned(zoned) => Some(zoned.time()),
        }
    }

    /// This kind of components, for an error message.
    fn kind(&self) -> &'static str {
        match self {
            Components::Date(_) => "a date",
            Components::Time(_) => "a time of day",
            Components::DateTime(_) => "a wall-clock datetime",
            Components::AtOffset(_) => "a datetime at an offset",
            Components::Zoned(_) => "a zoned datetime",
        }
    }

    /// These components as a Map: every key of their kind.
    pub(crate) fn to_map(&self) -> Value {
        let date = self.date().map(|date| {
            [
                ("year", i64::from(date.year())),
                ("month", i64::from(date.month())),
                ("day", i64::from(date.day())),
            ]
        });
        let time = self.time().map(|time| {
            [
                ("hour", i64::from(time.hour())),
                ("minute", i64::from(time.minute())),
                ("second", i64::from(time.second())),
                ("microsecond", i64::from(time.subsec_nanosecond() / 1000)),
            ]
        });
        let (offset, zone) = match self {
            Components::AtOffset(zoned) => (Some(zoned.offset()), None),
            Components::Zoned(zoned) => (Some(zoned.offset()), zoned.time_zone().iana_name()),
            _ => (None, None),
        };
        let offset = offset.map(|offset| ("offset", i64::from(offset.seconds()) * 1_000_000));
        let numbers = date
            .into_iter()
            .flatten()
            .chain(time.into_iter().flatten())
            .chain(offset)
            .map(|(key, number)| (MapKey::from(key), Value::Int(number)));
        let zone = zone.map(|name| (MapKey::from("zone"), Value::from(name)));
        numbers.chain(zone).collect()
    }
}

// --- Reading a Map ---

/// A components Map, as a script writes it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ComponentsMap {
    year: Option<i64>,
    month: Option<i64>,
    day: Option<i64>,
    hour: Option<i64>,
    minute: Option<i64>,
    second: Option<i64>,
    microsecond: Option<i64>,
    offset: Option<i64>,
    zone: Option<String>,
}

/// The components `map` holds, or what is wrong with it.
fn read_map(map: Value) -> Result<Components, String> {
    let ComponentsMap {
        year,
        month,
        day,
        hour,
        minute,
        second,
        microsecond,
        offset,
        zone,
    } = from_value(map).map_err(|err| err.message().into_owned())?;
    let date = date_from_fields(year, month, day)?;
    let time = time_from_fields(hour, minute, second, microsecond)?;
    let offset = offset.map(offset_from_field).transpose()?;
    let zone = zone.as_deref().map(zone_from_field).transpose()?;
    match (date, time) {
        (Some(date), Some(time)) => datetime_from_fields(date.to_datetime(time), offset, zone),
        (None, None) => Err("components need a date or a time of day".to_string()),
        _ if offset.is_some() || zone.is_some() => {
            Err("`offset` and `zone` need both a date and a time of day".to_string())
        }
        (Some(date), None) => Ok(Components::Date(date)),
        (None, Some(time)) => Ok(Components::Time(time)),
    }
}

/// `value`, the named field, as a `T` within `range`.
fn checked_field<T: TryFrom<i64>>(
    name: &str,
    value: i64,
    range: RangeInclusive<i64>,
) -> Result<T, String> {
    if !range.contains(&value) {
        return Err(format!(
            "`{name}` must be {} to {}, got {value}",
            range.start(),
            range.end()
        ));
    }
    Ok(T::try_from(value).unwrap_or_else(|_| unreachable!("the range fits the type")))
}

/// The date of the fields `year`, `month`, and `day`, if given, or what is
/// wrong with them. A date needs all three.
fn date_from_fields(
    year: Option<i64>,
    month: Option<i64>,
    day: Option<i64>,
) -> Result<Option<Date>, String> {
    let (year, month, day) = match (year, month, day) {
        (None, None, None) => return Ok(None),
        (Some(year), Some(month), Some(day)) => (year, month, day),
        _ => return Err("a date needs `year`, `month`, and `day`".to_string()),
    };
    let year = checked_field("year", year, -9999..=9999)?;
    let month = checked_field("month", month, 1..=12)?;
    let first = Date::new(year, month, 1).expect("the first of a valid month is a date");
    let day = checked_field("day", day, 1..=i64::from(first.days_in_month()))?;
    Ok(Some(
        first
            .with()
            .day(day)
            .build()
            .expect("a day within its month is a date"),
    ))
}

/// The time of day of the fields given, if any, or what is wrong with them. A
/// time of day needs `hour`; the rest default to 0.
fn time_from_fields(
    hour: Option<i64>,
    minute: Option<i64>,
    second: Option<i64>,
    microsecond: Option<i64>,
) -> Result<Option<Time>, String> {
    let hour = match (hour, minute, second, microsecond) {
        (None, None, None, None) => return Ok(None),
        (Some(hour), ..) => hour,
        _ => return Err("a time of day needs `hour`".to_string()),
    };
    let hour = checked_field("hour", hour, 0..=23)?;
    let minute = checked_field("minute", minute.unwrap_or(0), 0..=59)?;
    let second = checked_field("second", second.unwrap_or(0), 0..=59)?;
    let microsecond: i32 = checked_field("microsecond", microsecond.unwrap_or(0), 0..=999_999)?;
    Ok(Some(
        Time::new(hour, minute, second, microsecond * 1000).expect("fields in range are a time"),
    ))
}

/// The offset of the field `offset`, or what is wrong with it.
fn offset_from_field(micros: i64) -> Result<Offset, String> {
    offset_of_micros(micros)
        .ok_or_else(|| format!("`offset` must be {OFFSET_REQUIREMENT}, got {micros}"))
}

/// The zone the field `zone` names, or what is wrong with it.
fn zone_from_field(name: &str) -> Result<Zone, String> {
    named_zone(name).map(Zone::Named).ok_or_else(|| {
        format!(
            "`zone` must name a known time zone, got {}",
            Value::from(name).to_debug_string()
        )
    })
}

/// The components of a Map's `datetime` at its `offset` and in its `zone`, if
/// given, or what is wrong with them.
fn datetime_from_fields(
    datetime: DateTime,
    offset: Option<Offset>,
    zone: Option<Zone>,
) -> Result<Components, String> {
    let beyond_years = || format!("it is beyond {YEARS} on the timeline");
    let (Some(offset), Some(zone)) = (offset, &zone) else {
        return components_at(datetime, offset, zone).ok_or_else(beyond_years);
    };
    // A Map's offset must be the zone's exactly: the minute rounding allowed
    // in text does not apply.
    if zone.matching_offset(datetime, offset) != Some(offset) {
        return Err(format!(
            "`offset` must be one `zone` has at that time, got {}",
            i64::from(offset.seconds()) * 1_000_000
        ));
    }
    zone.pin_at(datetime, offset)
        .map(Into::into)
        .ok_or_else(beyond_years)
}

/// The components of `datetime` at `offset` and in `zone`, if given, or None
/// if the zone does not have the offset then or that is beyond the years jiff
/// supports.
///
/// With a zone, the offset picks between the two times of a fold. It may be
/// the zone's offset rounded to the minute, as text writes it; the zone's own
/// exact offset places the datetime.
pub(crate) fn components_at(
    datetime: DateTime,
    offset: Option<Offset>,
    zone: Option<Zone>,
) -> Option<Components> {
    match (offset, zone) {
        (None, None) => Some(Components::DateTime(datetime)),
        (Some(offset), None) => Zone::Fixed(offset).pin(datetime).map(Into::into),
        (None, Some(zone)) => zone.pin(datetime).map(Into::into),
        (Some(offset), Some(zone)) => {
            let exact = zone.matching_offset(datetime, offset)?;
            zone.pin_at(datetime, exact).map(Into::into)
        }
    }
}

// --- Reading text ---

/// The components that ISO 8601, RFC 3339, or RFC 9557 `text` holds, or
/// None if it holds none.
fn read_text(text: &str) -> Option<Components> {
    static PARSER: DateTimeParser = DateTimeParser::new();
    if let Ok(pieces) = PARSER.parse_pieces(text) {
        return read_pieces(&pieces);
    }
    // jiff's time parser accepts and drops an offset or zone annotation, but a
    // time of day alone has no place on the timeline. No time of day's text
    // holds these characters otherwise.
    if text.contains(['+', '-', 'Z', 'z', '[']) {
        return None;
    }
    let time = PARSER.parse_time(text).ok()?;
    Some(Components::Time(to_microseconds(time)))
}

/// The components of parsed `pieces`, or None if they place a date alone,
/// name an unknown zone, or give an offset the zone does not have then.
fn read_pieces(pieces: &Pieces<'_>) -> Option<Components> {
    let offset = match pieces.offset() {
        None => None,
        Some(PiecesOffset::Zulu) => Some(Offset::UTC),
        Some(PiecesOffset::Numeric(numeric)) => Some(numeric.offset()),
        Some(_) => return None,
    };
    let zone = match pieces.time_zone_annotation().map(TimeZoneAnnotation::kind) {
        None => None,
        Some(TimeZoneAnnotationKind::Named(name)) => Some(Zone::Named(named_zone(name.as_str())?)),
        Some(TimeZoneAnnotationKind::Offset(offset)) => Some(Zone::Fixed(*offset)),
        // An annotation of a kind jiff adds later is not yet a zone here.
        Some(_) => return None,
    };
    let Some(time) = pieces.time() else {
        return (offset.is_none() && zone.is_none()).then(|| Components::Date(pieces.date()));
    };
    let datetime = pieces.date().to_datetime(to_microseconds(time));
    if let (Some(PiecesOffset::Zulu), Some(zone)) = (pieces.offset(), &zone) {
        // `Z` gives the instant in UTC, leaving the zone to give the offset.
        let instant = datetime.to_zoned(TimeZone::UTC).ok()?.timestamp();
        return Some(zone.view(instant).into());
    }
    components_at(datetime, offset, zone)
}

/// `time` truncated to whole microseconds, Frost's precision.
pub(crate) fn to_microseconds(time: Time) -> Time {
    let nanosecond = time.subsec_nanosecond();
    time.with()
        .subsec_nanosecond(nanosecond - nanosecond % 1000)
        .build()
        .expect("a smaller fraction of the same second is a time")
}
