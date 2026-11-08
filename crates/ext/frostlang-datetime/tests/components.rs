//! `components` and `instant`: converting between instants and components, in
//! zones, at offsets, and in the local zone; and what a components Map may
//! hold.

use crate::common::{assert_nulls, assert_raises, assert_true, assert_values};

/// 2024-03-16T14:30:00Z, in microseconds since the Unix epoch.
const INSTANT: &str = "1710599400000000";

/// The first and last instants, which have a date at every offset.
const FIRST_INSTANT: &str = "-377705023201000000";
const LAST_INSTANT: &str = "253402207200999999";

/// Four hours, and five, in microseconds: New York's offsets west of UTC in
/// summer and in winter.
const EDT: &str = "-14400000000";
const EST: &str = "-18000000000";

// --- components of an instant ---

#[test]
fn an_instant_in_a_zone_has_every_key_of_a_zoned_datetime() {
    assert_values(&[
        (
            &format!("dt.components({INSTANT}, 'UTC')"),
            "{year: 2024, month: 3, day: 16, hour: 14, minute: 30, second: 0, microsecond: 0, \
             offset: 0, zone: 'UTC'}",
        ),
        (
            &format!("dt.components({INSTANT}, 'America/New_York')"),
            &format!(
                "{{year: 2024, month: 3, day: 16, hour: 10, minute: 30, second: 0, \
                 microsecond: 0, offset: {EDT}, zone: 'America/New_York'}}"
            ),
        ),
        (
            &format!("dt.components({INSTANT}, 'Asia/Tokyo')"),
            "{year: 2024, month: 3, day: 16, hour: 23, minute: 30, second: 0, microsecond: 0, \
             offset: 32400000000, zone: 'Asia/Tokyo'}",
        ),
        // The day before the epoch, and so before 1970.
        (
            "dt.components(0, 'America/New_York')",
            &format!(
                "{{year: 1969, month: 12, day: 31, hour: 19, minute: 0, second: 0, \
                 microsecond: 0, offset: {EST}, zone: 'America/New_York'}}"
            ),
        ),
    ]);
}

#[test]
fn an_instant_at_an_offset_has_no_zone() {
    assert_values(&[
        (
            &format!("dt.components({INSTANT}, {EST})"),
            &format!(
                "{{year: 2024, month: 3, day: 16, hour: 9, minute: 30, second: 0, \
                 microsecond: 0, offset: {EST}}}"
            ),
        ),
        // A zero offset is an offset too, not the UTC time zone.
        (
            &format!("dt.components({INSTANT}, 0)"),
            "{year: 2024, month: 3, day: 16, hour: 14, minute: 30, second: 0, microsecond: 0, \
             offset: 0}",
        ),
    ]);
}

#[test]
fn microseconds_carry_through_and_an_earlier_instant_floors() {
    assert_values(&[
        ("dt.components(1, 'UTC').microsecond", "1"),
        // One microsecond before the epoch is the last microsecond of 1969.
        (
            "dt.components(-1, 'UTC')",
            "{year: 1969, month: 12, day: 31, hour: 23, minute: 59, second: 59, \
             microsecond: 999999, offset: 0, zone: 'UTC'}",
        ),
    ]);
}

#[test]
fn without_a_zone_an_instant_is_seen_in_the_local_zone() {
    assert_true(&[&format!(
        r"
        def local = dt.local_zone()
        if local == null: not has(dt.components({INSTANT}), 'zone')
        else: dt.components({INSTANT}) == dt.components({INSTANT}, local)
        "
    )]);
}

#[test]
fn instants_stop_a_day_short_of_the_years_ends() {
    assert_values(&[
        (
            &format!("dt.components({FIRST_INSTANT}, 'UTC')"),
            "{year: -9999, month: 1, day: 2, hour: 1, minute: 59, second: 59, microsecond: 0, \
             offset: 0, zone: 'UTC'}",
        ),
        (
            &format!("dt.components({LAST_INSTANT}, 'UTC')"),
            "{year: 9999, month: 12, day: 30, hour: 22, minute: 0, second: 0, \
             microsecond: 999999, offset: 0, zone: 'UTC'}",
        ),
        // At the offsets furthest from UTC, the years' first and last days.
        (
            &format!("dt.components({FIRST_INSTANT}, -93599000000).day"),
            "1",
        ),
        (
            &format!("dt.components({LAST_INSTANT}, 93599000000).day"),
            "31",
        ),
    ]);
    let out_of_range = "Function ext.datetime.components requires argument 1 (value) to be an \
                        instant from -009999-01-02T01:59:59Z to 9999-12-30T22:00:00.999999Z, got";
    assert_raises(&[
        (
            "dt.components(253402207201000000, 'UTC')",
            &format!("{out_of_range} 253402207201000000"),
        ),
        (
            "dt.components(-377705023201000001, 'UTC')",
            &format!("{out_of_range} -377705023201000001"),
        ),
        (
            "dt.components(-9223372036854775807 - 1)",
            &format!("{out_of_range} -9223372036854775808"),
        ),
    ]);
}

#[test]
fn a_years_first_and_last_days_are_dates() {
    assert_values(&[
        (
            "dt.components({year: -9999, month: 1, day: 1, hour: 0})",
            "{year: -9999, month: 1, day: 1, hour: 0, minute: 0, second: 0, microsecond: 0}",
        ),
        (
            "dt.to_string({year: 9999, month: 12, day: 31})",
            "'9999-12-31'",
        ),
    ]);
}

// --- Converting and pinning ---

#[test]
fn a_zone_converts_a_value_on_the_timeline() {
    assert_values(&[
        // From an offset to a zone, and from one zone to another.
        (
            "dt.components({year: 2024, month: 3, day: 16, hour: 14, minute: 30, offset: 0}, \
             'Asia/Tokyo')",
            &format!("dt.components({INSTANT}, 'Asia/Tokyo')"),
        ),
        (
            &format!("dt.components(dt.components({INSTANT}, 'America/New_York'), 'Asia/Tokyo')"),
            &format!("dt.components({INSTANT}, 'Asia/Tokyo')"),
        ),
        // From a zone to an offset.
        (
            &format!("dt.components(dt.components({INSTANT}, 'Asia/Tokyo'), 0)"),
            &format!("dt.components({INSTANT}, 0)"),
        ),
    ]);
}

#[test]
fn a_zone_pins_a_wall_clock_value() {
    assert_values(&[
        (
            "dt.components({year: 2024, month: 3, day: 16, hour: 10, minute: 30}, \
             'America/New_York')",
            &format!("dt.components({INSTANT}, 'America/New_York')"),
        ),
        (
            &format!(
                "dt.components({{year: 2024, month: 3, day: 16, hour: 9, minute: 30}}, {EST})"
            ),
            &format!("dt.components({INSTANT}, {EST})"),
        ),
        // A date pins to the start of its day.
        (
            "dt.components({year: 2024, month: 3, day: 16}, 'Asia/Tokyo')",
            "{year: 2024, month: 3, day: 16, hour: 0, minute: 0, second: 0, microsecond: 0, \
             offset: 32400000000, zone: 'Asia/Tokyo'}",
        ),
    ]);
    assert_raises(&[(
        "dt.components({hour: 9}, 'UTC')",
        "Function ext.datetime.components requires a date, got a time of day",
    )]);
}

#[test]
fn without_a_zone_components_stay_as_they_are() {
    assert_values(&[
        (
            "dt.components({year: 2024, month: 3, day: 16})",
            "{year: 2024, month: 3, day: 16}",
        ),
        (
            "dt.components({hour: 9})",
            "{hour: 9, minute: 0, second: 0, microsecond: 0}",
        ),
        (
            "dt.components({year: 2024, month: 3, day: 16, hour: 9, minute: 5, second: 7, \
             microsecond: 8})",
            "{year: 2024, month: 3, day: 16, hour: 9, minute: 5, second: 7, microsecond: 8}",
        ),
        (
            "dt.components({year: 2024, month: 3, day: 16, hour: 9, offset: 3600000000})",
            "{year: 2024, month: 3, day: 16, hour: 9, minute: 0, second: 0, microsecond: 0, \
             offset: 3600000000}",
        ),
        // A zone without an offset gains the zone's.
        (
            "dt.components({year: 2024, month: 3, day: 16, hour: 9, zone: 'Europe/Paris'})",
            "{year: 2024, month: 3, day: 16, hour: 9, minute: 0, second: 0, microsecond: 0, \
             offset: 3600000000, zone: 'Europe/Paris'}",
        ),
    ]);
}

// --- instant ---

#[test]
fn instant_gives_the_instant_a_value_is() {
    assert_values(&[
        (&format!("dt.instant({INSTANT})"), INSTANT),
        (&format!("dt.instant({INSTANT}, 'Asia/Tokyo')"), INSTANT),
        (
            "dt.instant({year: 2024, month: 3, day: 16, hour: 14, minute: 30, offset: 0})",
            INSTANT,
        ),
        (
            &format!(
                "dt.instant({{year: 2024, month: 3, day: 16, hour: 10, minute: 30, \
                 offset: {EDT}, zone: 'America/New_York'}})"
            ),
            INSTANT,
        ),
        // A zone does not move a value already on the timeline.
        (
            "dt.instant({year: 2024, month: 3, day: 16, hour: 14, minute: 30, offset: 0}, \
             'Asia/Tokyo')",
            INSTANT,
        ),
    ]);
}

#[test]
fn instant_pins_a_wall_clock_value_to_the_zone_given_or_the_local_zone() {
    assert_values(&[
        (
            "dt.instant({year: 2024, month: 3, day: 16, hour: 10, minute: 30}, \
             'America/New_York')",
            INSTANT,
        ),
        (
            "dt.instant({year: 2024, month: 3, day: 16, hour: 14, minute: 30}, 0)",
            INSTANT,
        ),
        // A date is the start of its day.
        (
            "dt.instant({year: 1970, month: 1, day: 2}, 'UTC')",
            "86400000000",
        ),
    ]);
    assert_true(&[r"
        def wall_clock = {year: 2024, month: 3, day: 16, hour: 10, minute: 30}
        def local = dt.local_zone()
        if local == null: is_int(dt.instant(wall_clock))
        else: dt.instant(wall_clock) == dt.instant(wall_clock, local)
        "]);
    assert_raises(&[(
        "dt.instant({hour: 9})",
        "Function ext.datetime.instant requires a date, got a time of day",
    )]);
}

#[test]
fn components_and_instant_round_trip() {
    for zone in [
        "'UTC'",
        "'America/New_York'",
        "'Australia/Lord_Howe'",
        "-34200000000",
    ] {
        for instant in [
            "0",
            INSTANT,
            "-1",
            FIRST_INSTANT,
            LAST_INSTANT,
            "1710599400123456",
        ] {
            assert_values(&[(
                &format!("dt.instant(dt.components({instant}, {zone}))"),
                instant,
            )]);
        }
    }
}

// --- What a components Map holds ---

#[test]
fn a_time_of_day_needs_only_its_hour() {
    assert_values(&[
        (
            "dt.components({hour: 9, second: 30})",
            "{hour: 9, minute: 0, second: 30, microsecond: 0}",
        ),
        (
            "dt.components({hour: 23, minute: 59, second: 59, microsecond: 999999})",
            "{hour: 23, minute: 59, second: 59, microsecond: 999999}",
        ),
    ]);
}

#[test]
fn a_leap_day_is_a_date_only_in_a_leap_year() {
    assert_values(&[
        (
            "dt.components({year: 2024, month: 2, day: 29})",
            "{year: 2024, month: 2, day: 29}",
        ),
        (
            "dt.components({year: 2000, month: 2, day: 29})",
            "{year: 2000, month: 2, day: 29}",
        ),
    ]);
    let invalid = "Function ext.datetime.components requires argument 1 (value) to be valid \
                   components:";
    assert_raises(&[
        (
            "dt.components({year: 2023, month: 2, day: 29})",
            &format!("{invalid} `day` must be 1 to 28, got 29"),
        ),
        (
            "dt.components({year: 1900, month: 2, day: 29})",
            &format!("{invalid} `day` must be 1 to 28, got 29"),
        ),
    ]);
}

#[test]
fn each_field_must_be_in_its_range() {
    let invalid = "Function ext.datetime.instant requires argument 1 (value) to be valid \
                   components:";
    let date = "year: 2024, month: 3, day: 16";
    let cases = [
        (
            "year: 10000, month: 1, day: 1",
            "`year` must be -9999 to 9999, got 10000",
        ),
        (
            "year: -10000, month: 1, day: 1",
            "`year` must be -9999 to 9999, got -10000",
        ),
        (
            "year: 2024, month: 0, day: 1",
            "`month` must be 1 to 12, got 0",
        ),
        (
            "year: 2024, month: 13, day: 1",
            "`month` must be 1 to 12, got 13",
        ),
        (
            "year: 2024, month: 4, day: 31",
            "`day` must be 1 to 30, got 31",
        ),
        (
            "year: 2024, month: 1, day: 0",
            "`day` must be 1 to 31, got 0",
        ),
        (
            &format!("{date}, hour: 24"),
            "`hour` must be 0 to 23, got 24",
        ),
        (
            &format!("{date}, hour: -1"),
            "`hour` must be 0 to 23, got -1",
        ),
        (
            &format!("{date}, hour: 0, minute: 60"),
            "`minute` must be 0 to 59, got 60",
        ),
        // No leap seconds: Unix time has none.
        (
            &format!("{date}, hour: 0, second: 60"),
            "`second` must be 0 to 59, got 60",
        ),
        (
            &format!("{date}, hour: 0, microsecond: 1000000"),
            "`microsecond` must be 0 to 999999, got 1000000",
        ),
        (
            &format!("{date}, hour: 0, microsecond: -1"),
            "`microsecond` must be 0 to 999999, got -1",
        ),
    ];
    for (fields, problem) in cases {
        assert_raises(&[(
            &format!("dt.instant({{{fields}}}, 'UTC')"),
            &format!("{invalid} {problem}"),
        )]);
    }
}

#[test]
fn a_map_must_hold_a_whole_date_or_time_of_day() {
    let invalid = "Function ext.datetime.components requires argument 1 (value) to be valid \
                   components:";
    assert_raises(&[
        (
            "dt.components({})",
            &format!("{invalid} components need a date or a time of day"),
        ),
        (
            "dt.components({year: 2024, month: 3})",
            &format!("{invalid} a date needs `year`, `month`, and `day`"),
        ),
        (
            "dt.components({day: 16})",
            &format!("{invalid} a date needs `year`, `month`, and `day`"),
        ),
        (
            "dt.components({minute: 30})",
            &format!("{invalid} a time of day needs `hour`"),
        ),
        (
            "dt.components({year: 2024, month: 3, day: 16, offset: 0})",
            &format!("{invalid} `offset` and `zone` need both a date and a time of day"),
        ),
        (
            "dt.components({hour: 9, zone: 'UTC'})",
            &format!("{invalid} `offset` and `zone` need both a date and a time of day"),
        ),
        (
            "dt.components({offset: 0})",
            &format!("{invalid} components need a date or a time of day"),
        ),
    ]);
}

#[test]
fn a_map_holds_no_other_keys() {
    let invalid = "Function ext.datetime.components requires argument 1 (value) to be valid \
                   components:";
    let expected = "expected one of `year`, `month`, `day`, `hour`, `minute`, `second`, \
                    `microsecond`, `offset`, `zone` (at a key)";
    assert_raises(&[
        (
            "dt.components({years: 2024, month: 3, day: 16})",
            &format!("{invalid} unknown field `years`, {expected}"),
        ),
        // A weekday is not a component: `weekday` gives it.
        (
            "dt.components({year: 2024, month: 3, day: 16, weekday: 6})",
            &format!("{invalid} unknown field `weekday`, {expected}"),
        ),
    ]);
}

#[test]
fn each_field_has_its_type() {
    let invalid = "Function ext.datetime.components requires argument 1 (value) to be valid \
                   components:";
    assert_raises(&[
        (
            "dt.components({year: '2024', month: 3, day: 16})",
            &format!("{invalid} expected Int, got String (at `year`)"),
        ),
        (
            "dt.components({hour: 9.5})",
            &format!("{invalid} expected Int, got Float (at `hour`)"),
        ),
        (
            "dt.components({year: 2024, month: 3, day: 16, hour: 9, zone: 0})",
            &format!("{invalid} expected String, got Int (at `zone`)"),
        ),
    ]);
}

#[test]
fn an_offset_is_whole_seconds_under_26_hours() {
    let invalid = "Function ext.datetime.components requires argument 1 (value) to be valid \
                   components:";
    let datetime = "year: 2024, month: 3, day: 16, hour: 9";
    assert_values(&[(
        &format!("dt.components({{{datetime}, offset: 93599000000}}).offset"),
        "93599000000",
    )]);
    let cases = ["1", "93600000000", "-93600000000", "500000"];
    for offset in cases {
        assert_raises(&[(
            &format!("dt.components({{{datetime}, offset: {offset}}})"),
            &format!(
                "{invalid} `offset` must be Int microseconds of a whole number of seconds under \
                 26 hours, got {offset}"
            ),
        )]);
    }
}

#[test]
fn a_zone_must_be_known() {
    assert_raises(&[(
        "dt.components({year: 2024, month: 3, day: 16, hour: 9, zone: 'Mars/Olympus_Mons'})",
        "Function ext.datetime.components requires argument 1 (value) to be valid components: \
         `zone` must name a known time zone, got \"Mars/Olympus_Mons\"",
    )]);
}

#[test]
fn a_value_beyond_the_years_on_the_timeline_raises() {
    assert_raises(&[
        (
            "dt.instant({year: 9999, month: 12, day: 31, hour: 23, offset: -3600000000})",
            "Function ext.datetime.instant requires argument 1 (value) to be valid components: \
             it is beyond the years -9999 to 9999 on the timeline",
        ),
        (
            "dt.instant({year: 9999, month: 12, day: 31, hour: 23}, -3600000000)",
            "Function ext.datetime.instant has no result within the years -9999 to 9999",
        ),
    ]);
}

#[test]
fn text_that_is_no_time_value_gives_null() {
    assert_nulls(&[
        "dt.components('')",
        "dt.components('yesterday')",
        "dt.components('2024-02-30')",
        "dt.instant('2024-03-16T25:00:00Z')",
        "dt.instant('not a time', 'UTC')",
    ]);
}
