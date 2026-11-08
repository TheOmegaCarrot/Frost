//! Durations as friendly text, such as `1h 30m`.

use frostlang::{FrostString, Value, frost_fn};
use jiff::fmt::friendly::{Designator, Direction, SpanPrinter};
use jiff::{SignedDuration, Span, SpanRelativeTo, SpanRound, Unit};

#[frost_fn]
pub(crate) fn parse_duration(text: FrostString) -> Value {
    let text = text.as_str();
    // A signed duration reads exact units as far as an Int reaches, but no
    // days or weeks; a span reads those, as far as about 20,000 years, and
    // refuses months and years, which have no fixed length.
    text.parse::<SignedDuration>()
        .or_else(|_| {
            text.parse::<Span>()
                .and_then(|span| span.to_duration(SpanRelativeTo::days_are_24_hours()))
        })
        .ok()
        .and_then(nearest_micros)
        .map_or(Value::Null, Value::Int)
}

/// `duration` in whole microseconds, a half rounding away from zero, if that
/// fits an Int.
fn nearest_micros(duration: SignedDuration) -> Option<i64> {
    let nanos = duration.as_nanos();
    i64::try_from((nanos + nanos.signum() * 500) / 1000).ok()
}

#[frost_fn]
pub(crate) fn format_duration(duration: i64) -> String {
    // Human-time designators are the compact ones, but `us` rather than `µs`:
    // ASCII, and what a script author types.
    static PRINTER: SpanPrinter = SpanPrinter::new()
        .designator(Designator::HumanTime)
        .direction(Direction::Sign);
    let duration = SignedDuration::from_micros(duration);
    let in_days = Span::try_from(duration)
        .and_then(|span| span.round(SpanRound::new().largest(Unit::Day).days_are_24_hours()));
    match in_days {
        Ok(span) => PRINTER.span_to_string(&span),
        // A span holds about 20,000 years; a longer duration is in hours.
        Err(_) => PRINTER.duration_to_string(&duration),
    }
}
