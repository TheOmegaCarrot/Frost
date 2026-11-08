//! `parse_duration` and `format_duration`: durations as friendly text.

use crate::common::{assert_nulls, assert_values};

/// Microseconds in a second, a minute, an hour, and a day.
const SECOND: i64 = 1_000_000;
const MINUTE: i64 = 60 * SECOND;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;

#[test]
fn parse_duration_reads_friendly_text() {
    let cases = [
        ("'1h 30m'", HOUR + 30 * MINUTE),
        ("'1h30m'", HOUR + 30 * MINUTE),
        ("'1 hour 30 minutes'", HOUR + 30 * MINUTE),
        ("'1.5h'", HOUR + 30 * MINUTE),
        ("'90s'", 90 * SECOND),
        ("'250ms'", 250_000),
        ("'5us'", 5),
        ("'0s'", 0),
        ("'-5m'", -5 * MINUTE),
        ("'5m ago'", -5 * MINUTE),
    ];
    for (text, micros) in cases {
        assert_values(&[(&format!("dt.parse_duration({text})"), &micros.to_string())]);
    }
}

#[test]
fn parse_duration_reads_iso_8601_text() {
    assert_values(&[
        (
            "dt.parse_duration('PT1H30M')",
            &(HOUR + 30 * MINUTE).to_string(),
        ),
        ("dt.parse_duration('P2D')", &(2 * DAY).to_string()),
        ("dt.parse_duration('-PT1S')", &(-SECOND).to_string()),
    ]);
}

#[test]
fn a_day_is_24_hours_and_a_week_7_days() {
    assert_values(&[
        ("dt.parse_duration('1d')", &DAY.to_string()),
        ("dt.parse_duration('7 days')", &(7 * DAY).to_string()),
        ("dt.parse_duration('1w')", &(7 * DAY).to_string()),
        (
            "dt.parse_duration('1w 1d 1h')",
            &(8 * DAY + HOUR).to_string(),
        ),
    ]);
}

#[test]
fn a_duration_rounds_to_the_nearest_microsecond() {
    assert_values(&[
        ("dt.parse_duration('1499ns')", "1"),
        // A half rounds away from zero, as `std.time` rounds it.
        ("dt.parse_duration('1500ns')", "2"),
        ("dt.parse_duration('-1500ns')", "-2"),
        ("dt.parse_duration('499ns')", "0"),
    ]);
}

#[test]
fn text_that_is_no_duration_gives_null() {
    assert_nulls(&[
        "dt.parse_duration('')",
        "dt.parse_duration('soon')",
        "dt.parse_duration('5 parsecs')",
        // Months and years are no fixed length.
        "dt.parse_duration('1 month')",
        "dt.parse_duration('2y')",
        "dt.parse_duration('P1M')",
        // Beyond Int microseconds either way.
        "dt.parse_duration('2562047789h')",
        "dt.parse_duration('-2562047789h')",
    ]);
}

#[test]
fn days_reach_about_20_000_years() {
    assert_values(&[(
        "dt.parse_duration('7304484d')",
        &(7_304_484 * DAY).to_string(),
    )]);
    assert_nulls(&["dt.parse_duration('7304485d')"]);
}

#[test]
fn format_duration_writes_friendly_text_up_to_days() {
    let cases = [
        (0, "0s"),
        // ASCII `us`, which `parse_duration` reads too.
        (5, "5us"),
        (1_500_001, "1s 500ms 1us"),
        (HOUR + 30 * MINUTE, "1h 30m"),
        (3 * DAY, "3d"),
        (8 * DAY + HOUR, "8d 1h"),
        (-90 * SECOND, "-1m 30s"),
    ];
    for (micros, text) in cases {
        assert_values(&[(
            &format!("dt.format_duration({micros})"),
            &format!("'{text}'"),
        )]);
    }
}

#[test]
fn format_duration_writes_any_int() {
    // Too long for days, as jiff counts them: in hours instead.
    assert_values(&[
        (
            "dt.format_duration(9223372036854775807)",
            "'2562047788h 54s 775ms 807us'",
        ),
        (
            "dt.format_duration(-9223372036854775807 - 1)",
            "'-2562047788h 54s 775ms 808us'",
        ),
    ]);
}

#[test]
fn format_and_parse_round_trip() {
    for micros in [
        0,
        1,
        -1,
        1_500_001,
        HOUR + 30 * MINUTE,
        3 * DAY + 7,
        -8 * DAY - HOUR,
        i64::MAX,
        i64::MIN,
    ] {
        assert_values(&[(
            &format!("dt.parse_duration(dt.format_duration({micros}))"),
            &micros.to_string(),
        )]);
    }
}
