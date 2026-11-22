//! `parse`'s arity and the types of its arguments.

use crate::common::{raises, value};

/// Assert `expression` raises exactly `message`.
fn assert_raises(expression: &str, message: &str) {
    assert_eq!(raises(expression), message, "{expression:?}");
}

#[test]
fn parse_takes_two_arguments() {
    for (call, count) in [
        ("argparse.parse()", 0),
        ("argparse.parse(['prog'])", 1),
        ("argparse.parse(['prog'], {}, {})", 3),
    ] {
        assert_raises(
            call,
            &format!(
                "Function ext.argparse.parse expects 2 arguments, but was called with {count}"
            ),
        );
    }
}

#[test]
fn args_is_an_array_of_strings() {
    let prefix = "Function ext.argparse.parse requires";
    for (args, got) in [
        ("null", "Null"),
        ("'prog'", "String"),
        ("x'00'", "Bytes"),
        ("{prog: 1}", "Map"),
        ("1", "Int"),
    ] {
        assert_raises(
            &format!("argparse.parse({args}, {{}})"),
            &format!("{prefix} Array as argument 1 (args), got {got}"),
        );
    }
    for (args, got) in [
        ("['prog', 1]", "Int"),
        ("['prog', null]", "Null"),
        ("['prog', x'00']", "Bytes"),
        ("['prog', ['a']]", "Array"),
    ] {
        assert_raises(
            &format!("argparse.parse({args}, {{}})"),
            &format!("{prefix} String as element 1 of argument 1 (args), got {got}"),
        );
    }
    assert_raises(
        "argparse.parse([1], {})",
        &format!("{prefix} String as element 0 of argument 1 (args), got Int"),
    );
}

#[test]
fn args_starts_with_the_programs_name() {
    assert_raises(
        "argparse.parse([], {})",
        "Function ext.argparse.parse requires argument 1 (args) to be non-empty, \
         starting with the program's name",
    );
    // The name alone is a command line with no arguments.
    assert_eq!(value("argparse.parse(['prog'], {})"), value("{value: {}}"));
}

#[test]
fn spec_is_a_map() {
    for (spec, got) in [
        ("null", "Null"),
        ("[]", "Array"),
        ("'spec'", "String"),
        ("1", "Int"),
    ] {
        assert_raises(
            &format!("argparse.parse(['prog'], {spec})"),
            &format!("Function ext.argparse.parse requires Map as argument 2 (spec), got {got}"),
        );
    }
}

#[test]
fn args_is_checked_before_spec() {
    assert_raises(
        "argparse.parse(null, null)",
        "Function ext.argparse.parse requires Array as argument 1 (args), got Null",
    );
    assert_raises(
        "argparse.parse([], {flags: {help: {}}})",
        "Function ext.argparse.parse requires argument 1 (args) to be non-empty, \
         starting with the program's name",
    );
}
