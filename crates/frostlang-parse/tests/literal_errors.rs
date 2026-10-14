//! Errors inside String, Bytes, based Int, and Float literals: what they say, where
//! their label lands, and their help.

use frostlang_parse::parse_program;

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

// -- Int literals in other bases --
// The error names the literal's first fault and labels it.

const BINARY_HELP: &str = "a binary Int's digits are `0` and `1`";
const OCTAL_HELP: &str = "an octal Int's digits are `0` to `7`";
const HEX_HELP: &str = "a hexadecimal Int's digits are `0` to `9` and `a` to `f`, in either case";

#[test]
fn a_based_int_literal_with_an_invalid_digit_labels_it() {
    // (source, message, the character, label, help)
    let cases = [
        (
            "def a = 0b102",
            "invalid character `2` in binary Int literal",
            "2",
            "not a binary digit",
            BINARY_HELP,
        ),
        (
            "def a = 0o8",
            "invalid character `8` in octal Int literal",
            "8",
            "not an octal digit",
            OCTAL_HELP,
        ),
        (
            "def a = 0xfg",
            "invalid character `g` in hexadecimal Int literal",
            "g",
            "not a hexadecimal digit",
            HEX_HELP,
        ),
        (
            "def a = 0XAG",
            "invalid character `G` in hexadecimal Int literal",
            "G",
            "not a hexadecimal digit",
            HEX_HELP,
        ),
        // A letter that is a hex digit is not a binary one.
        (
            "def b = 0b1c",
            "invalid character `c` in binary Int literal",
            "c",
            "not a binary digit",
            BINARY_HELP,
        ),
        // The first fault is the one named.
        (
            "def a = 0o19z",
            "invalid character `9` in octal Int literal",
            "9",
            "not an octal digit",
            OCTAL_HELP,
        ),
    ];
    for (source, message, character, label, help) in cases {
        assert_error(source, message, character, label, Some(help));
    }
}

#[test]
fn a_based_int_literal_in_an_interpolation_labels_the_digit_in_the_whole_source() {
    let src = "def a = $'${0b3}'";
    let err = parse_program("test.frst", src).expect_err(src);
    assert_eq!(err.message(), "invalid character `3` in binary Int literal");
    let labels: Vec<(&str, &str)> = err
        .labels()
        .iter()
        .map(|label| (&src[label.span.start..label.span.end], label.text.as_str()))
        .collect();
    assert_eq!(
        labels,
        [("3", "not a binary digit"), ("${", "in this interpolation")]
    );
    assert_eq!(err.help(), Some(BINARY_HELP));
}

#[test]
fn a_based_int_literal_with_no_digits_labels_its_prefix() {
    // (source, message, the prefix, help)
    let cases = [
        (
            "def a = 0b",
            "binary Int literal with no digits",
            "0b",
            BINARY_HELP,
        ),
        (
            "def a = 0O + 1",
            "octal Int literal with no digits",
            "0O",
            OCTAL_HELP,
        ),
        (
            "f(0x)",
            "hexadecimal Int literal with no digits",
            "0x",
            HEX_HELP,
        ),
    ];
    for (source, message, prefix, help) in cases {
        assert_error(
            source,
            message,
            prefix,
            &format!("no digits follow this `{prefix}`"),
            Some(help),
        );
    }
}

// Digit separators (`_`) are never the fault: the error names the first real fault
// and ignores the separators around it.

#[test]
fn a_based_int_literal_of_only_separators_has_no_digits() {
    // (source, the prefix as written, message, help)
    let cases = [
        ("def a = 0b_", "0b", "binary", BINARY_HELP),
        ("def a = 0b__", "0b", "binary", BINARY_HELP),
        ("def a = 0B_", "0B", "binary", BINARY_HELP),
        ("def a = 0o_", "0o", "octal", OCTAL_HELP),
        ("def a = 0O___", "0O", "octal", OCTAL_HELP),
        ("def a = 0x_", "0x", "hexadecimal", HEX_HELP),
        ("def a = 0X__", "0X", "hexadecimal", HEX_HELP),
        ("f(0x_, 1)", "0x", "hexadecimal", HEX_HELP),
        ("def a = 0o_ + 1", "0o", "octal", OCTAL_HELP),
        ("[0b_]", "0b", "binary", BINARY_HELP),
    ];
    for (source, prefix, base, help) in cases {
        assert_error(
            source,
            &format!("{base} Int literal with no digits"),
            prefix,
            &format!("no digits follow this `{prefix}`"),
            Some(help),
        );
    }
}

#[test]
fn a_bad_character_among_separators_is_the_fault() {
    // (source, the bad character, base, label, help)
    let cases = [
        (
            "def a = 0x_g",
            "g",
            "hexadecimal",
            "not a hexadecimal digit",
            HEX_HELP,
        ),
        (
            "def a = 0x__g__",
            "g",
            "hexadecimal",
            "not a hexadecimal digit",
            HEX_HELP,
        ),
        (
            "def a = 0xff_fg",
            "g",
            "hexadecimal",
            "not a hexadecimal digit",
            HEX_HELP,
        ),
        (
            "def a = 0xf_g_f",
            "g",
            "hexadecimal",
            "not a hexadecimal digit",
            HEX_HELP,
        ),
        (
            "def a = 0XA_G",
            "G",
            "hexadecimal",
            "not a hexadecimal digit",
            HEX_HELP,
        ),
        (
            "def a = 0b1_2",
            "2",
            "binary",
            "not a binary digit",
            BINARY_HELP,
        ),
        (
            "def a = 0b_2",
            "2",
            "binary",
            "not a binary digit",
            BINARY_HELP,
        ),
        (
            "def a = 0B1__0_2_1",
            "2",
            "binary",
            "not a binary digit",
            BINARY_HELP,
        ),
        (
            "def a = 0b1_c",
            "c",
            "binary",
            "not a binary digit",
            BINARY_HELP,
        ),
        (
            "def a = 0o7__8",
            "8",
            "octal",
            "not an octal digit",
            OCTAL_HELP,
        ),
        (
            "def a = 0O_9",
            "9",
            "octal",
            "not an octal digit",
            OCTAL_HELP,
        ),
        (
            "def a = 0o1_7_8_",
            "8",
            "octal",
            "not an octal digit",
            OCTAL_HELP,
        ),
        // The first fault is the one named, however many follow.
        (
            "def a = 0b1_23_4",
            "2",
            "binary",
            "not a binary digit",
            BINARY_HELP,
        ),
        (
            "def a = 0x_gz",
            "g",
            "hexadecimal",
            "not a hexadecimal digit",
            HEX_HELP,
        ),
    ];
    for (source, character, base, label, help) in cases {
        assert_error(
            source,
            &format!("invalid character `{character}` in {base} Int literal"),
            character,
            label,
            Some(help),
        );
    }
}

#[test]
fn a_based_int_literal_with_separators_in_an_interpolation_adds_the_interpolation() {
    // (source, message, the labeled text, its label, help)
    let cases = [
        (
            "def a = $'${0x_}'",
            "hexadecimal Int literal with no digits",
            "0x",
            "no digits follow this `0x`",
            HEX_HELP,
        ),
        (
            "def a = $'${0b__}'",
            "binary Int literal with no digits",
            "0b",
            "no digits follow this `0b`",
            BINARY_HELP,
        ),
        (
            "def a = $'${0o7_8}'",
            "invalid character `8` in octal Int literal",
            "8",
            "not an octal digit",
            OCTAL_HELP,
        ),
        (
            "def a = $\"x ${0b_2}\"",
            "invalid character `2` in binary Int literal",
            "2",
            "not a binary digit",
            BINARY_HELP,
        ),
    ];
    for (src, message, covered, label, help) in cases {
        let err = parse_program("test.frst", src).expect_err(src);
        assert_eq!(err.message(), message, "{src:?}");
        let labels: Vec<(&str, &str)> = err
            .labels()
            .iter()
            .map(|label| (&src[label.span.start..label.span.end], label.text.as_str()))
            .collect();
        assert_eq!(
            labels,
            [(covered, label), ("${", "in this interpolation")],
            "{src:?}"
        );
        assert_eq!(err.help(), Some(help), "{src:?}");
    }
}

// -- Float literals --
// A malformed Float or exponent is labeled whole, and the help shows the literal
// repaired, separators kept.

#[test]
fn a_float_with_no_digits_after_its_point_before_an_exponent_suggests_a_zero() {
    // (source, the literal, the help's example)
    let cases = [
        ("def a = 1.e3", "1.e3", "1.0e3"),
        ("def a = 1.E3", "1.E3", "1.0E3"),
        ("def a = 1.e+3", "1.e+3", "1.0e+3"),
        ("def a = 1.E-3", "1.E-3", "1.0E-3"),
        ("def a = 1_.e3", "1_.e3", "1_.0e3"),
        ("def a = 1_000.e-3", "1_000.e-3", "1_000.0e-3"),
        ("def a = 1__0.E+3", "1__0.E+3", "1__0.0E+3"),
        ("def a = 1.e_3", "1.e_3", "1.0e_3"),
        ("def a = 1.e+_3", "1.e+_3", "1.0e+_3"),
        ("def a = 1_0.e_3_0", "1_0.e_3_0", "1_0.0e_3_0"),
        // A trailing separator after an exponent digit is kept, not taken for a
        // digitless exponent.
        ("def a = 1.e3_", "1.e3_", "1.0e3_"),
        ("def a = 1.E-3__", "1.E-3__", "1.0E-3__"),
        ("f(1.e3)", "1.e3", "1.0e3"),
        ("[2.E5, 1]", "2.E5", "2.0E5"),
    ];
    for (source, literal, example) in cases {
        assert_error(
            source,
            &format!("invalid number `{literal}`"),
            literal,
            "unrecognized",
            Some(&format!(
                "a Float needs digits after its point, like `{example}`"
            )),
        );
    }
}

// Nothing in the exponent is a digit either, so the example completes it too.
#[test]
fn a_float_with_no_digits_after_its_point_or_in_its_exponent_completes_both() {
    // (source, the literal, the help's example)
    let cases = [
        ("def a = 1.e_", "1.e_", "1.0e3"),
        ("def a = 1.e__", "1.e__", "1.0e3"),
        ("def a = 1.E_", "1.E_", "1.0E3"),
        ("def a = 1.e+__", "1.e+__", "1.0e+3"),
        ("def a = 1.E-_", "1.E-_", "1.0E-3"),
        ("def a = 1_0.e_", "1_0.e_", "1_0.0e3"),
        ("def a = 1_.e+_", "1_.e+_", "1_.0e+3"),
        ("f(1.e_)", "1.e_", "1.0e3"),
    ];
    for (source, literal, example) in cases {
        assert_error(
            source,
            &format!("invalid number `{literal}`"),
            literal,
            "unrecognized",
            Some(&format!(
                "a Float needs digits after its point, like `{example}`"
            )),
        );
    }
}

#[test]
fn an_exponent_with_no_digit_suggests_one() {
    // (source, the literal, the help's example)
    let cases = [
        ("def a = 1e_", "1e_", "1e3"),
        ("def a = 1E_", "1E_", "1E3"),
        ("def a = 1e__", "1e__", "1e3"),
        ("def a = 1e+__", "1e+__", "1e+3"),
        ("def a = 1e-_", "1e-_", "1e-3"),
        ("def a = 1.5E-_", "1.5E-_", "1.5E-3"),
        ("def a = 1.5e_", "1.5e_", "1.5e3"),
        ("def a = .5e_", ".5e_", ".5e3"),
        ("def a = .5E+_", ".5E+_", ".5E+3"),
        ("def a = 1_000e_", "1_000e_", "1_000e3"),
        ("def a = 1_0.2_5e-_", "1_0.2_5e-_", "1_0.2_5e-3"),
        ("def a = 1.5_e_", "1.5_e_", "1.5_e3"),
        ("def a = .5_e+__", ".5_e+__", ".5_e+3"),
        ("f(1e_, 2)", "1e_", "1e3"),
        ("[1.5E-_]", "1.5E-_", "1.5E-3"),
    ];
    for (source, literal, example) in cases {
        assert_error(
            source,
            &format!("invalid number `{literal}`"),
            literal,
            "unrecognized",
            Some(&format!("an exponent needs digits, like `{example}`")),
        );
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
