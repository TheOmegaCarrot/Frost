//! How `frost` ends a script that calls `fatal(value)`: what it prints to
//! standard error, and its exit status, for each form `value` may take.

use std::io;
use std::process::Command;

/// What `frost -e code` ended with: its exit status, standard output, and
/// standard error.
fn eval(code: &str) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_frost"))
        .args(["-e", code])
        .output()
        .expect("`frost` should run");
    (
        output.status.code().expect("`frost` exits with a status"),
        String::from_utf8(output.stdout).expect("the output is UTF-8"),
        String::from_utf8(output.stderr).expect("the error output is UTF-8"),
    )
}

/// Assert that `fatal(payload)`, called after printing a line, prints nothing
/// more to standard output, `stderr` to standard error, and exits with `code`.
#[track_caller]
fn assert_ends(payload: &str, stderr: &str, code: i32) {
    let source = format!(
        r"
print('before')
fatal({payload})
print('after')
"
    );
    assert_eq!(
        eval(&source),
        (code, "before\n".to_string(), stderr.to_string()),
        "fatal({payload})"
    );
}

#[test]
fn a_string_is_the_message_and_the_status_is_1() {
    assert_ends("'bad thing'", "bad thing\n", 1);
    // As given: no label, no trace, and a newline only where it lacks one.
    assert_ends("'two\\nlines'", "two\nlines\n", 1);
    assert_ends("'ends\\n'", "ends\n", 1);
    assert_ends("''", "\n", 1);
}

#[test]
fn a_map_of_message_and_code_chooses_both() {
    assert_ends("{message: 'usage', code: 2}", "usage\n", 2);
    assert_ends("{message: 'top', code: 255}", "top\n", 255);
    assert_ends("{message: 'low', code: 1}", "low\n", 1);
    assert_ends("{message: 'default'}", "default\n", 1);
}

#[test]
fn a_map_without_a_message_ends_silently() {
    assert_ends("{code: 3}", "", 3);
    assert_ends("{}", "", 1);
}

#[test]
fn any_other_value_is_printed_in_its_debug_form_with_status_1() {
    for (payload, debug_form) in [
        // A misspelled key shows, rather than ending silently.
        (
            "{mesage: 'x', code: 2}",
            r#"{ ["code"]: 2, ["mesage"]: "x" }"#,
        ),
        (
            "{message: 'x', extra: 1}",
            r#"{ ["extra"]: 1, ["message"]: "x" }"#,
        ),
        ("{message: 1}", r#"{ ["message"]: 1 }"#),
        ("{[1]: 'x'}", r#"{ [1]: "x" }"#),
        ("42", "42"),
        ("null", "null"),
        ("[1, 'two']", r#"[ 1, "two" ]"#),
    ] {
        assert_ends(payload, &format!("{debug_form}\n"), 1);
    }
}

#[test]
fn a_bad_code_is_printed_with_why_and_the_status_is_1() {
    let why = "fatal: `code` must be an Int from 1 to 255, so the exit status is 1";
    for (payload, debug_form) in [
        (
            "{message: 'x', code: 0}",
            r#"{ ["code"]: 0, ["message"]: "x" }"#,
        ),
        (
            "{message: 'x', code: 256}",
            r#"{ ["code"]: 256, ["message"]: "x" }"#,
        ),
        (
            "{message: 'x', code: -1}",
            r#"{ ["code"]: -1, ["message"]: "x" }"#,
        ),
        ("{code: 2.0}", r#"{ ["code"]: 2.0 }"#),
        ("{code: '2'}", r#"{ ["code"]: "2" }"#),
    ] {
        assert_ends(payload, &format!("{debug_form}\n{why}\n"), 1);
    }
}

#[test]
fn the_exit_status_stands_when_standard_error_cannot_be_written() {
    // A pipe whose read end is already closed: every write to it fails.
    let (reader, writer) = io::pipe().expect("a pipe can be made");
    drop(reader);
    let status = Command::new(env!("CARGO_BIN_EXE_frost"))
        .args(["-e", "fatal({message: 'unseen', code: 3})"])
        .stderr(writer)
        .status()
        .expect("`frost` should run");
    assert_eq!(status.code(), Some(3), "{status:?}");
}

#[test]
fn an_ordinary_error_keeps_its_label_and_trace() {
    let (code, _, stderr) = eval("error('plain')");
    assert_eq!(code, 1);
    assert!(stderr.starts_with("Error: plain\n"), "{stderr}");
    assert!(stderr.contains("in <main>"), "the trace: {stderr}");
}
