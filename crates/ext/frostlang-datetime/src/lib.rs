//! Dates, times, and time zones for Frost scripts: calendar components, IANA
//! time zones, and text, built on Frost's Int microseconds.
//!
//! A host installs the extension with [`extension`], and scripts import it as
//! `ext.datetime`:
//!
//! ```
//! use frostlang::ImporterBuilder;
//!
//! let importer = ImporterBuilder::new()
//!     .with_extension(frostlang_datetime::extension())
//!     .expect("no other extension is named `datetime`")
//!     .build();
//! ```
//!
//! Time zones come from the system's copy of the IANA Time Zone Database. On
//! platforms that never have one, such as Windows, the extension carries its
//! own copy.
//!
//! The `bundled-tzdb` feature builds that copy in on every platform, as a
//! fallback: where the system has a copy, it is still used instead. Enable it
//! for systems that may lack the database, such as minimal containers.

mod calendar;
mod durations;
mod text;
mod value;
mod zone;

use frostlang::Extension;

/// The `ext.datetime` extension.
///
/// A time value is any of:
///
/// - An instant: Int microseconds since the Unix epoch, as `std.os.now()` and
///   `std.fs.stat` give, and as `std.time` builds durations.
/// - Components: a Map whose keys give its kind. `year`, `month`, and `day`
///   make a date; `hour`, `minute`, `second`, and `microsecond` a time of day,
///   with all but `hour` defaulting to 0; both make a wall-clock datetime. With
///   both, `offset` (Int microseconds east of UTC, a whole number of seconds)
///   places it on the timeline, and `zone` (an IANA name, such as
///   `'America/New_York'`) places it by that zone's rules. Any other key is an
///   error. A script may build or edit a Map freely; a function given one
///   raises if it is not valid components.
/// - Text in ISO 8601, RFC 3339, or RFC 9557 form, such as `'2024-03-16'`,
///   `'2024-03-16T14:30:00Z'`, or `'2024-03-16T14:30:00-04:00[America/New_York]'`.
///
/// An instant, and a datetime with an offset or a zone, are on the timeline. A
/// date, a time of day, and a datetime with neither are wall-clock values.
///
/// - An offset fixes a datetime's instant. `'2024-03-16T14:30:00Z'` and
///   `{..., offset: 0}` are 14:30 UTC. A zone beside an offset only says which
///   zone's wall clock shows it: `'2024-03-16T14:30:00Z[America/New_York]'` is
///   still 14:30 UTC, 10:30 in New York.
/// - A zone alone pins a datetime's wall-clock time by its rules:
///   `'2024-03-16T10:30[America/New_York]'` is 10:30 in New York, as is the
///   wall-clock `'2024-03-16T10:30:00'` given the zone argument
///   `'America/New_York'`.
///
/// Where a function takes a `zone`, it is an IANA name or Int microseconds of a
/// fixed offset. It converts a value on the timeline, and pins a wall-clock
/// value there, a date at the start of its day. Without one, a value stays as
/// it is, except where an instant must be seen on the calendar, or a
/// wall-clock value placed on the timeline: then the system's local zone is
/// used.
///
/// An offset and a zone together must agree: the offset must be one the zone
/// has at that time. A Map where they disagree, as after editing the month of
/// one that has both, raises; drop its `offset` to let the zone give it. Text
/// where they disagree is not a time value.
///
/// Where an offset has seconds, as some historical ones do, `to_string`,
/// `to_rfc3339`, and `to_rfc9557` write them, though RFC 3339 has none:
/// rounding would name another instant. Text read back may round such an
/// offset to the minute; a Map must give it exactly.
///
/// Where a wall-clock time occurs twice, as clocks go back, the earlier is
/// taken, unless an `offset` picks the other. Where it is skipped, as clocks go
/// forward, it moves forward by the gap.
///
/// A zone key, or a zone in text, appears only where a zone was named: a zero
/// offset is an offset like any other, and only the zone `'UTC'` is UTC.
///
/// - `components(value, zone?)` gives a value's components Map, with every key
///   of its kind.
/// - `instant(value, zone?)` gives the instant a value is.
/// - `to_string(value, zone?)` gives a value's text in the standard form of its
///   kind: ISO 8601 for a date, a time of day, or a wall-clock datetime, and
///   RFC 3339 for a value on the timeline, as most readers of timestamps
///   expect.
/// - `to_rfc3339(value, zone?)` gives the RFC 3339 text of the instant a value
///   is, such as `'2024-03-16T10:30:00-04:00'`, pinning a wall-clock value as
///   `instant` does.
/// - `to_rfc9557(value, zone?)` gives the same with the zone's name, as in
///   `'2024-03-16T10:30:00-04:00[America/New_York]'`, where a zone is named.
///   Reading it back keeps the zone's rules; readers of plain RFC 3339 reject
///   it.
/// - `format(value, pattern, zone?)` gives a value's text by a strftime
///   pattern, such as `'%Y-%m-%d %H:%M'`. It raises if the pattern needs a field
///   the value lacks.
/// - `parse(text, pattern)` gives the components of text by a strftime
///   pattern: those the pattern has, or Null if the text does not match. A
///   pattern must have a date or a time of day. A pattern with an offset or
///   zone must also have a date and a time of day. A pattern with `%s` cannot
///   have a fraction of a second.
/// - `weekday(value, zone?)` gives the ISO weekday of a value's date, from 1
///   for Monday to 7 for Sunday. A zone converts a value on the timeline; a
///   wall-clock value's date is its own.
/// - `before(a, b)` gives whether `a` comes before `b`, for `sorted`. Values on
///   the timeline compare as instants, and wall-clock values of one kind by
///   their components. Other pairs raise, a wall-clock value against one on
///   the timeline included: pin it with `instant` first. Text that is not a
///   time value raises, so that every comparison has an answer.
/// - `local_zone()` gives the IANA name of the system's time zone, or Null if
///   it has none.
/// - `parse_duration(text)` gives the Int microseconds of friendly duration
///   text, such as `'1h 30m'`, or ISO 8601 text, such as `'PT1H30M'`. A day is
///   24 hours and a week 7 days; months and years have no fixed length, so text
///   with them is not a duration. Text in hours and smaller units reaches as
///   far as an Int; text with days or weeks, about 20,000 years.
/// - `format_duration(duration)` gives the friendly text of Int microseconds,
///   in units up to days, such as `'1d 2h 30m'`, with a leading `-` if
///   negative.
///
/// Text that is not a time value or duration gives Null, except where noted.
/// Years run from -9999 to 9999, and instants stop a day short of either end,
/// so that every instant has a date at every offset. Text finer than a
/// microsecond is truncated, and a duration rounded to the nearest.
pub fn extension() -> Extension {
    Extension::builder("datetime")
        .function(calendar::components_frost())
        .function(calendar::instant_frost())
        .function(text::to_string_frost())
        .function(text::to_rfc3339_frost())
        .function(text::to_rfc9557_frost())
        .function(text::format_frost())
        .function(text::parse_frost())
        .function(calendar::weekday_frost())
        .function(calendar::before_frost())
        .function(zone::local_zone_frost())
        .function(durations::parse_duration_frost())
        .function(durations::format_duration_frost())
        .build()
}
