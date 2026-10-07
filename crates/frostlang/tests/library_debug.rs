//! The Debug globals, from Frost source: `assert` and `debug_dump`.
//!
//! `assert(condition, error?)` raises when `condition` is falsy, and otherwise
//! returns it. A String `error` is raised as `Failed assertion: {error}`, any
//! other value as is. `debug_dump(value)` renders `value` as `to_string` does, except
//! that a String is quoted and escaped. Neither prints.
//!
//! The rendering itself is pinned by the runtime's own tests; these pin that
//! `debug_dump` is that rendering.
//!
//! The harness runs every case under every optimization permutation, so a call
//! over literals is checked both folded and at run time. Cases that capture their
//! input are never folded.

mod script;

use frostlang::Value;
use script::assertions::{Library, library_assertions};
use script::{Script, raises, run};

library_assertions!(Library::GLOBALS);

// --- assert ---

#[test]
fn a_passing_assert_returns_its_condition() {
    assert_values(&[
        ("assert(true)", "true"),
        ("assert(1)", "1"),
        ("assert([1, 'a'])", "[1, 'a']"),
        ("assert(true, 'unused')", "true"),
        // Only Null and False are falsy.
        ("assert(0)", "0"),
        ("assert('')", "''"),
        ("assert([])", "[]"),
        ("assert({})", "{}"),
    ]);
}

#[test]
fn a_failing_assert_raises() {
    assert_raises(&[
        ("assert(false)", "Failed assertion"),
        ("assert(null)", "Failed assertion"),
        (
            "assert(false, 'x must be positive')",
            "Failed assertion: x must be positive",
        ),
        ("assert(null, 'no value')", "Failed assertion: no value"),
        ("assert(false, '')", "Failed assertion: "),
    ]);
}

#[test]
fn a_failing_assert_is_catchable() {
    let source = r"
        def result = try_call(fn -> assert(1 > 2, 'order'))
        [result.ok, result.error]
    ";
    assert_values(&[(source, "[false, 'Failed assertion: order']")]);
}

#[test]
fn assert_tests_a_runtime_condition() {
    let assert_on = |condition: Value| {
        Script::new("assert(c, 'runtime')")
            .capture("c", condition)
            .outcome()
            .map(|finished| finished.tail)
    };
    assert_eq!(assert_on(Value::Int(7)), Ok(Value::Int(7)));
    assert_eq!(
        assert_on(Value::Bool(false)),
        Err("Failed assertion: runtime".to_string())
    );
}

#[test]
fn a_failing_assert_raises_a_non_string_error_as_is() {
    for error in [
        "42",
        "1.5",
        "true",
        "null",
        "x'6869'",
        "['a', 1]",
        "{code: 3, reason: 'bad'}",
    ] {
        let source = format!(
            r"
            def result = try_call(fn -> assert(false, {error}))
            [result.ok, result.error]
            "
        );
        assert_values(&[(&source, &format!("[false, {error}]"))]);
    }
}

#[test]
fn a_failing_assert_raises_what_error_would_for_a_non_string() {
    for error in ["42", "{code: 3}", "x'ff'"] {
        assert_eq!(
            raises(&format!("assert(false, {error})")),
            raises(&format!("error({error})")),
            "{error}"
        );
    }
}

#[test]
fn a_passing_assert_accepts_any_error() {
    assert_values(&[
        ("assert(1, 42)", "1"),
        ("assert(1, null)", "1"),
        ("assert(1, {code: 3})", "1"),
        ("assert(1, fn -> 0)", "1"),
    ]);
}

#[test]
fn assert_prints_nothing() {
    for source in [
        "assert(true)",
        "assert(1, 'unused')",
        "assert(false)",
        "assert(null, 'no value')",
        "assert(false, {code: 3})",
    ] {
        assert!(printed(source).is_empty(), "{source:?} performs no output");
    }
}

#[test]
fn assert_takes_one_or_two_arguments() {
    for (source, argc) in [("assert()", 0), ("assert(true, 'a', 'b')", 3)] {
        assert_eq!(
            raises(source),
            format!(
                "Function assert expects between 1 and 2 arguments, but was called with {argc}"
            ),
            "{source:?}"
        );
    }
}

// --- debug_dump ---

#[test]
fn debug_dump_quotes_and_escapes_a_string() {
    assert_values(&[
        ("debug_dump('hi')", r#"'"hi"'"#),
        ("debug_dump('')", r#"'""'"#),
        (r"debug_dump('a\nb')", r#"'"a\\nb"'"#),
        (r#"debug_dump('say "hi"')"#, r#"'"say \\"hi\\""'"#),
    ]);
}

#[test]
fn debug_dump_renders_any_other_value_as_to_string_does() {
    for value in [
        "null",
        "42",
        "-1.5",
        "true",
        "x'6869'",
        "[1, 'a', [null]]",
        "{k: 'v', [1]: [2]}",
        "fn x -> x",
        "print",
    ] {
        assert_eq!(
            run(&format!("debug_dump({value})")),
            run(&format!("to_string({value})")),
            "{value}"
        );
    }
}

#[test]
fn debug_dump_renders_a_runtime_value() {
    let dumped = Script::new("debug_dump(v)")
        .capture("v", Value::from("tab\there"))
        .run();
    assert_eq!(dumped, Value::from(r#""tab\there""#));
}

#[test]
fn debug_dump_prints_nothing() {
    let script = Script::new("debug_dump('quiet')");
    assert!(script.printed().is_empty(), "debug_dump performs no output");
}

#[test]
fn debug_dump_takes_exactly_one_argument() {
    for (source, argc) in [("debug_dump()", 0), ("debug_dump(1, 2)", 2)] {
        assert_eq!(
            raises(source),
            format!("Function debug_dump expects 1 arguments, but was called with {argc}"),
            "{source:?}"
        );
    }
}
