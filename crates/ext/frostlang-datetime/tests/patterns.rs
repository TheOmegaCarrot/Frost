//! `format` and `parse`: time values as text by strftime patterns.

use crate::common::{assert_nulls, assert_raises, assert_true, assert_values, raises};

/// 2024-03-16T14:30:00Z, in microseconds since the Unix epoch.
const INSTANT: &str = "1710599400000000";

/// Assert `expression` raises an error that starts with `prefix`: the rest is
/// jiff's account of the problem.
fn assert_raises_starting(expression: &str, prefix: &str) {
    let message = raises(expression);
    assert!(
        message.starts_with(prefix),
        "{expression:?} raised {message:?}, not starting {prefix:?}"
    );
}

// --- format ---

#[test]
fn format_writes_a_zero_offset_as_any_other_and_only_the_utc_zone_as_utc() {
    assert_values(&[
        (
            &format!("dt.format({INSTANT}, '%Z %Q %z', 18000000000)"),
            "'+05 +0500 +0500'",
        ),
        (
            &format!("dt.format({INSTANT}, '%Z %Q %z', 0)"),
            "'+00 +0000 +0000'",
        ),
        (
            "dt.format('2024-03-16T14:30:00Z', '%Z %Q %z')",
            "'+00 +0000 +0000'",
        ),
        (
            &format!("dt.format({INSTANT}, '%Z %Q %z', 'UTC')"),
            "'UTC UTC +0000'",
        ),
    ]);
}

#[test]
fn format_writes_a_value_by_its_pattern() {
    assert_values(&[
        (
            &format!("dt.format({INSTANT}, '%Y-%m-%d %H:%M:%S %Z %z', 'America/New_York')"),
            "'2024-03-16 10:30:00 EDT -0400'",
        ),
        (
            &format!("dt.format({INSTANT}, '%Q', 'America/New_York')"),
            "'America/New_York'",
        ),
        (
            &format!("dt.format({INSTANT}, '%A, %B %-d', 'UTC')"),
            "'Saturday, March 16'",
        ),
        (&format!("dt.format({INSTANT}, '%j', 'UTC')"), "'076'"),
        (
            &format!("dt.format({INSTANT}, '%s', 'Asia/Tokyo')"),
            "'1710599400'",
        ),
        ("dt.format(1710599400123456, '%S%.f', 'UTC')", "'00.123456'"),
        (
            "dt.format({year: 2024, month: 3, day: 16}, '%d/%m/%Y')",
            "'16/03/2024'",
        ),
        ("dt.format({hour: 21, minute: 5}, '%I:%M %p')", "'09:05 PM'"),
        (
            "dt.format({year: 2024, month: 3, day: 16, hour: 9}, '%Y-%m-%dT%H')",
            "'2024-03-16T09'",
        ),
        ("dt.format({year: 2024, month: 3, day: 16}, '')", "''"),
    ]);
}

#[test]
fn format_without_a_zone_sees_an_instant_in_the_local_zone() {
    assert_true(&[&format!(
        r"
        def local = dt.local_zone()
        local == null or dt.format({INSTANT}, '%c %z') == dt.format({INSTANT}, '%c %z', local)
        "
    )]);
}

#[test]
fn format_raises_for_a_field_the_value_lacks_or_an_unknown_directive() {
    let prefix = "Function ext.datetime.format could not use its pattern: ";
    assert_raises_starting("dt.format({year: 2024, month: 3, day: 16}, '%z')", prefix);
    assert_raises_starting("dt.format({year: 2024, month: 3, day: 16}, '%H')", prefix);
    assert_raises_starting("dt.format({hour: 9}, '%Y')", prefix);
    assert_raises_starting(&format!("dt.format({INSTANT}, '%J')"), prefix);
}

// --- parse ---

#[test]
fn parse_gives_the_components_the_pattern_fixes() {
    assert_values(&[
        (
            "dt.parse('16/03/2024', '%d/%m/%Y')",
            "{year: 2024, month: 3, day: 16}",
        ),
        (
            "dt.parse('9:05 pm', '%I:%M %p')",
            "{hour: 21, minute: 5, second: 0, microsecond: 0}",
        ),
        (
            "dt.parse('2024-03-16 09:05', '%Y-%m-%d %H:%M')",
            "{year: 2024, month: 3, day: 16, hour: 9, minute: 5, second: 0, microsecond: 0}",
        ),
        (
            "dt.parse('2024-03-16 09:30 -0500', '%Y-%m-%d %H:%M %z')",
            "{year: 2024, month: 3, day: 16, hour: 9, minute: 30, second: 0, microsecond: 0, \
             offset: -18000000000}",
        ),
        (
            "dt.parse('2024-03-16 10:30 America/New_York', '%Y-%m-%d %H:%M %Q')",
            &format!("dt.components({INSTANT}, 'America/New_York')"),
        ),
        // A day of the year fixes a date as well as a month and day do.
        (
            "dt.parse('2024 076', '%Y %j')",
            "{year: 2024, month: 3, day: 16}",
        ),
        // An offset picks between repeated times, as in components.
        (
            "dt.instant(dt.parse('2024-11-03 01:30 -0500 America/New_York', '%Y-%m-%d %H:%M %z %Q'))",
            "1730615400000000",
        ),
    ]);
}

#[test]
fn parse_of_a_unix_timestamp_is_at_its_offset_or_utc() {
    assert_values(&[
        (
            "dt.parse('1710599400', '%s')",
            "{year: 2024, month: 3, day: 16, hour: 14, minute: 30, second: 0, microsecond: 0, \
             offset: 0}",
        ),
        (
            "dt.parse('1710599400 +0900', '%s %z')",
            "{year: 2024, month: 3, day: 16, hour: 23, minute: 30, second: 0, microsecond: 0, \
             offset: 32400000000}",
        ),
        ("dt.instant(dt.parse('1710599400', '%s'))", INSTANT),
    ]);
}

#[test]
fn parse_of_a_unix_timestamp_in_a_zone_must_have_the_zones_offset() {
    assert_values(&[(
        "dt.parse('1710599400 -0400 America/New_York', '%s %z %Q')",
        "dt.components(1710599400000000, 'America/New_York')",
    )]);
    assert_nulls(&["dt.parse('1710599400 +0900 America/New_York', '%s %z %Q')"]);
}

#[test]
fn parse_truncates_text_finer_than_a_microsecond() {
    assert_values(&[(
        "dt.parse('09:05:07.123456789', '%H:%M:%S%.f').microsecond",
        "123456",
    )]);
}

#[test]
fn parse_gives_null_for_text_that_does_not_match_or_is_no_datetime() {
    assert_nulls(&[
        "dt.parse('16-03-2024', '%d/%m/%Y')",
        "dt.parse('', '%d/%m/%Y')",
        "dt.parse('30/02/2024', '%d/%m/%Y')",
        "dt.parse('25:00', '%H:%M')",
        // 2024-03-16 was a Saturday.
        "dt.parse('Monday 2024-03-16', '%A %Y-%m-%d')",
        "dt.parse('2024-03-16 10:30 Mars/Olympus_Mons', '%Y-%m-%d %H:%M %Q')",
    ]);
}

#[test]
fn parse_raises_for_a_pattern_with_neither_a_date_nor_a_time_of_day() {
    assert_raises(&[
        (
            "dt.parse('2024-03', '%Y-%m')",
            "Function ext.datetime.parse requires a pattern with a date or a time of day, \
             got \"%Y-%m\"",
        ),
        (
            "dt.parse('', '')",
            "Function ext.datetime.parse requires a pattern with a date or a time of day, \
             got \"\"",
        ),
    ]);
}

#[test]
fn parse_raises_for_a_pattern_with_an_offset_or_zone_but_no_datetime() {
    let requirement = "Function ext.datetime.parse requires a pattern with an offset or zone to \
                       have a date and a time of day too, got";
    assert_raises(&[
        (
            "dt.parse('2024-03-16 -0500', '%Y-%m-%d %z')",
            &format!("{requirement} \"%Y-%m-%d %z\""),
        ),
        (
            "dt.parse('10:30 America/New_York', '%H:%M %Q')",
            &format!("{requirement} \"%H:%M %Q\""),
        ),
    ]);
}

#[test]
fn parse_raises_for_a_fraction_of_a_second_beside_a_unix_timestamp() {
    assert_raises(&[(
        "dt.parse('1700000000.5', '%s%.f')",
        "Function ext.datetime.parse requires a pattern with `%s` to have no fraction of a \
         second, got \"%s%.f\"",
    )]);
}

#[test]
fn parse_raises_for_a_pattern_it_cannot_use() {
    let prefix = "Function ext.datetime.parse could not use its pattern: ";
    assert_raises_starting("dt.parse('x', '%J')", prefix);
    // A zone's abbreviation, such as EST, names no one zone.
    assert_raises_starting("dt.parse('2024-03-16 EST', '%Y-%m-%d %Z')", prefix);
}

#[test]
fn format_and_parse_round_trip() {
    for zone in ["'UTC'", "'America/New_York'", "'Asia/Kolkata'"] {
        assert_values(&[(
            &format!(
                "dt.instant(dt.parse(dt.format({INSTANT}, '%Y-%m-%dT%H:%M:%S%.f %z %Q', {zone}), \
                 '%Y-%m-%dT%H:%M:%S%.f %z %Q'))"
            ),
            INSTANT,
        )]);
    }
}
