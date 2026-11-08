//! Reading the calendar: `weekday`, `before`, and `local_zone`.

use crate::common::{assert_raises, assert_true, assert_values};

// --- weekday ---

#[test]
fn weekday_numbers_monday_1_to_sunday_7() {
    // 2024-03-11 was a Monday.
    for (day, weekday) in (11..=17).zip(1..=7) {
        assert_values(&[(
            &format!("dt.weekday({{year: 2024, month: 3, day: {day}}})"),
            &weekday.to_string(),
        )]);
    }
    assert_values(&[
        ("dt.weekday({year: 1970, month: 1, day: 1})", "4"),
        ("dt.weekday({year: 2024, month: 3, day: 16, hour: 23})", "6"),
    ]);
}

#[test]
fn weekday_of_an_instant_is_of_its_date_in_the_zone() {
    // 2024-03-16T20:00:00Z: a Saturday in UTC, already Sunday in Tokyo.
    let instant = "1710619200000000";
    assert_values(&[
        (&format!("dt.weekday({instant}, 'UTC')"), "6"),
        (&format!("dt.weekday({instant}, 'Asia/Tokyo')"), "7"),
        (&format!("dt.weekday({instant}, 32400000000)"), "7"),
    ]);
    assert_true(&[&format!(
        r"
        def local = dt.local_zone()
        local == null or dt.weekday({instant}) == dt.weekday({instant}, local)
        "
    )]);
}

#[test]
fn weekday_of_a_wall_clock_value_is_of_its_own_date_in_any_zone() {
    // Samoa skipped 2011-12-30, a Friday, crossing the date line.
    assert_values(&[
        ("dt.weekday('2011-12-30')", "5"),
        ("dt.weekday('2011-12-30', 'Pacific/Apia')", "5"),
        ("dt.weekday('2011-12-30T12:00', 'Pacific/Apia')", "5"),
        ("dt.weekday('2024-03-16', 'Asia/Tokyo')", "6"),
    ]);
}

#[test]
fn weekday_of_a_time_of_day_alone_raises() {
    assert_raises(&[(
        "dt.weekday({hour: 9})",
        "Function ext.datetime.weekday requires a date, got a time of day",
    )]);
}

// --- before ---

#[test]
fn before_orders_instants() {
    assert_values(&[
        ("dt.before(1, 2)", "true"),
        ("dt.before(2, 1)", "false"),
        ("dt.before(1, 1)", "false"),
        ("dt.before(-1, 0)", "true"),
    ]);
}

#[test]
fn before_orders_values_on_the_timeline_as_instants() {
    assert_values(&[
        // 09:30 in New York is 14:30 UTC, after 12:00 UTC.
        (
            "dt.before('2024-03-16T12:00:00Z', '2024-03-16T09:30:00-05:00')",
            "true",
        ),
        (
            "dt.before('2024-03-16T09:30:00-05:00', '2024-03-16T12:00:00Z')",
            "false",
        ),
        ("dt.before(dt.components(5, 'Asia/Tokyo'), 6)", "true"),
        // The two times it was 01:30 in New York, by their offsets.
        (
            "dt.before('2024-11-03T01:30-04:00[America/New_York]', \
             '2024-11-03T01:30-05:00[America/New_York]')",
            "true",
        ),
    ]);
}

#[test]
fn before_orders_wall_clock_values_of_one_kind() {
    assert_values(&[
        ("dt.before('2023-12-31', '2024-01-01')", "true"),
        ("dt.before('2024-01-01', '2023-12-31')", "false"),
        // ISO 8601 writes a year before 1 with six digits.
        ("dt.before('-000001-01-01', '0001-01-01')", "true"),
        ("dt.before('09:00', '10:00')", "true"),
        ("dt.before('10:00:00.000001', '10:00')", "false"),
        ("dt.before('2024-01-01T23:59', '2024-01-02T00:00')", "true"),
    ]);
}

#[test]
fn before_raises_for_a_wall_clock_value_against_one_on_the_timeline() {
    let cannot = "Function ext.datetime.before cannot order";
    assert_raises(&[
        (
            "dt.before({year: 2024, month: 3, day: 16, hour: 12}, 0)",
            &format!("{cannot} a wall-clock datetime against an instant"),
        ),
        (
            "dt.before('2024-03-16T14:30:00Z', '2024-03-16')",
            &format!("{cannot} a datetime at an offset against a date"),
        ),
        (
            "dt.before('09:00', '2024-03-16T10:30[America/New_York]')",
            &format!("{cannot} a time of day against a zoned datetime"),
        ),
    ]);
    // Pinned first, it orders.
    assert_values(&[(
        "dt.before(dt.instant({year: 2024, month: 3, day: 16, hour: 12}, 'UTC'), \
         1710599400000000)",
        "true",
    )]);
}

#[test]
fn before_orders_consistently_around_a_skipped_time() {
    // 02:30 on 2024-03-10 was skipped in New York. Two wall-clock times order
    // by their clocks; pinned, the skipped one moves past 03:10, so ordering
    // the pins against an instant as well would be no consistent order.
    assert_values(&[("dt.before('2024-03-10T02:30', '2024-03-10T03:10')", "true")]);
    assert_raises(&[(
        "dt.before('2024-03-10T03:10', '2024-03-10T03:20-04:00')",
        "Function ext.datetime.before cannot order a wall-clock datetime against a datetime \
         at an offset",
    )]);
}

#[test]
fn before_raises_for_values_that_do_not_order() {
    assert_raises(&[
        (
            "dt.before('2024-01-01', '09:00')",
            "Function ext.datetime.before cannot order a date against a time of day",
        ),
        (
            "dt.before('2024-01-01', '2024-01-01T09:00')",
            "Function ext.datetime.before cannot order a date against a wall-clock datetime",
        ),
    ]);
}

#[test]
fn before_sorts_values() {
    assert_values(&[
        (
            "sorted(['2024-03-16', '2023-01-01', '2024-01-05'], dt.before)",
            "['2023-01-01', '2024-01-05', '2024-03-16']",
        ),
        (
            "sorted(['2024-03-16T12:00:00+09:00', '2024-03-16T00:00:00Z', 1], dt.before)",
            "[1, '2024-03-16T00:00:00Z', '2024-03-16T12:00:00+09:00']",
        ),
    ]);
}

// --- local_zone ---

#[test]
fn local_zone_names_a_known_zone_or_is_null() {
    assert_true(&[r"
        def local = dt.local_zone()
        local == null or dt.components(0, local).zone == local
        "]);
}
