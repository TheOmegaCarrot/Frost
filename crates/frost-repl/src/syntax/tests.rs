//! White-box: the terminal input's reading of source as it is typed, which a
//! test cannot reach through a real terminal.

use crate::syntax::{Class, classify, is_unfinished, remove_line_continuations};

// --- Whether a segment is finished ---

#[test]
fn complete_input_is_finished() {
    for source in [
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
    ] {
        assert!(!is_unfinished(source), "{source:?} is finished");
    }
}

#[test]
fn an_open_bracket_is_unfinished() {
    for source in ["f(", "[1,", "{a: ", "$($ +", "((1)", "[{(", "f(\n1,\n"] {
        assert!(is_unfinished(source), "{source:?} is unfinished");
    }
}

#[test]
fn a_trailing_colon_or_arrow_is_unfinished() {
    for source in [
        "if x:",
        "if x: 1 else:",
        "fn x ->",
        "fn x ->  ",
        "if x: # a note",
    ] {
        assert!(is_unfinished(source), "{source:?} is unfinished");
    }
}

#[test]
fn an_unclosed_string_that_may_span_lines_is_unfinished() {
    for source in ["'abc", "\"abc", "'''abc", "\"\"\"abc\nmore", "f('abc"] {
        assert!(is_unfinished(source), "{source:?} is unfinished");
    }
}

#[test]
fn an_unclosed_string_that_may_not_span_lines_is_left_to_the_parser() {
    // Format, raw, and Bytes strings end on their line, so more lines cannot
    // finish them.
    for source in ["$'abc", "$\"abc", "R'(abc", "x'00"] {
        assert!(!is_unfinished(source), "{source:?} is finished");
    }
}

#[test]
fn a_trailing_backslash_is_unfinished() {
    for source in ["a \\", "a \\  ", "def a = 1 \\\ndef b = 2 \\"] {
        assert!(is_unfinished(source), "{source:?} is unfinished");
    }
}

#[test]
fn what_is_inside_a_string_or_comment_does_not_count() {
    for source in [
        "'('",
        "\"[{\"",
        "'ends in a colon:'",
        "\"->\"",
        "1 # (",
        "1 # [{ :",
        "1 # ->",
        "'\\\\'",
    ] {
        assert!(!is_unfinished(source), "{source:?} is finished");
    }
}

#[test]
fn a_backslash_before_the_end_of_its_line_is_left_to_the_parser() {
    for source in ["a \\ b", "a \\\nb"] {
        assert!(!is_unfinished(source), "{source:?} is finished");
    }
}

#[test]
fn a_bracket_closing_nothing_is_left_to_the_parser() {
    for source in [")", "f())", "]["] {
        assert!(!is_unfinished(source), "{source:?} is finished");
    }
}

// --- Line continuations ---

#[test]
fn a_line_continuation_is_removed_and_its_line_break_kept() {
    for (source, expected) in [
        ("a \\\nb", "a \nb"),
        ("a\\\nb\\\nc", "a\nb\nc"),
        ("def a = 1 \\  \ndef b = 2", "def a = 1   \ndef b = 2"),
        ("f(1, \\\n2)", "f(1, \n2)"),
    ] {
        assert_eq!(remove_line_continuations(source), expected, "{source:?}");
    }
}

#[test]
fn a_backslash_that_is_no_line_continuation_is_kept() {
    for source in [
        "no backslash at all",
        "a \\ b",
        "'a \\' b'",
        "'\\\\'",
        "x # a note \\\ny",
    ] {
        assert_eq!(remove_line_continuations(source), source, "{source:?}");
    }
}

// --- Highlighting ---

/// Each classified stretch of `source`, as its text and class.
fn classes(source: &str) -> Vec<(&str, Class)> {
    classify(source)
        .into_iter()
        .map(|(span, class)| (&source[span], class))
        .collect()
}

#[test]
fn classification_covers_the_source_exactly_in_order() {
    for source in [
        "",
        "   ",
        "def x = [1, 'two', {a: 3.5}] # note",
        "if x: f(\n  'open\n",
        ")] stray \\ ~",
        "$'fmt ${x}' x'00' R'(raw)'",
    ] {
        let mut at = 0;
        for (span, _) in classify(source) {
            assert_eq!(span.start, at, "{source:?}: a gap or overlap at {at}");
            assert!(
                span.end > span.start,
                "{source:?}: an empty stretch at {at}"
            );
            at = span.end;
        }
        assert_eq!(at, source.len(), "{source:?}: the end is not covered");
    }
}

#[test]
fn tokens_are_classified_by_kind() {
    use Class::*;
    assert_eq!(
        classes("def x = 1 and not true"),
        [
            ("def", Keyword),
            (" ", Plain),
            ("x", Plain),
            (" ", Plain),
            ("=", Plain),
            (" ", Plain),
            ("1", Number),
            (" ", Plain),
            ("and", Keyword),
            (" ", Plain),
            ("not", Keyword),
            (" ", Plain),
            ("true", Keyword),
        ]
    );
    for literal in ["2.5", "1e3", "42"] {
        assert_eq!(classes(literal), [(literal, Number)], "{literal:?}");
    }
    for literal in [
        "'s'",
        "\"s\"",
        "'''s'''",
        "$'f ${x}'",
        "$\"f\"",
        "R'(raw)'",
        "x'00ff'",
    ] {
        assert_eq!(classes(literal), [(literal, String)], "{literal:?}");
    }
}

#[test]
fn a_string_still_being_typed_is_a_string() {
    assert_eq!(classes("'unclosed"), [("'unclosed", Class::String)]);
    assert_eq!(
        classes("\"unclosed\nmore"),
        [("\"unclosed\nmore", Class::String)]
    );
}

#[test]
fn a_comment_runs_to_the_end_of_its_line() {
    use Class::*;
    assert_eq!(
        classes("1 # note\n2"),
        [
            ("1", Number),
            (" ", Plain),
            ("# note", Comment),
            ("\n", Plain),
            ("2", Number),
        ]
    );
    assert_eq!(classes("# only"), [("# only", Comment)]);
    // A `#` in a String is no comment.
    assert_eq!(classes("'#'"), [("'#'", String)]);
}

#[test]
fn brackets_are_classified_by_how_deeply_they_nest() {
    use Class::*;
    assert_eq!(
        classes("([{}])"),
        [
            ("(", Bracket(0)),
            ("[", Bracket(1)),
            ("{", Bracket(2)),
            ("}", Bracket(2)),
            ("]", Bracket(1)),
            (")", Bracket(0)),
        ]
    );
    assert_eq!(
        classes("()$()"),
        [
            ("(", Bracket(0)),
            (")", Bracket(0)),
            ("$(", Bracket(0)),
            (")", Bracket(0)),
        ]
    );
}

#[test]
fn a_bracket_closing_nothing_is_marked() {
    use Class::*;
    assert_eq!(classes(")("), [(")", UnmatchedBracket), ("(", Bracket(0))]);
}
