//! White-box: how the terminal frontend colors source, which a test cannot see
//! through a real terminal.

use crate::highlight::{Class, classify};

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
