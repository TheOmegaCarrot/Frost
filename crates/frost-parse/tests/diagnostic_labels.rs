//! Where a parse error's labels land, and what they say.
//! Each label is checked as (the source text it covers, its text), primary first.

use frost_parse::parse_program;

/// The message and labels of the error `src` fails with, each label as the
/// source text it covers and its text.
fn diagnosis(src: &str) -> (String, Vec<(&str, String)>) {
    let err = parse_program("test.frst", src).expect_err(src);
    let labels = err
        .labels()
        .iter()
        .map(|label| (&src[label.span.start..label.span.end], label.text.clone()))
        .collect();
    (err.message().to_owned(), labels)
}

// -- A statement followed by more on its line --
// The finished statement is labeled, so the error shows what was read as complete.

#[test]
fn a_one_line_statement_is_labeled_whole() {
    let (message, labels) = diagnosis("let x = 5");
    assert_eq!(message, "expected a line break or `;`, but found `x`");
    assert_eq!(
        labels,
        [
            ("x", "unexpected".to_owned()),
            ("let", "this is a complete statement".to_owned()),
        ]
    );
}

#[test]
fn a_one_line_statement_after_others_is_labeled_whole() {
    let source = r"
        def a = 1
        print(a) print(a)
    ";
    let (_, labels) = diagnosis(source);
    assert_eq!(
        labels,
        [
            ("print", "unexpected".to_owned()),
            ("print(a)", "this is a complete statement".to_owned()),
        ]
    );
}

// A label spanning lines renders as a cluttered bracket, so a multiline
// statement is labeled at its last token.
#[test]
fn a_multiline_statement_is_labeled_at_its_end() {
    let source = r"
        def f = fn x -> {
            x
        } 5
    ";
    let (message, labels) = diagnosis(source);
    assert_eq!(message, "expected a line break or `;`, but found `5`");
    assert_eq!(
        labels,
        [
            ("5", "unexpected".to_owned()),
            ("}", "a complete statement ends here".to_owned()),
        ]
    );
}

// -- A list item followed by something other than `,` or the closer --
// A list that runs across lines gets its opener labeled with the kind of list.

#[test]
fn a_multiline_list_labels_its_opener() {
    // (source, the found token, the opener, the opener's label)
    let cases = [
        (
            r"
                [
                    1
                    2
                ]
            ",
            "2",
            "[",
            "in this Array literal",
        ),
        (
            r"
                {
                    a: 1
                    b: 2
                }
            ",
            "b",
            "{",
            "in this Map literal",
        ),
        (
            r"
                f(
                    1
                    2
                )
            ",
            "2",
            "(",
            "in this call",
        ),
        (
            r"
                x @ f(
                    1
                    2
                )
            ",
            "2",
            "(",
            "in this call",
        ),
        (
            r"
                match x {
                    1 => 2
                    3 => 4
                }
            ",
            "3",
            "{",
            "in this `match`",
        ),
        (
            r"
                match x {
                    [a
                    b] => 1
                }
            ",
            "b",
            "[",
            "in this Array pattern",
        ),
        (
            r"
                match x {
                    {a
                    b} => 1
                }
            ",
            "b",
            "{",
            "in this Map pattern",
        ),
        (
            r"
                def [
                    a
                    b
                ] = xs
            ",
            "b",
            "[",
            "in this Array pattern",
        ),
        (
            r"
                def {
                    a
                    b
                } = m
            ",
            "b",
            "{",
            "in this Map pattern",
        ),
        (
            r"
                defn f(
                    a
                    b
                ) -> a
            ",
            "b",
            "(",
            "in this parameter list",
        ),
    ];
    for (source, found, opener, label) in cases {
        let (_, labels) = diagnosis(source);
        assert_eq!(
            labels,
            [(found, "unexpected".to_owned()), (opener, label.to_owned()),],
            "{source:?}"
        );
    }
}

#[test]
fn a_multiline_list_names_the_found_token() {
    let source = r"
        def config = {
            name: 'frost',
            version: 1
            debug: true
        }
    ";
    let (message, labels) = diagnosis(source);
    assert_eq!(message, "expected `,` or `}`, but found `debug`");
    assert_eq!(
        labels,
        [
            ("debug", "unexpected".to_owned()),
            ("{", "in this Map literal".to_owned()),
        ]
    );
}

// On one line, the opener is in view beside the error; labeling it adds clutter.
#[test]
fn a_one_line_list_labels_only_the_found_token() {
    for source in [
        "[1 2]",
        "{a: 1 b: 2}",
        "f(1 2)",
        "match x { 1 => 2 3 => 4 }",
    ] {
        let (_, labels) = diagnosis(source);
        assert_eq!(labels.len(), 1, "{source:?}: {labels:?}");
        assert_eq!(labels[0].1, "unexpected", "{source:?}");
    }
}

// Only the innermost list is labeled, even when the lists enclosing it span lines too.
#[test]
fn only_the_innermost_list_is_labeled() {
    let source = r"
        f(
            [
                1
                2
            ]
        )
    ";
    let (_, labels) = diagnosis(source);
    assert_eq!(
        labels,
        [
            ("2", "unexpected".to_owned()),
            ("[", "in this Array literal".to_owned()),
        ]
    );
}

// -- Running out of input --
// The last token is labeled, since a label at the very end would not be drawn.
// A bracket still open is labeled too, since closing it is the likely fix.

#[test]
fn the_end_of_input_labels_the_last_token() {
    let (message, labels) = diagnosis("def g = fn x ->");
    assert_eq!(
        message,
        "expected an expression, but found the end of input"
    );
    assert_eq!(labels, [("->", "the input ends after this".to_owned())]);
}

#[test]
fn the_end_of_input_labels_an_unclosed_bracket() {
    let (message, labels) = diagnosis("def x = (1");
    assert_eq!(message, "expected `)`, but found the end of input");
    assert_eq!(
        labels,
        [
            ("1", "the input ends after this".to_owned()),
            ("(", "this `(` is not closed".to_owned()),
        ]
    );
}

// Trailing line breaks and comments are not tokens to label.
#[test]
fn the_last_token_is_found_past_trailing_lines() {
    let source = r"
        def x = [1, 2
        # a comment

    ";
    let (_, labels) = diagnosis(source);
    assert_eq!(
        labels,
        [
            ("2", "the input ends after this".to_owned()),
            ("[", "this `[` is not closed".to_owned()),
        ]
    );
}

// When the unclosed bracket is itself the last token, it gets the one label.
#[test]
fn an_unclosed_last_token_is_labeled_once() {
    let (message, labels) = diagnosis("f(");
    assert_eq!(
        message,
        "expected an expression, but found the end of input"
    );
    assert_eq!(labels, [("(", "this `(` is not closed".to_owned())]);
}

#[test]
fn the_innermost_unclosed_bracket_is_labeled() {
    let source = r"
        match x {
            1 => [1, 2
    ";
    let (_, labels) = diagnosis(source);
    assert_eq!(labels[1], ("[", "this `[` is not closed".to_owned()));
}

// Brackets closed before the end are passed over for the one still open.
#[test]
fn closed_brackets_are_not_labeled() {
    let (_, labels) = diagnosis("[(1), {a: 2}, f(3)[0], 4");
    assert_eq!(
        labels,
        [
            ("4", "the input ends after this".to_owned()),
            ("[", "this `[` is not closed".to_owned()),
        ]
    );
}

#[test]
fn every_kind_of_opener_is_labeled() {
    for (source, opener) in [
        ("f(1", "("),
        ("$($ * 2", "$("),
        ("[1", "["),
        ("{a: 1", "{"),
        ("do { 1", "{"),
    ] {
        let (_, labels) = diagnosis(source);
        assert_eq!(
            labels[1],
            (opener, format!("this `{opener}` is not closed")),
            "{source:?}"
        );
    }
}

// -- Running out of an interpolation --
// An interpolation's tokens end at its closing `}`, which is what the parse finds
// there, not the end of input.

#[test]
fn an_empty_interpolation_finds_its_closing_brace() {
    let source = "$'a ${}'";
    let (message, labels) = diagnosis(source);
    assert_eq!(message, "expected an expression, but found `}`");
    assert_eq!(
        labels,
        [
            ("}", "unexpected".to_owned()),
            (source, "in this format String".to_owned()),
        ]
    );
}

#[test]
fn an_unfinished_interpolation_labels_its_unclosed_bracket() {
    let source = "$'a ${f(}'";
    let (message, labels) = diagnosis(source);
    assert_eq!(message, "expected an expression, but found `}`");
    assert_eq!(
        labels,
        [
            ("}", "unexpected".to_owned()),
            ("(", "this `(` is not closed".to_owned()),
            (source, "in this format String".to_owned()),
        ]
    );
}
