//! Every function's arity and argument types.

use crate::common::{assert_arity, assert_raises};

/// Each function taking a time value and an optional zone.
const VALUE_AND_ZONE: &[&str] = &[
    "components",
    "instant",
    "to_string",
    "to_rfc3339",
    "to_rfc9557",
    "weekday",
];

#[test]
fn each_function_has_its_arity() {
    for function in VALUE_AND_ZONE {
        assert_arity(function, "between 1 and 2", &[0, 3]);
    }
    assert_arity("format", "between 2 and 3", &[0, 1, 4]);
    assert_arity("parse", 2, &[0, 1, 3]);
    assert_arity("before", 2, &[0, 1, 3]);
    assert_arity("local_zone", 0, &[1]);
    assert_arity("parse_duration", 1, &[0, 2]);
    assert_arity("format_duration", 1, &[0, 2]);
}

#[test]
fn a_time_value_is_an_int_a_string_or_a_map() {
    let calls = VALUE_AND_ZONE
        .iter()
        .map(|function| (format!("dt.{function}(VALUE)"), *function, 1, "value"))
        .chain([
            ("dt.format(VALUE, '%Y')".to_string(), "format", 1, "value"),
            ("dt.before(VALUE, 0)".to_string(), "before", 1, "a"),
            ("dt.before(0, VALUE)".to_string(), "before", 2, "b"),
        ]);
    for (call, function, position, name) in calls {
        for (value, got) in [
            ("1.5", "Float"),
            ("null", "Null"),
            ("[1]", "Array"),
            ("x'00'", "Bytes"),
        ] {
            assert_raises(&[(
                &call.replace("VALUE", value),
                &format!(
                    "Function ext.datetime.{function} requires Int or String or Map as \
                     argument {position} ({name}), got {got}"
                ),
            )]);
        }
    }
}

#[test]
fn a_zone_is_a_string_or_an_int() {
    let calls = VALUE_AND_ZONE
        .iter()
        .map(|function| (format!("dt.{function}(0, ZONE)"), *function, 2))
        .chain([("dt.format(0, '%Y', ZONE)".to_string(), "format", 3)]);
    for (call, function, position) in calls {
        for (zone, got) in [("1.5", "Float"), ("null", "Null"), ("{}", "Map")] {
            assert_raises(&[(
                &call.replace("ZONE", zone),
                &format!(
                    "Function ext.datetime.{function} requires Int or String as argument \
                     {position} (zone), got {got}"
                ),
            )]);
        }
        for (zone, shown) in [
            ("'Mars/Olympus_Mons'", "\"Mars/Olympus_Mons\""),
            ("''", "\"\""),
            ("1", "1"),
            ("93600000000", "93600000000"),
        ] {
            assert_raises(&[(
                &call.replace("ZONE", zone),
                &format!(
                    "Function ext.datetime.{function} requires argument {position} (zone) to \
                     be a known time zone, or Int microseconds of a whole number of seconds \
                     under 26 hours, got {shown}"
                ),
            )]);
        }
    }
}

#[test]
fn text_arguments_are_strings() {
    assert_raises(&[
        (
            "dt.format(0, 5)",
            "Function ext.datetime.format requires String as argument 2 (pattern), got Int",
        ),
        (
            "dt.parse(5, '%Y')",
            "Function ext.datetime.parse requires String as argument 1 (text), got Int",
        ),
        (
            "dt.parse('5', x'00')",
            "Function ext.datetime.parse requires String as argument 2 (pattern), got Bytes",
        ),
        (
            "dt.parse_duration(5)",
            "Function ext.datetime.parse_duration requires String as argument 1 (text), got Int",
        ),
    ]);
}

#[test]
fn format_duration_takes_an_int() {
    for (value, got) in [("1.5", "Float"), ("'1h'", "String"), ("null", "Null")] {
        assert_raises(&[(
            &format!("dt.format_duration({value})"),
            &format!(
                "Function ext.datetime.format_duration requires Int as argument 1 (duration), \
                 got {got}"
            ),
        )]);
    }
}
