//! Errors inside String and Bytes literals: what they say, where their label lands,
//! and their help.

use frost_parse::parse_program;

/// The error `src` fails with: its message, its one label as the source text it
/// covers and the label's text, and its help.
fn diagnosis(src: &str) -> (String, (&str, String), Option<String>) {
    let err = parse_program("test.frst", src).expect_err(src);
    let [label] = err.labels() else {
        panic!("{src:?}: expected one label, got {:?}", err.labels());
    };
    (
        err.message().to_owned(),
        (&src[label.span.start..label.span.end], label.text.clone()),
        err.help().map(str::to_owned),
    )
}

/// Asserts that `src` fails with `message`, one label covering `covered` with the
/// text `label`, and `help`; `covered` must be the first such text in `src`.
#[track_caller]
fn assert_error(src: &str, message: &str, covered: &str, label: &str, help: Option<&str>) {
    let err = parse_program("test.frst", src).expect_err(src);
    let (actual_message, actual_label, actual_help) = diagnosis(src);
    assert_eq!(actual_message, message, "{src:?}");
    assert_eq!(actual_label, (covered, label.to_owned()), "{src:?}");
    assert_eq!(
        Some(err.labels()[0].span.start),
        src.find(covered),
        "{src:?}: the label is on a later {covered:?}"
    );
    assert_eq!(actual_help.as_deref(), help, "{src:?}");
}

const SINGLE_QUOTED_ESCAPES: &str =
    r"this String's escapes are `\n`, `\r`, `\t`, `\\`, `\0`, `\'`, and `\u{...}`";
const DOUBLE_QUOTED_ESCAPES: &str =
    r#"this String's escapes are `\n`, `\r`, `\t`, `\\`, `\0`, `\"`, and `\u{...}`"#;
const SINGLE_FORMAT_ESCAPES: &str =
    r"this format String's escapes are `\n`, `\r`, `\t`, `\\`, `\0`, `\$`, `\'`, and `\u{...}`";
const DOUBLE_FORMAT_ESCAPES: &str =
    r#"this format String's escapes are `\n`, `\r`, `\t`, `\\`, `\0`, `\$`, `\"`, and `\u{...}`"#;
const MULTILINE_ESCAPES: &str =
    r#"this multiline String's escapes are `\t`, `\\`, `\0`, `\'`, `\"`, and `\u{...}`"#;
const UNICODE_ESCAPE_HELP: &str =
    r"a Unicode escape holds 1 to 6 hex digits in braces, like `\u{e9}`";

// -- Invalid escapes --
// Only the escape is labeled, and the help lists the escapes that kind of String takes.

#[test]
fn an_unknown_escape_labels_the_escape_and_lists_the_valid_ones() {
    // (source, the escape, the help)
    let cases = [
        (r"def a = 'a\qb'", r"\q", SINGLE_QUOTED_ESCAPES),
        (r#"def a = "a\x41""#, r"\x", DOUBLE_QUOTED_ESCAPES),
        (r"def a = $'\q${1}'", r"\q", SINGLE_FORMAT_ESCAPES),
        (r#"def a = $"${1} \q""#, r"\q", DOUBLE_FORMAT_ESCAPES),
        // The label covers the whole of a multibyte character.
        ("def a = 'caf\\\u{e9}'", "\\\u{e9}", SINGLE_QUOTED_ESCAPES),
    ];
    for (source, escape, help) in cases {
        let message = format!("invalid escape sequence `{escape}`");
        assert_error(source, &message, escape, "unrecognized", Some(help));
    }
}

// An escape another kind of String takes is known, so it is not "unrecognized",
// and the help says how to write its character here.
#[test]
fn an_escape_of_another_kind_of_string_says_what_to_write() {
    let no_dollar = "`$` needs no backslash outside a format String";
    let no_double = r#"`"` needs no backslash in a `'` String"#;
    let no_single = r#"`'` needs no backslash in a `"` String"#;
    // (source, the escape, the help)
    let cases = [
        (r"print('\$')", r"\$", SINGLE_QUOTED_ESCAPES, no_dollar),
        (
            r#"def a = 'it\"s'"#,
            r#"\""#,
            SINGLE_QUOTED_ESCAPES,
            no_double,
        ),
        (
            r#"def a = "it\'s""#,
            r"\'",
            DOUBLE_QUOTED_ESCAPES,
            no_single,
        ),
        (
            r#"def a = $"it\'s""#,
            r"\'",
            DOUBLE_FORMAT_ESCAPES,
            no_single,
        ),
        (
            r#"def a = $'say \"hi\"'"#,
            r#"\""#,
            SINGLE_FORMAT_ESCAPES,
            no_double,
        ),
    ];
    for (source, escape, escapes, instead) in cases {
        let message = format!("invalid escape sequence `{escape}`");
        let help = format!("{escapes}; {instead}");
        assert_error(source, &message, escape, "this escape", Some(&help));
    }
}

// A space or control character after the `\` is named by its code point, since in
// backticks it would print as itself.
#[test]
fn an_escape_of_an_invisible_character_names_its_code_point() {
    let cases = [
        ("def a = $\"\\\t\"", "\\\t", "U+0009", DOUBLE_FORMAT_ESCAPES),
        ("def a = '\\ '", "\\ ", "U+0020", SINGLE_QUOTED_ESCAPES),
        (
            "def a = '\\\u{7}'",
            "\\\u{7}",
            "U+0007",
            SINGLE_QUOTED_ESCAPES,
        ),
    ];
    for (source, escape, code_point, help) in cases {
        let message = format!(r"invalid escape sequence: `\` followed by {code_point}");
        assert_error(source, &message, escape, "unrecognized", Some(help));
    }
}

#[test]
fn an_invalid_escape_after_a_valid_one_is_labeled() {
    assert_error(
        r"def a = 'a\nb\\c\qd'",
        r"invalid escape sequence `\q`",
        r"\q",
        "unrecognized",
        Some(SINGLE_QUOTED_ESCAPES),
    );
}

// -- Multiline Strings --
// Lines lose their indentation before escapes are decoded, but the label still lands
// on the escape in the source.

#[test]
fn a_multiline_invalid_escape_labels_the_escape() {
    let source = r"
        def a = '''
            ok \\ fine
            then \q here
            '''
    ";
    assert_error(
        source,
        r"invalid escape sequence `\q`",
        r"\q",
        "unrecognized",
        Some(MULTILINE_ESCAPES),
    );
}

#[test]
fn a_multiline_dollar_escape_needs_no_backslash() {
    let source = r"
        def a = '''
            ok \\ fine
            then \$ here
            '''
    ";
    let help = format!("{MULTILINE_ESCAPES}; `$` needs no backslash outside a format String");
    assert_error(
        source,
        r"invalid escape sequence `\$`",
        r"\$",
        "this escape",
        Some(&help),
    );
}

#[test]
fn a_multiline_line_break_escape_points_to_real_line_breaks() {
    let source = r"
        def a = '''
            a\nb
            '''
    ";
    let help = format!(r"{MULTILINE_ESCAPES}; it takes real line breaks instead of `\n`");
    assert_error(
        source,
        r"invalid escape sequence `\n`",
        r"\n",
        "this escape",
        Some(&help),
    );
}

// A backslash ending a line names the line end, rather than printing a raw line
// break in the message.
#[test]
fn a_multiline_backslash_at_line_end_is_named() {
    let lf = r#"
        def a = """
            abc\
            def
            """
    "#;
    let crlf = lf.replace('\n', "\r\n");
    // The last line of the body ends before the closing delimiter's line.
    let last_line = r"
        def a = '''
            abc\
            '''
    ";
    for source in [lf, &crlf, last_line] {
        assert_error(
            source,
            r"invalid escape sequence: `\` at the end of a line",
            "\\",
            "this escape",
            Some(MULTILINE_ESCAPES),
        );
    }
}

// A format String is single-line, so a backslash ending its line ends it too soon.
#[test]
fn a_format_backslash_at_line_end_is_named() {
    let single = r"
        def a = $'abc\
        def'
    ";
    let double = r#"
        def a = $"abc\
        def"
    "#;
    for (source, quote) in [(single, '\''), (double, '"')] {
        let help = format!("a format String ends with `{quote}` on the same line");
        assert_error(
            source,
            r"invalid escape sequence: `\` at the end of a line",
            "\\",
            "this escape",
            Some(&help),
        );
    }
}

// -- Unicode escapes --

#[test]
fn a_malformed_unicode_escape_labels_the_escape() {
    let multiline = r"
        def a = '''
            ok
            \u{zz}
            '''
    ";
    // (source, message, the escape)
    let cases = [
        (
            r#"def a = "\u41""#,
            r"`\u` escape must be followed by `{`",
            r"\u",
        ),
        (r"def a = '\u{41'", r"unterminated `\u` escape", r"\u{41"),
        (
            r"def a = 'x\u{zz}y'",
            r"invalid `\u` escape `\u{zz}`",
            r"\u{zz}",
        ),
        (r"def a = '\u{}'", r"invalid `\u` escape `\u{}`", r"\u{}"),
        (
            r"def a = '\u{1234567}'",
            r"`\u` escape `\u{1234567}` has more than 6 hex digits",
            r"\u{1234567}",
        ),
        (
            r"def a = $'${1}\u{zz}'",
            r"invalid `\u` escape `\u{zz}`",
            r"\u{zz}",
        ),
        (multiline, r"invalid `\u` escape `\u{zz}`", r"\u{zz}"),
    ];
    for (source, message, escape) in cases {
        assert_error(
            source,
            message,
            escape,
            "this escape",
            Some(UNICODE_ESCAPE_HELP),
        );
    }
}

// The escape's form is right; its value is not, so the form's help would not help.
#[test]
fn an_out_of_range_unicode_escape_has_no_help() {
    for (source, escape) in [
        (r"def a = '\u{110000}'", r"\u{110000}"),
        (r"def a = $'\u{d800}'", r"\u{d800}"),
    ] {
        let hex = &escape[3..escape.len() - 1];
        let message = format!(r"`\u{{{hex}}}` is not a valid Unicode scalar value");
        assert_error(source, &message, escape, "this escape", None);
    }
}

// -- Bytes literals --
// The error names the literal's first fault and labels the character at fault.

const PAIRS_HELP: &str = "a Bytes literal holds pairs of hex digits, like `x'00ff'`";
const NO_SPACES_HELP: &str =
    "a Bytes literal holds pairs of hex digits with no spaces, like `x'00ff'`";

#[test]
fn a_bytes_literal_with_a_non_hex_digit_labels_it() {
    // (source, message, the character)
    let cases = [
        (
            "def b = x'0g'",
            "invalid character `g` in Bytes literal",
            "g",
        ),
        (
            "def b = x'hello world'",
            "invalid character `h` in Bytes literal",
            "h",
        ),
        (
            "def b = x'0\u{e9}'",
            "invalid character `\u{e9}` (U+00E9) in Bytes literal",
            "\u{e9}",
        ),
        // The other quote is in the literal, which a later quote closes.
        (
            r#"def b = x"00'ff""#,
            "invalid character `'` in Bytes literal",
            "'",
        ),
        // A backtick in backticks would read as noise.
        ("def b = x'0`'", "invalid backtick in Bytes literal", "`"),
    ];
    for (source, message, character) in cases {
        assert_error(
            source,
            message,
            character,
            "not a hex digit",
            Some(PAIRS_HELP),
        );
    }
}

#[test]
fn a_bytes_literal_with_whitespace_labels_it() {
    // (source, message, the source from the whitespace on)
    let cases = [
        ("def b = x'00 ff'", "space in Bytes literal", " ff"),
        (
            "def b = x'00\tff'",
            "invalid character U+0009 in Bytes literal",
            "\tff",
        ),
    ];
    for (source, message, from_whitespace) in cases {
        let err = parse_program("test.frst", source).expect_err(source);
        let (actual_message, label, help) = diagnosis(source);
        assert_eq!(actual_message, message, "{source:?}");
        assert_eq!(
            label,
            (&from_whitespace[..1], "not a hex digit".to_owned()),
            "{source:?}"
        );
        assert_eq!(
            Some(err.labels()[0].span.start),
            source.find(from_whitespace),
            "{source:?}: the label is not on the whitespace in the literal"
        );
        assert_eq!(help.as_deref(), Some(NO_SPACES_HELP), "{source:?}");
    }
}

#[test]
fn a_bytes_literal_with_an_odd_digit_count_labels_the_last_digit() {
    for (source, digit) in [
        ("def b = x'abc'", "c"),
        ("def b = x'a'", "a"),
        (r#"def b = x"012""#, "2"),
    ] {
        assert_error(
            source,
            "odd number of hex digits in Bytes literal",
            digit,
            "this digit has no pair",
            Some(PAIRS_HELP),
        );
    }
}

// With no closing quote on its line, the literal is unclosed, whatever follows it:
// the code after it is no fault of the literal's.
#[test]
fn an_unclosed_bytes_literal_labels_its_opener() {
    let on_a_later_line = r#"
        def b = x"00ff
        print("done")
    "#;
    let cases = [
        ("def b = x'00ff", "x'"),
        ("def b = x'", "x'"),
        ("print(x'00ff)", "x'"),
        ("def b = x'00ff + 1", "x'"),
        // The other quote does not close it.
        (r#"def b = x"00ff'"#, "x\""),
        (on_a_later_line, "x\""),
    ];
    for (source, opener) in cases {
        let quote = &opener[1..];
        assert_error(
            source,
            "unclosed Bytes literal",
            opener,
            &format!("this `{opener}` is not closed"),
            Some(&format!(
                "a Bytes literal ends with `{quote}` on the same line"
            )),
        );
    }
}

// -- Unclosed Strings --
// As for a Bytes literal, the label names the opener left unclosed.

#[test]
fn an_unclosed_string_labels_its_opener() {
    let multiline = r#"
        def x = """
        abc
    "#;
    let backslash_at_line_end = r#"
        def a = "abc\
        def"
    "#;
    // (source, message, the opener, the help)
    let cases = [
        (
            "print('abc)",
            "unclosed String",
            "'",
            "a String ends with `'` on the same line",
        ),
        (
            backslash_at_line_end,
            "unclosed String",
            "\"",
            "a String ends with `\"` on the same line",
        ),
        (
            "print($'total: ${n)",
            "unclosed format String",
            "$'",
            "a format String ends with `'` on the same line",
        ),
        (
            "def a = R'(abc",
            "unclosed raw String",
            "R'",
            "a raw String ends with `)'` on the same line",
        ),
        (
            multiline,
            "unclosed multiline String",
            "\"\"\"",
            "a multiline String ends with `\"\"\"`",
        ),
    ];
    for (source, message, opener, help) in cases {
        assert_error(
            source,
            message,
            opener,
            &format!("this `{opener}` is not closed"),
            Some(help),
        );
    }
}
