//! Daylight saving time: wall-clock times skipped as clocks go forward, those
//! repeated as they go back, and offsets a zone does not have.
//!
//! In New York in 2024, clocks went forward at 02:00 on March 10, skipping to
//! 03:00, and back at 02:00 on November 3, repeating 01:00 to 02:00.

use crate::common::{assert_nulls, assert_raises, assert_values};

/// Four hours, and five, in microseconds: New York's offsets west of UTC in
/// summer and in winter.
const EDT: &str = "-14400000000";
const EST: &str = "-18000000000";

/// 2024-03-10T07:30:00Z: 03:30 in New York, just after clocks went forward.
const AFTER_GAP: &str = "1710055800000000";

/// 2024-11-03T05:30:00Z and an hour later: the two times it was 01:30 in New
/// York.
const FIRST_0130: &str = "1730611800000000";
const SECOND_0130: &str = "1730615400000000";

#[test]
fn a_skipped_time_moves_forward_by_the_gap() {
    assert_values(&[
        (
            "dt.instant({year: 2024, month: 3, day: 10, hour: 2, minute: 30}, \
             'America/New_York')",
            AFTER_GAP,
        ),
        (
            "dt.components({year: 2024, month: 3, day: 10, hour: 2, minute: 30}, \
             'America/New_York')",
            &format!(
                "{{year: 2024, month: 3, day: 10, hour: 3, minute: 30, second: 0, \
                 microsecond: 0, offset: {EDT}, zone: 'America/New_York'}}"
            ),
        ),
    ]);
    // Neither offset is the zone's at a skipped time.
    for offset in [EST, EDT] {
        assert_raises(&[(
            &format!(
                "dt.instant({{year: 2024, month: 3, day: 10, hour: 2, minute: 30, \
                 offset: {offset}, zone: 'America/New_York'}})"
            ),
            &format!(
                "Function ext.datetime.instant requires argument 1 (value) to be valid \
                 components: `offset` must be one `zone` has at that time, got {offset}"
            ),
        )]);
    }
}

#[test]
fn an_offset_alone_follows_no_zone_rules() {
    assert_values(&[(
        &format!(
            "dt.instant({{year: 2024, month: 3, day: 10, hour: 2, minute: 30, offset: {EDT}}})"
        ),
        "1710052200000000",
    )]);
}

#[test]
fn a_skipped_midnight_starts_the_day_at_the_first_time_there_is() {
    // In Sao Paulo, clocks went forward at midnight on 2018-11-04, so that day
    // began at 01:00, 03:00 UTC.
    assert_values(&[
        (
            "dt.instant({year: 2018, month: 11, day: 4}, 'America/Sao_Paulo')",
            "1541300400000000",
        ),
        (
            "dt.components({year: 2018, month: 11, day: 4}, 'America/Sao_Paulo').hour",
            "1",
        ),
    ]);
}

#[test]
fn a_repeated_time_takes_the_earlier_unless_an_offset_picks() {
    assert_values(&[
        (
            "dt.instant({year: 2024, month: 11, day: 3, hour: 1, minute: 30}, \
             'America/New_York')",
            FIRST_0130,
        ),
        (
            &format!(
                "dt.instant({{year: 2024, month: 11, day: 3, hour: 1, minute: 30, \
                 offset: {EDT}, zone: 'America/New_York'}})"
            ),
            FIRST_0130,
        ),
        (
            &format!(
                "dt.instant({{year: 2024, month: 11, day: 3, hour: 1, minute: 30, \
                 offset: {EST}, zone: 'America/New_York'}})"
            ),
            SECOND_0130,
        ),
        (
            "dt.instant('2024-11-03T01:30:00-05:00[America/New_York]')",
            SECOND_0130,
        ),
    ]);
}

#[test]
fn components_round_trip_through_a_repeated_time() {
    for instant in [FIRST_0130, SECOND_0130] {
        assert_values(&[
            (
                &format!("dt.instant(dt.components({instant}, 'America/New_York'))"),
                instant,
            ),
            (
                &format!("dt.instant(dt.to_string({instant}, 'America/New_York'))"),
                instant,
            ),
        ]);
    }
    assert_values(&[
        (
            &format!("dt.components({FIRST_0130}, 'America/New_York').offset"),
            EDT,
        ),
        (
            &format!("dt.components({SECOND_0130}, 'America/New_York').offset"),
            EST,
        ),
    ]);
}

#[test]
fn a_map_with_an_offset_the_zone_does_not_have_raises() {
    // January's offset, carried into July by changing the month.
    let january = "dt.components({year: 2024, month: 1, day: 15, hour: 12}, 'America/New_York')";
    for function in ["instant", "components", "to_string", "weekday"] {
        assert_raises(&[(
            &format!("dt.{function}({january} + {{month: 7}})"),
            &format!(
                "Function ext.datetime.{function} requires argument 1 (value) to be valid \
                 components: `offset` must be one `zone` has at that time, got {EST}"
            ),
        )]);
    }
}

#[test]
fn dropping_a_stale_offset_lets_the_zone_give_it() {
    let july = "dissoc(dt.components({year: 2024, month: 1, day: 15, hour: 12}, \
                'America/New_York') + {month: 7}, 'offset')";
    assert_values(&[
        (&format!("dt.instant({july})"), "1721059200000000"),
        (&format!("dt.components({july}).offset"), EDT),
    ]);
}

#[test]
fn text_with_an_offset_the_zone_does_not_have_contradicts_itself() {
    assert_nulls(&[
        "dt.instant('2024-07-15T12:00:00-05:00[America/New_York]')",
        "dt.components('2024-03-16T14:30:00+05:00[+06:00]')",
        // At a skipped time, the zone has neither offset.
        "dt.instant('2024-03-10T02:30:00-05:00[America/New_York]')",
        "dt.parse('2024-07-15 12:00 -0500 America/New_York', '%Y-%m-%d %H:%M %z %Q')",
    ]);
}

#[test]
fn text_may_round_to_the_minute_an_offset_with_seconds() {
    // Monrovia was 44 minutes 30 seconds behind UTC until 1972; RFC 3339 text,
    // as most tools write it, rounds that to 45 minutes.
    assert_values(&[
        (
            "dt.instant('1968-02-01T23:15:00-00:45[Africa/Monrovia]')",
            "dt.instant('1968-02-01T23:15:00-00:44:30[Africa/Monrovia]')",
        ),
        (
            "dt.components('1968-02-01T23:15:00-00:45[Africa/Monrovia]').offset",
            "-2670000000",
        ),
        (
            "dt.instant('1799-12-31T19:03:58-04:56[America/New_York]')",
            "-5364662400000000",
        ),
    ]);
    // Seconds the text does write must be the zone's.
    assert_nulls(&["dt.instant('1968-02-01T23:15:00-00:44:31[Africa/Monrovia]')"]);
}

#[test]
fn a_rounded_offset_picks_a_repeated_time_but_the_zones_own_offset_places_it() {
    // New York went from local mean time, 4:56:02 behind UTC, to 5 hours behind
    // at noon on 1883-11-18, so 12:01 then occurred twice. The rounded `-04:56`
    // picks the first, which is still 12:01 on the wall clock.
    assert_values(&[(
        "dt.to_rfc9557('1883-11-18T12:01:00-04:56[America/New_York]')",
        "'1883-11-18T12:01:00-04:56:02[America/New_York]'",
    )]);
}

#[test]
fn an_exact_offset_wins_over_one_it_rounds() {
    // Denver went from local mean time, 6:59:56 behind UTC, to 7 hours behind
    // at noon on 1883-11-18, so `-07:00` names the second offset exactly and
    // rounds the first. 12:00:02 at -07:00 is the second 12:00:02.
    let second = "dt.instant('1883-11-18T12:00:02-07:00')";
    assert_values(&[
        (
            &format!("dt.instant(dt.to_rfc9557({second}, 'America/Denver'))"),
            second,
        ),
        (
            &format!("dt.instant(dt.components(dt.components({second}, 'America/Denver')))"),
            second,
        ),
        (
            "dt.instant('1883-11-18T12:00:02-07:00[America/Denver]')",
            second,
        ),
    ]);
}

#[test]
fn a_map_offset_must_be_exact() {
    // Text may round an offset with seconds, but a Map holds the exact one.
    assert_values(&[(
        "dt.components({year: 1968, month: 2, day: 1, hour: 23, minute: 15, \
         offset: -2670000000, zone: 'Africa/Monrovia'}).offset",
        "-2670000000",
    )]);
    assert_raises(&[(
        "dt.components({year: 1968, month: 2, day: 1, hour: 23, minute: 15, \
         offset: -2700000000, zone: 'Africa/Monrovia'})",
        "Function ext.datetime.components requires argument 1 (value) to be valid components: \
         `offset` must be one `zone` has at that time, got -2700000000",
    )]);
}

#[test]
fn a_day_across_a_change_is_not_24_hours() {
    assert_values(&[
        (
            r"
            def ny = 'America/New_York'
            dt.instant({year: 2024, month: 3, day: 11}, ny) - dt.instant({year: 2024, month: 3, day: 10}, ny)
            ",
            "82800000000",
        ),
        (
            r"
            def ny = 'America/New_York'
            dt.instant({year: 2024, month: 11, day: 4}, ny) - dt.instant({year: 2024, month: 11, day: 3}, ny)
            ",
            "90000000000",
        ),
    ]);
}
