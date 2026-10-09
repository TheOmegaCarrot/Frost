//! Where a parse error's labels land, and what they say.
//! Each label is checked as (the source text it covers, its text), primary first.

use frostlang_parse::parse_program;

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

// A token touching the statement's last name or literal makes one malformed token with
// it, not a statement after a complete one, so only the token is labeled.
#[test]
fn a_token_touching_the_statement_is_labeled_alone() {
    // (source, the found token)
    let cases = [
        ("def x = 0b101", "b101"),
        ("def x = 1__0", "__0"),
        ("def a = 'it''s'", "'s'"),
        (r#"def x = r"abc""#, r#""abc""#),
    ];
    for (source, found) in cases {
        let (_, labels) = diagnosis(source);
        assert_eq!(labels, [(found, "unexpected".to_owned())], "{source:?}");
    }
}

// After a closer, a touching token starts the next statement, so the complete one is
// still labeled.
#[test]
fn a_token_touching_a_closer_still_labels_the_statement() {
    // (source, the found token, the statement)
    let cases = [
        (r#"print("a")print("b")"#, "print", r#"print("a")"#),
        ("def x = [1, 2]x", "x", "def x = [1, 2]"),
    ];
    for (source, found, statement) in cases {
        let (_, labels) = diagnosis(source);
        assert_eq!(
            labels,
            [
                (found, "unexpected".to_owned()),
                (statement, "this is a complete statement".to_owned()),
            ],
            "{source:?}"
        );
    }
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

// An entry that cannot start labels its list's opener as an item without its comma does.
#[test]
fn a_multiline_entry_start_labels_its_opener() {
    // (source, the found token, the source from the innermost `{` on, the `{`'s label)
    let cases = [
        (
            r"
                def y = match x {
                    1 => {a: 1,
                    3 => 4
                }
            ",
            "3",
            "{a: 1,",
            "in this Map literal",
        ),
        (
            r"
                def a = {a: 1,
                ...b}
            ",
            "...",
            "{a: 1,",
            "in this Map literal",
        ),
        (
            r"
                def {
                    a,
                    1
                } = m
            ",
            "1",
            "{",
            "in this Map pattern",
        ),
        (
            r"
                match m {
                    {a,
                    1} => a
                }
            ",
            "1",
            "{a,",
            "in this Map pattern",
        ),
    ];
    for (source, found, from_opener, label) in cases {
        let (_, labels) = diagnosis(source);
        assert_eq!(
            labels,
            [(found, "unexpected".to_owned()), ("{", label.to_owned())],
            "{source:?}"
        );
        let err = parse_program("test.frst", source).expect_err(source);
        assert_eq!(
            Some(err.labels()[1].span.start),
            source.find(from_opener),
            "{source:?}: the label is on another `{{`"
        );
    }
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
    let (message, labels) = diagnosis("$'a ${}'");
    assert_eq!(message, "expected an expression, but found `}`");
    assert_eq!(
        labels,
        [
            ("}", "unexpected".to_owned()),
            ("${", "in this interpolation".to_owned()),
        ]
    );
}

// The format String's closing quote seems to be there, but the interpolation before
// it never closes, so the interpolation's `${` is labeled rather than the String's
// opening; and so is the String nested in the interpolation that holds the quote.
#[test]
fn an_unclosed_interpolation_labels_its_opener() {
    let cases = [
        // The quote opens a nested String, which runs to the end of the line.
        (
            "def a = $'${ {a: 1 }'",
            ("'", "this starts a String nested in it"),
        ),
        // The quote closes a nested String.
        ("def b = $'x ${'}'", ("'}'", "this String is nested in it")),
        (
            "def c = $'${$'a}'",
            ("$'a}'", "this String is nested in it"),
        ),
    ];
    for (source, nested) in cases {
        let (message, labels) = diagnosis(source);
        assert_eq!(
            message, "unclosed interpolation in format String",
            "{source:?}"
        );
        assert_eq!(
            labels,
            [
                ("${", "this `${` is not closed".to_owned()),
                (nested.0, nested.1.to_owned()),
            ],
            "{source:?}"
        );
        assert_eq!(
            help(source).as_deref(),
            Some("inside `${...}`, each `{` needs a `}`, and a `'` starts a nested String"),
            "{source:?}"
        );
    }
}

#[test]
fn an_unfinished_interpolation_labels_its_unclosed_bracket() {
    let (message, labels) = diagnosis("$'a ${f(}'");
    assert_eq!(message, "expected an expression, but found `}`");
    assert_eq!(
        labels,
        [
            ("}", "unexpected".to_owned()),
            ("(", "this `(` is not closed".to_owned()),
            ("${", "in this interpolation".to_owned()),
        ]
    );
}

// -- Errors inside an interpolation --
// The interpolation's `${` is labeled, not the whole format String: a label holding
// the error's own labels on one line does not render reliably.

#[test]
fn an_interpolation_error_labels_its_opener() {
    // (source, the error's own labels, which the `${` label follows)
    let cases = [
        (
            "def a = $'hi ${ (1 }'",
            vec![("}", "unexpected"), ("(", "this `(` is not closed")],
        ),
        ("def a = $'x ${a b} y'", vec![("b", "unexpected")]),
        // The interpolation's text is lexed on its own.
        ("def a = $'x ${1 ~ 2} y'", vec![("~", "unrecognized")]),
        // A String nested in the interpolation
        (r"def a = $'x ${ 'a\qb' }'", vec![(r"\q", "unrecognized")]),
        (r"def a = $'x ${ x'0g' }'", vec![("g", "not a hex digit")]),
    ];
    for (source, inner) in cases {
        let (_, labels) = diagnosis(source);
        let mut expected: Vec<(&str, String)> = inner
            .into_iter()
            .map(|(text, label)| (text, label.to_owned()))
            .collect();
        expected.push(("${", "in this interpolation".to_owned()));
        assert_eq!(labels, expected, "{source:?}");
    }
}

// Identical labels on each enclosing `${` would stack on one line; only the
// innermost interpolation, the one holding the error, is labeled.
#[test]
fn only_the_innermost_interpolation_is_labeled() {
    let source = "def a = $'${$'${$'${ + }'}'}'";
    let err = parse_program("test.frst", source).expect_err(source);
    let (message, labels) = diagnosis(source);
    assert_eq!(message, "expected an expression, but found `+`");
    assert_eq!(
        labels,
        [
            ("+", "unexpected".to_owned()),
            ("${", "in this interpolation".to_owned()),
        ]
    );
    assert_eq!(
        Some(err.labels()[1].span.start),
        source.rfind("${"),
        "the label is on an outer `${{`"
    );
}

// An interpolation nested in one that never closes is labeled as the enclosing one.
#[test]
fn an_unclosed_nested_interpolation_labels_both_openers() {
    let source = r#"def a = $'${ $"${ {a: 1 }" }'"#;
    let err = parse_program("test.frst", source).expect_err(source);
    let (message, labels) = diagnosis(source);
    assert_eq!(message, "unclosed interpolation in format String");
    assert_eq!(
        labels,
        [
            ("${", "this `${` is not closed".to_owned()),
            ("\"", "this starts a String nested in it".to_owned()),
            ("${", "in this interpolation".to_owned()),
        ]
    );
    let starts: Vec<usize> = err.labels().iter().map(|label| label.span.start).collect();
    assert_eq!(
        starts,
        [
            source.rfind("${").unwrap(),
            source.rfind('"').unwrap(),
            source.find("${").unwrap()
        ],
        "the inner `${{` is the unclosed one"
    );
}

// -- An interpolation that does not close on its line --
// An interpolation is single-line, as its format String is. The error labels what
// kept it open, and its help says how to close it.

/// The help of the error `src` fails with.
fn help(src: &str) -> Option<String> {
    let err = parse_program("test.frst", src).expect_err(src);
    err.help().map(ToOwned::to_owned)
}

const ONE_LINE_HELP: &str =
    "an interpolation ends with `}` on the same line, as its format String does";

#[test]
fn an_interpolation_holding_a_comment_is_unclosed() {
    let source = "def a = $'${x # note}'";
    let (message, labels) = diagnosis(source);
    assert_eq!(message, "unclosed interpolation in format String");
    assert_eq!(
        labels,
        [
            ("${", "this `${` is not closed".to_owned()),
            ("#", "this comment hides the rest of the line".to_owned()),
        ]
    );
    assert_eq!(
        help(source).as_deref(),
        Some("an interpolation cannot hold a comment")
    );
    // Past the interpolation, a `#` is the format String's text.
    parse_program("test.frst", "def a = $'${x} # text'").expect("`#` is text");
}

// With no quote later on the line to close it, the format String is what is
// unclosed, though its interpolation is too: even in brackets, an interpolation
// does not continue onto the next line.
#[test]
fn a_format_string_whose_interpolation_ends_its_line_is_unclosed() {
    let multiline_expression = r"
        def a = $'${(1 +
        2)}'
    ";
    let multiline_array = r"
        def a = $'${[1,
        2]}'
    ";
    for source in [
        multiline_expression,
        multiline_array,
        "def a = $'${x",
        "def a = $'${x\n}'",
        "def a = $'${x\r\n}'",
    ] {
        let (message, labels) = diagnosis(source);
        assert_eq!(message, "unclosed format String", "{source:?}");
        assert_eq!(
            labels,
            [("$'", "this `$'` is not closed".to_owned())],
            "{source:?}"
        );
        assert_eq!(
            help(source).as_deref(),
            Some("a format String ends with `'` on the same line"),
            "{source:?}"
        );
    }
}

#[test]
fn an_interpolation_ending_its_line_before_the_quote_is_unclosed() {
    // The quote later on the line is in a String nested in the interpolation.
    let source = "def a = $'${f('a') + b";
    let (message, labels) = diagnosis(source);
    assert_eq!(message, "unclosed interpolation in format String");
    assert_eq!(labels, [("${", "this `${` is not closed".to_owned())]);
    assert_eq!(help(source).as_deref(), Some(ONE_LINE_HELP));
}

#[test]
fn an_interpolation_holding_a_multiline_string_is_unclosed() {
    let source = r"
        def a = $'${'''
            text
            '''}'
    ";
    let (message, labels) = diagnosis(source);
    assert_eq!(message, "unclosed interpolation in format String");
    assert_eq!(
        labels,
        [
            ("${", "this `${` is not closed".to_owned()),
            ("'''", "this String spans lines".to_owned()),
        ]
    );
    assert_eq!(help(source).as_deref(), Some(ONE_LINE_HELP));
}

#[test]
fn a_one_line_multiline_string_in_an_interpolation_is_its_own_error() {
    // It closes on the line, so the interpolation does too; the String itself is
    // what is wrong.
    let message = diagnosis("def a = $'${'''}'''}'").0;
    assert_eq!(
        message,
        "multiline String must begin with a newline after the opening delimiter"
    );
}

#[test]
fn an_unclosed_brace_in_an_interpolation_is_labeled() {
    for (source, brace) in [
        ("def a = $'${ {a: '}'", "{a"),
        // The innermost `{` left open
        ("def a = $'${ {a: {b: '}'", "{b"),
        ("def a = $'${ {a: {b: 1} c: 'd'", "{a"),
    ] {
        let err = parse_program("test.frst", source).expect_err(source);
        let (message, labels) = diagnosis(source);
        assert_eq!(
            message, "unclosed interpolation in format String",
            "{source:?}"
        );
        assert_eq!(
            labels,
            [
                ("${", "this `${` is not closed".to_owned()),
                ("{", "this `{` is not closed".to_owned()),
            ],
            "{source:?}"
        );
        assert_eq!(
            err.labels()[1].span.start,
            source.find(brace).unwrap(),
            "{source:?}"
        );
        assert_eq!(
            help(source).as_deref(),
            Some("inside `${...}`, each `{` needs a `}` on the same line"),
            "{source:?}"
        );
    }
}

// A String of another kind than the format String's own quote is unclosed in its
// own right, and is reported as it would be anywhere, in the interpolation.
#[test]
fn an_unclosed_string_in_an_interpolation_is_reported_as_itself() {
    let cases = [
        (
            r#"def a = $'${"a}'"#,
            "unclosed String",
            "\"",
            "a String ends with `\"` on the same line",
        ),
        (
            r#"def a = $"${'a}""#,
            "unclosed String",
            "'",
            "a String ends with `'` on the same line",
        ),
        (
            r#"def a = $'${R"(a}'"#,
            "unclosed raw String",
            "R\"",
            "a raw String ends with `)\"` on the same line",
        ),
        (
            r#"def a = $'${$"a}'"#,
            "unclosed format String",
            "$\"",
            "a format String ends with `\"` on the same line",
        ),
        (
            "def a = $'${'''a}'",
            "unclosed multiline String",
            "'''",
            "a multiline String ends with `'''`",
        ),
        (
            r#"def a = $"${x'7d}""#,
            "unclosed Bytes literal",
            "x'",
            "a Bytes literal ends with `'` on the same line",
        ),
    ];
    for (source, message, opener, expected_help) in cases {
        let (found, labels) = diagnosis(source);
        assert_eq!(found, message, "{source:?}");
        assert_eq!(
            labels,
            [
                (opener, format!("this `{opener}` is not closed")),
                ("${", "in this interpolation".to_owned()),
            ],
            "{source:?}"
        );
        assert_eq!(help(source).as_deref(), Some(expected_help), "{source:?}");
    }
}

// -- A missing `with` --
// When the token found in its place is on a later line, that line may hold nothing
// wrong, so the keyword missing its `with` is labeled too.

#[test]
fn a_with_missing_at_line_end_labels_the_keyword() {
    // (source, the found token, the keyword)
    let cases = [
        (
            r"
                def s = xs @ filter(fn x -> x > 2)
                print(s)
            ",
            "print",
            "filter",
        ),
        (
            r"
                def s = map xs

                # a comment
                def y = 2
            ",
            "def",
            "map",
        ),
        (
            r"
                def s = reduce xs init: 0
                print(s)
            ",
            "print",
            "reduce",
        ),
        (
            r"
                foreach [
                    1,
                    2
                ]
                print(1)
            ",
            "print",
            "foreach",
        ),
    ];
    for (source, found, keyword) in cases {
        let (message, labels) = diagnosis(source);
        assert_eq!(
            message,
            format!("expected `with`, but found `{found}`"),
            "{source:?}"
        );
        assert_eq!(
            labels,
            [
                (found, "unexpected".to_owned()),
                (keyword, format!("this `{keyword}` needs `with`")),
            ],
            "{source:?}"
        );
    }
}

// On the line where the operand ends, the error is where `with` belongs, so the
// keyword needs no label, even when the operand starts lines above.
#[test]
fn a_with_missing_on_the_operand_line_labels_only_the_found_token() {
    let multiline_operand = r"
        def s = map [
            1
        ] f
    ";
    let multiline_init = r"
        def s = reduce xs init: [
            0
        ] f
    ";
    for source in [
        "def s = map xs f",
        "def s = reduce xs init: 0 f",
        multiline_operand,
        multiline_init,
    ] {
        let (_, labels) = diagnosis(source);
        assert_eq!(labels, [("f", "unexpected".to_owned())], "{source:?}");
    }
}

// Inside an interpolation, what is found past the last token is its closing `}`,
// on the same line: an interpolation is single-line.
#[test]
fn a_with_missing_at_an_interpolation_end_finds_its_closing_brace() {
    let (message, labels) = diagnosis("def a = $'${map xs}'");
    assert_eq!(message, "expected `with`, but found `}`");
    assert_eq!(
        labels,
        [
            ("}", "unexpected".to_owned()),
            ("${", "in this interpolation".to_owned()),
        ]
    );
}
