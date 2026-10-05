//! `complete_segment`: whether typed source is ready to run, and the segment it
//! makes when it is.

use frostlang_repl::complete_segment;

/// Assert that each of `sources` is complete, and makes itself unchanged.
fn assert_complete_as_is(sources: &[&str]) {
    for source in sources {
        assert_eq!(
            complete_segment(source).as_deref(),
            Some(*source),
            "{source:?} should be complete, as it is"
        );
    }
}

/// Assert that each of `sources` needs more lines.
fn assert_unfinished(sources: &[&str]) {
    for source in sources {
        assert_eq!(
            complete_segment(source),
            None,
            "{source:?} should need more lines"
        );
    }
}

// --- Whether source is complete ---

#[test]
fn complete_source_is_complete() {
    assert_complete_as_is(&[
        "",
        "1 + 2",
        "f(1)",
        "[1, [2, 3]]",
        "{a: {b: 1}}",
        "$(x)",
        "if x: 1 else: 2",
        "fn x -> x",
        "'a string'",
        "'''multiline\nstring'''",
        "f(\n1\n)",
        "def a = 1\ndef b = 2",
    ]);
}

#[test]
fn an_open_bracket_needs_more_lines() {
    assert_unfinished(&["f(", "[1,", "{a: ", "$($ +", "((1)", "[{(", "f(\n1,\n"]);
}

#[test]
fn a_trailing_colon_or_arrow_needs_more_lines() {
    assert_unfinished(&[
        "if x:",
        "if x: 1 else:",
        "fn x ->",
        "fn x ->  ",
        "if x: # a note",
    ]);
}

#[test]
fn an_open_string_that_may_span_lines_needs_more_lines() {
    assert_unfinished(&["'abc", "\"abc", "'''abc", "\"\"\"abc\nmore", "f('abc"]);
}

#[test]
fn an_open_string_that_may_not_span_lines_is_left_to_the_compiler() {
    // Format, raw, and Bytes strings end on their line, so more lines cannot
    // close them.
    assert_complete_as_is(&["$'abc", "$\"abc", "R'(abc", "x'00"]);
}

#[test]
fn a_trailing_line_continuation_needs_more_lines() {
    assert_unfinished(&["a \\", "a \\  ", "def a = 1 \\\ndef b = 2 \\"]);
}

#[test]
fn what_is_inside_a_string_or_comment_does_not_count() {
    assert_complete_as_is(&[
        "'('",
        "\"[{\"",
        "'ends in a colon:'",
        "\"->\"",
        "1 # (",
        "1 # [{ :",
        "1 # ->",
        "1 # \\",
        "'\\\\'",
    ]);
}

#[test]
fn a_backslash_before_the_end_of_its_line_is_left_to_the_compiler() {
    // Not even a comment may follow a line continuation.
    assert_complete_as_is(&["a \\ b", "a \\ # a note"]);
}

#[test]
fn a_bracket_closing_nothing_is_left_to_the_compiler() {
    assert_complete_as_is(&[")", "f())", "]["]);
}

// --- The segment made ---

#[test]
fn a_line_continuation_is_removed_and_its_line_break_kept() {
    for (source, expected) in [
        ("a \\\nb", "a \nb"),
        ("a\\\nb\\\nc", "a\nb\nc"),
        ("def a = 1 \\  \ndef b = 2", "def a = 1   \ndef b = 2"),
        ("f(1, \\\n2)", "f(1, \n2)"),
    ] {
        assert_eq!(
            complete_segment(source).as_deref(),
            Some(expected),
            "{source:?}"
        );
    }
}

#[test]
fn a_backslash_that_is_no_line_continuation_is_kept() {
    assert_complete_as_is(&[
        "no backslash at all",
        "a \\ b",
        "'a \\' b'",
        "'\\\\'",
        "x # a note \\\ny",
    ]);
}

// --- Metacommands ---

#[test]
fn a_metacommand_is_complete_at_the_end_of_its_line() {
    // What would leave Frost source unfinished does not count.
    assert_complete_as_is(&[
        ":help",
        ":disassemble if x:",
        ":disassemble fn x ->",
        ":ast f(1,",
        ":ast 'open",
        ":",
        ":undef x # not a comment",
    ]);
}

#[test]
fn a_metacommand_continues_after_a_line_continuation() {
    assert_unfinished(&[":disassemble f(1, \\", ":undef a \\\n b \\"]);
    assert_eq!(
        complete_segment(":disassemble f(1, \\\n2)").as_deref(),
        Some(":disassemble f(1, \n2)")
    );
}

#[test]
fn only_a_colon_first_makes_a_metacommand_line() {
    // Frost source rules apply to the rest, which these do not finish.
    assert_unfinished(&[" :ast f(", "x :", "f(\n:ast"]);
}
