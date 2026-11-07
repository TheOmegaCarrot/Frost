//! `std.time`, from Frost source.
//!
//! Each case runs with only `std.time` installed, bound as `time`. A duration
//! is Int microseconds: the constructors make one from a count of a unit, and
//! the `as_*` inverses give one back as a Float count of a unit.

use crate::script;

use frostlang::stdlib::StdlibConfig;
use frostlang::{ImporterBuilder, Stdlib, Value, stdlib};
use script::Script;
use script::assertions::{Library, library_assertions};

library_assertions!(Library::module(stdlib::time, "time"));

/// Each unit's name and its length in microseconds.
const UNITS: &[(&str, i64)] = &[
    ("milliseconds", 1_000),
    ("seconds", 1_000_000),
    ("minutes", 60_000_000),
    ("hours", 3_600_000_000),
    ("days", 86_400_000_000),
    ("weeks", 604_800_000_000),
];

// --- The module ---

#[test]
fn the_module_holds_its_functions() {
    assert_values(&[(
        "sorted(keys(time))",
        "['as_days', 'as_hours', 'as_milliseconds', 'as_minutes', 'as_seconds', 'as_weeks', \
         'days', 'hours', 'milliseconds', 'minutes', 'seconds', 'weeks']",
    )]);
}

#[test]
fn the_module_is_contained() {
    let stdlib = Stdlib::contained(StdlibConfig::default());
    let contained = ImporterBuilder::new().with_stdlib(stdlib).build();
    let result = Script::new("import('std.time').seconds(1)")
        .importer(contained)
        .run();
    assert_eq!(result, Value::Int(1_000_000));
}

// --- Constructors ---

#[test]
fn each_constructor_counts_its_unit_in_microseconds() {
    for &(name, micros) in UNITS {
        assert_values(&[
            (&format!("time.{name}(1)"), &micros.to_string()),
            (&format!("time.{name}(3)"), &(3 * micros).to_string()),
            (&format!("time.{name}(-2)"), &(-2 * micros).to_string()),
            (&format!("time.{name}(0)"), "0"),
        ]);
    }
}

#[test]
fn a_float_count_rounds_to_the_nearest_microsecond() {
    assert_values(&[
        ("time.seconds(0.25)", "250000"),
        ("time.days(0.5)", "43200000000"),
        ("time.seconds(2.0)", "2000000"),
        // 0.1 is not exact in binary; the product still lands on a whole microsecond.
        ("time.seconds(0.1)", "100000"),
        ("time.milliseconds(0.0004)", "0"),
        ("time.milliseconds(0.0006)", "1"),
        ("time.milliseconds(-0.0004)", "0"),
        ("time.milliseconds(-0.0006)", "-1"),
        ("time.milliseconds(-0.0)", "0"),
        // 0.0625 milliseconds is exactly 62.5 microseconds: a half rounds away
        // from zero, as `math.round` rounds it.
        ("time.milliseconds(0.0625)", "63"),
        ("time.milliseconds(-0.0625)", "-63"),
    ]);
    for &(name, _) in UNITS {
        assert_values(&[(&format!("is_int(time.{name}(1.5))"), "true")]);
    }
}

#[test]
fn a_duration_past_the_int_range_is_an_error() {
    // The largest count of milliseconds whose microseconds fit an Int, and one past it.
    assert_values(&[("time.milliseconds(9223372036854775)", "9223372036854775000")]);
    let cases = [
        ("milliseconds", "9223372036854776"),
        ("milliseconds", "-9223372036854776"),
        ("weeks", "9223372036854775807"),
        ("seconds", "1e300"),
        ("days", "-1e300"),
        ("milliseconds", "1e16"),
    ];
    for (name, count) in cases {
        let shown = script(&format!("to_string({count})")).run();
        let shown = shown.as_str().expect("a String");
        assert_raises(&[(
            &format!("time.{name}({count})"),
            &format!("Function std.time.{name} has no Int result for {shown}"),
        )]);
    }
}

#[test]
fn the_constructors_require_a_number() {
    for &(name, _) in UNITS {
        for (arg, got) in [("'1'", "String"), ("null", "Null"), ("[1]", "Array")] {
            assert_raises(&[(
                &format!("time.{name}({arg})"),
                &format!("Function std.time.{name} requires Numeric as argument 1, got {got}"),
            )]);
        }
        assert_arity(name, "1", &[0, 2]);
    }
}

// --- Inverses ---

#[test]
fn each_inverse_counts_microseconds_in_its_unit_as_a_float() {
    for &(name, micros) in UNITS {
        assert_values(&[
            (&format!("time.as_{name}({micros})"), "1.0"),
            (&format!("time.as_{name}({})", 3 * micros / 2), "1.5"),
            (&format!("time.as_{name}({})", -micros / 2), "-0.5"),
            (&format!("time.as_{name}(0)"), "0.0"),
        ]);
    }
    assert_values(&[("time.as_milliseconds(1)", "0.001")]);
}

#[test]
fn an_inverse_undoes_its_constructor() {
    for &(name, _) in UNITS {
        assert_values(&[
            (&format!("time.as_{name}(time.{name}(2.5))"), "2.5"),
            (&format!("time.as_{name}(time.{name}(-7))"), "-7.0"),
        ]);
    }
}

#[test]
fn the_inverses_require_an_int() {
    for &(name, _) in UNITS {
        let inverse = format!("as_{name}");
        for (arg, got) in [("1.5", "Float"), ("'1'", "String"), ("null", "Null")] {
            assert_raises(&[(
                &format!("time.{inverse}({arg})"),
                &format!("Function std.time.{inverse} requires Int as argument 1, got {got}"),
            )]);
        }
        assert_arity(&inverse, "1", &[0, 2]);
    }
}
