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
