//! Time values as text in standard forms: `to_string`, `to_rfc3339`,
//! `to_rfc9557`, and text taken wherever a time value is.

use crate::common::{assert_nulls, assert_raises, assert_true, assert_values};

/// 2024-03-16T14:30:00Z, in microseconds since the Unix epoch.
const INSTANT: &str = "1710599400000000";

#[test]
fn to_string_gives_the_standard_form_of_each_kind() {
    assert_values(&[
        (
            "dt.to_string({year: 2024, month: 3, day: 16})",
            "'2024-03-16'",
        ),
        (
            "dt.to_string({year: -44, month: 3, day: 15})",
            "'-000044-03-15'",
        ),
        (
            "dt.to_string({hour: 9, minute: 5, second: 7})",
            "'09:05:07'",
        ),
        (
            "dt.to_string({hour: 9, microsecond: 8})",
            "'09:00:00.000008'",
        ),
        (
            "dt.to_string({year: 2024, month: 3, day: 16, hour: 9, minute: 5})",
            "'2024-03-16T09:05:00'",
        ),
        (
            &format!("dt.to_string({INSTANT}, -18000000000)"),
            "'2024-03-16T09:30:00-05:00'",
        ),
        (
            &format!("dt.to_string({INSTANT}, 0)"),
            "'2024-03-16T14:30:00+00:00'",
        ),
        // A value in a zone is RFC 3339 too: the offset, with no annotation.
        (
            &format!("dt.to_string({INSTANT}, 'America/New_York')"),
            "'2024-03-16T10:30:00-04:00'",
        ),
        (
            &format!("dt.to_string({INSTANT}, 'UTC')"),
            "'2024-03-16T14:30:00+00:00'",
        ),
        (
            "dt.to_string(1710599400123456, 'UTC')",
            "'2024-03-16T14:30:00.123456+00:00'",
        ),
    ]);
}

#[test]
fn to_rfc3339_gives_the_instant_with_its_offset() {
    assert_values(&[
        (
            &format!("dt.to_rfc3339({INSTANT}, 'America/New_York')"),
            "'2024-03-16T10:30:00-04:00'",
        ),
        (
            &format!("dt.to_rfc3339({INSTANT}, 0)"),
            "'2024-03-16T14:30:00+00:00'",
        ),
        (
            "dt.to_rfc3339('2024-03-16T10:30:00-04:00[America/New_York]')",
            "'2024-03-16T10:30:00-04:00'",
        ),
        // A wall-clock value is pinned, a date at the start of its day.
        (
            "dt.to_rfc3339('2024-03-16T10:30', 'America/New_York')",
            "'2024-03-16T10:30:00-04:00'",
        ),
        (
            "dt.to_rfc3339('2024-03-16', 'Asia/Tokyo')",
            "'2024-03-16T00:00:00+09:00'",
        ),
    ]);
}

#[test]
fn to_rfc9557_names_the_zone_where_one_is_named() {
    assert_values(&[
        (
            &format!("dt.to_rfc9557({INSTANT}, 'America/New_York')"),
            "'2024-03-16T10:30:00-04:00[America/New_York]'",
        ),
        (
            &format!("dt.to_rfc9557({INSTANT}, 'UTC')"),
            "'2024-03-16T14:30:00+00:00[UTC]'",
        ),
        (
            "dt.to_rfc9557('2024-03-16T10:30', 'America/New_York')",
            "'2024-03-16T10:30:00-04:00[America/New_York]'",
        ),
        // An offset names no zone.
        (
            &format!("dt.to_rfc9557({INSTANT}, -18000000000)"),
            "'2024-03-16T09:30:00-05:00'",
        ),
        (
            "dt.to_rfc9557('2024-03-16T14:30:00Z')",
            "'2024-03-16T14:30:00+00:00'",
        ),
    ]);
}

#[test]
fn the_rfc_forms_pin_a_wall_clock_value_to_the_local_zone_by_default() {
    assert_true(&[r"
        def local = dt.local_zone()
        def wall_clock = '2024-03-16T10:30'
        def by_default = [dt.to_rfc3339(wall_clock), dt.to_rfc9557(wall_clock)]
        local == null or by_default == [dt.to_rfc3339(wall_clock, local), dt.to_rfc9557(wall_clock, local)]
        "]);
}

#[test]
fn the_rfc_forms_raise_for_a_time_of_day_alone() {
    for function in ["to_rfc3339", "to_rfc9557"] {
        assert_raises(&[(
            &format!("dt.{function}('10:30')"),
            &format!("Function ext.datetime.{function} requires a date, got a time of day"),
        )]);
    }
}

#[test]
fn to_rfc9557_round_trips_a_zone() {
    assert_values(&[(
        "dt.components(dt.to_rfc9557(1710599400000001, 'Europe/London'))",
        "dt.components(1710599400000001, 'Europe/London')",
    )]);
}

#[test]
fn to_string_without_a_zone_sees_an_instant_in_the_local_zone() {
    assert_true(&[&format!(
        r"
        def local = dt.local_zone()
        local == null or dt.to_string({INSTANT}) == dt.to_string({INSTANT}, local)
        "
    )]);
}

#[test]
fn text_reads_as_the_components_it_holds() {
    assert_values(&[
        (
            "dt.components('2024-03-16')",
            "{year: 2024, month: 3, day: 16}",
        ),
        (
            "dt.components('09:05')",
            "{hour: 9, minute: 5, second: 0, microsecond: 0}",
        ),
        (
            "dt.components('2024-03-16T09:05:07')",
            "{year: 2024, month: 3, day: 16, hour: 9, minute: 5, second: 7, microsecond: 0}",
        ),
        // A space may separate the date and time, as TOML and SQL write them.
        (
            "dt.components('2024-03-16 09:05:07')",
            "{year: 2024, month: 3, day: 16, hour: 9, minute: 5, second: 7, microsecond: 0}",
        ),
        (
            "dt.components('2024-03-16T14:30:00Z')",
            "{year: 2024, month: 3, day: 16, hour: 14, minute: 30, second: 0, microsecond: 0, \
             offset: 0}",
        ),
        (
            "dt.components('2024-03-16T09:30:00-05:00')",
            "{year: 2024, month: 3, day: 16, hour: 9, minute: 30, second: 0, microsecond: 0, \
             offset: -18000000000}",
        ),
        (
            "dt.components('2024-03-16T10:30:00-04:00[America/New_York]')",
            &format!("dt.components({INSTANT}, 'America/New_York')"),
        ),
        // A zone with no offset takes the zone's.
        (
            "dt.components('2024-03-16T10:30[America/New_York]')",
            &format!("dt.components({INSTANT}, 'America/New_York')"),
        ),
    ]);
}

#[test]
fn text_finer_than_a_microsecond_is_truncated() {
    assert_values(&[
        ("dt.components('09:05:07.123456789').microsecond", "123456"),
        (
            "dt.instant('2024-03-16T14:30:00.999999999Z')",
            "1710599400999999",
        ),
        // Truncating the wall-clock time floors an instant before the epoch too.
        ("dt.instant('1969-12-31T23:59:59.9999999Z')", "-1"),
    ]);
}

#[test]
fn text_is_a_time_value_everywhere() {
    assert_values(&[
        ("dt.instant('2024-03-16T14:30:00Z')", INSTANT),
        ("dt.weekday('2024-03-16')", "6"),
        ("dt.before('2024-03-16', '2024-03-17')", "true"),
        ("dt.format('2024-03-16', '%d/%m/%Y')", "'16/03/2024'"),
        (
            "dt.to_rfc9557('2024-03-16T14:30:00Z', 'Asia/Tokyo')",
            "'2024-03-16T23:30:00+09:00[Asia/Tokyo]'",
        ),
    ]);
}

#[test]
fn to_string_round_trips_each_kind() {
    let values = [
        "{year: 2024, month: 3, day: 16}",
        "{hour: 9, minute: 5, second: 7, microsecond: 8}",
        "{year: 2024, month: 3, day: 16, hour: 9, minute: 5, second: 7, microsecond: 8}",
        "{year: 2024, month: 3, day: 16, hour: 9, minute: 0, second: 0, microsecond: 0, \
         offset: -18000000000}",
        "{year: 2024, month: 3, day: 16, hour: 9, minute: 0, second: 0, microsecond: 0, \
         offset: 0}",
        "dt.components(1710599400000001, 1000000)",
    ];
    for value in values {
        assert_values(&[(
            &format!("dt.components(dt.to_string({value}))"),
            &format!("dt.components({value})"),
        )]);
    }
}

#[test]
fn text_that_is_no_time_value_gives_null_from_every_function() {
    for text in [
        "''",
        "'tomorrow'",
        "'2024-13-01'",
        "'2024-03-16T14:30:00+99:00'",
    ] {
        assert_nulls(&[
            &format!("dt.components({text})"),
            &format!("dt.instant({text})"),
            &format!("dt.to_string({text})"),
            &format!("dt.format({text}, '%Y')"),
            &format!("dt.weekday({text})"),
        ]);
        // A comparator always answers, so `before` raises instead.
        let shown = format!("\"{}\"", text.trim_matches('\''));
        assert_raises(&[
            (
                &format!("dt.before({text}, 0)"),
                &format!(
                    "Function ext.datetime.before requires argument 1 (a) to be a time value, \
                     got {shown}"
                ),
            ),
            (
                &format!("dt.before(0, {text})"),
                &format!(
                    "Function ext.datetime.before requires argument 2 (b) to be a time value, \
                     got {shown}"
                ),
            ),
        ]);
    }
}

#[test]
fn z_with_a_zone_is_the_instant_in_utc_seen_in_the_zone() {
    assert_values(&[
        (
            "dt.instant('2024-03-16T14:30:00Z[America/New_York]')",
            INSTANT,
        ),
        (
            "dt.components('2024-03-16T14:30:00Z[America/New_York]')",
            &format!("dt.components({INSTANT}, 'America/New_York')"),
        ),
    ]);
}

#[test]
fn an_offset_with_seconds_keeps_them_in_text() {
    assert_values(&[
        (
            "dt.to_string(0, 30000000)",
            "'1970-01-01T00:00:30+00:00:30'",
        ),
        (
            "dt.to_string(0, -30000000)",
            "'1969-12-31T23:59:30-00:00:30'",
        ),
        ("dt.instant(dt.to_string(0, 30000000))", "0"),
        // New York's local mean time before 1883: 4:56:02 behind UTC.
        (
            "dt.to_string(-5364662400000000, 'America/New_York')",
            "'1799-12-31T19:03:58-04:56:02'",
        ),
        (
            "dt.to_rfc9557(-5364662400000000, 'America/New_York')",
            "'1799-12-31T19:03:58-04:56:02[America/New_York]'",
        ),
        (
            "dt.instant(dt.to_string(-5364662400000000, 'America/New_York'))",
            "-5364662400000000",
        ),
        (
            "dt.instant(dt.to_rfc9557(-5364662400000000, 'America/New_York'))",
            "-5364662400000000",
        ),
    ]);
}

#[test]
fn a_time_of_day_alone_has_no_offset_or_zone() {
    assert_nulls(&[
        "dt.components('14:30-05:00')",
        "dt.components('14:30+01:00')",
        "dt.components('14:30Z')",
        "dt.components('T10:00[America/New_York]')",
    ]);
}

#[test]
fn text_naming_an_unknown_zone_or_placing_a_date_alone_is_no_time_value() {
    assert_nulls(&[
        "dt.components('2024-03-16T10:30:00-04:00[Mars/Olympus_Mons]')",
        "dt.components('2024-03-16Z')",
        "dt.components('2024-03-16[America/New_York]')",
    ]);
}

#[test]
fn a_zone_annotation_may_be_an_offset() {
    assert_values(&[
        (
            "dt.to_string('2024-03-16T14:30[+05:00]')",
            "'2024-03-16T14:30:00+05:00'",
        ),
        // A zero offset stays an offset, not the UTC time zone.
        (
            "dt.components('2024-03-16T14:30[+00:00]')",
            "{year: 2024, month: 3, day: 16, hour: 14, minute: 30, second: 0, microsecond: 0, \
             offset: 0}",
        ),
        (
            "dt.components('2024-03-16T14:30+05:00[+05:00]').offset",
            "18000000000",
        ),
    ]);
}
