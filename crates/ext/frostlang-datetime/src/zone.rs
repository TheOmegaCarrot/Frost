//! Time zones: as scripts name them, and the local one.

use std::sync::LazyLock;

use frostlang::native::{ArgSite, FromArg};
use frostlang::{FrostError, FrostType, TypeSet, Value, frost_fn};
use jiff::civil::DateTime;
use jiff::tz::{AmbiguousOffset, AmbiguousZoned, Offset, OffsetConflict, TimeZone};
use jiff::{Timestamp, Unit, Zoned};

use crate::value::TimelineComponents;

/// Where a time value is seen or pinned: a time zone, or a fixed offset.
#[derive(Clone, Debug)]
pub(crate) enum Zone {
    /// A time zone, with its rules.
    Named(TimeZone),
    /// A fixed offset east of UTC, which no rules change.
    Fixed(Offset),
}

impl Zone {
    /// The local zone: the system's.
    pub(crate) fn local() -> Self {
        Zone::Named(TimeZone::system())
    }

    /// This zone as jiff's.
    fn time_zone(&self) -> TimeZone {
        // `TimeZone::fixed(Offset::ZERO)` is jiff's UTC zone, which `%Z` and
        // `%Q` write as `UTC`. A zero offset gets its own zone, so that they
        // write `+00` and `+0000`, as for any other offset.
        static ZERO_OFFSET: LazyLock<TimeZone> = LazyLock::new(|| {
            TimeZone::posix("<+00>0").expect("a fixed zero offset is a POSIX time zone")
        });
        match self {
            Zone::Named(zone) => zone.clone(),
            Zone::Fixed(offset) if *offset == Offset::ZERO => ZERO_OFFSET.clone(),
            Zone::Fixed(offset) => TimeZone::fixed(*offset),
        }
    }

    /// `zoned`, a datetime in this zone, as the components this zone gives.
    fn components_of(&self, zoned: Zoned) -> TimelineComponents {
        match self {
            Zone::Named(_) => TimelineComponents::Zoned(zoned),
            Zone::Fixed(_) => TimelineComponents::AtOffset(zoned),
        }
    }

    /// `instant` seen in this zone.
    pub(crate) fn view(&self, instant: Timestamp) -> TimelineComponents {
        self.components_of(instant.to_zoned(self.time_zone()))
    }

    /// Wall-clock `datetime` pinned in this zone, or None if that is beyond
    /// the years jiff supports.
    pub(crate) fn pin(&self, datetime: DateTime) -> Option<TimelineComponents> {
        let zoned = datetime.to_zoned(self.time_zone()).ok()?;
        Some(self.components_of(zoned))
    }

    /// Wall-clock `datetime` in this zone at `offset`, one the zone has then,
    /// or None if it is beyond the years jiff supports.
    pub(crate) fn pin_at(&self, datetime: DateTime, offset: Offset) -> Option<TimelineComponents> {
        let zoned = OffsetConflict::Reject
            .resolve(datetime, offset, self.time_zone())
            .and_then(AmbiguousZoned::compatible)
            .ok()?;
        Some(self.components_of(zoned))
    }

    /// Whether this zone has `offset` at `instant`, as [`names_offset`] reads
    /// it.
    pub(crate) fn has_offset_at(&self, instant: Timestamp, offset: Offset) -> bool {
        names_offset(offset, self.time_zone().to_offset(instant))
    }

    /// The offset this zone has at wall-clock `datetime` that `offset` names,
    /// as [`names_offset`] reads it, or None if it has none such: either offset
    /// of a time that occurs twice, and neither of a time that is skipped.
    ///
    /// An exact match wins over a rounded one: where a zone went from an
    /// offset with seconds to the whole minute it rounds to, as from local mean
    /// time, `offset` may name both.
    pub(crate) fn matching_offset(&self, datetime: DateTime, offset: Offset) -> Option<Offset> {
        let candidates = match self.time_zone().to_ambiguous_timestamp(datetime).offset() {
            AmbiguousOffset::Unambiguous { offset } => vec![offset],
            AmbiguousOffset::Fold { before, after } => vec![before, after],
            AmbiguousOffset::Gap { .. } => vec![],
        };
        let exact = candidates.iter().find(|&&actual| actual == offset);
        exact
            .or_else(|| {
                candidates
                    .iter()
                    .find(|&&actual| names_offset(offset, actual))
            })
            .copied()
    }
}

/// Whether `given` names a zone's `actual` offset: the same, or `actual`
/// rounded to the minute, as RFC 3339 text writes an offset with seconds. jiff
/// and Temporal read text so.
fn names_offset(given: Offset, actual: Offset) -> bool {
    given == actual
        || actual
            .round(Unit::Minute)
            .is_ok_and(|rounded| rounded == given)
}

/// A zone argument: an IANA time zone name, or Int microseconds of a fixed
/// offset east of UTC.
pub(crate) struct ZoneArg(pub(crate) Zone);

impl FromArg for ZoneArg {
    const TYPES: TypeSet = FrostType::String.or(FrostType::Int);

    fn from_arg(value: Value, site: &ArgSite<'_>) -> Result<Self, FrostError> {
        let zone = match &value {
            Value::String(name) => named_zone(name.as_str()).map(Zone::Named),
            Value::Int(micros) => offset_of_micros(*micros).map(Zone::Fixed),
            _ => unreachable!("a zone is a String or an Int"),
        };
        zone.map(Self).ok_or_else(|| {
            site.requires(format_args!(
                "a known time zone, or {OFFSET_REQUIREMENT}, got {}",
                value.to_debug_string()
            ))
        })
    }
}

/// What an offset given as an Int must be, for error messages.
pub(crate) const OFFSET_REQUIREMENT: &str =
    "Int microseconds of a whole number of seconds under 26 hours";

/// The time zone the IANA Time Zone Database names `name`, if any.
pub(crate) fn named_zone(name: &str) -> Option<TimeZone> {
    jiff::tz::db().get(name).ok()
}

/// The offset of `micros` east of UTC, if it is a whole number of seconds
/// within the range of an offset.
pub(crate) fn offset_of_micros(micros: i64) -> Option<Offset> {
    if micros % 1_000_000 != 0 {
        return None;
    }
    let seconds = i32::try_from(micros / 1_000_000).ok()?;
    Offset::from_seconds(seconds).ok()
}

#[frost_fn]
pub(crate) fn local_zone() -> Value {
    TimeZone::system()
        .iana_name()
        .map_or(Value::Null, Value::from)
}
