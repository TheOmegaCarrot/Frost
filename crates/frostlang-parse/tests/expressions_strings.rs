use crate::helpers;

use frostlang_parse::ast::*;
use frostlang_parse::parse_program;
use helpers::*;

/// The literal's text.
fn str_text(expr: &Spanned<Expr>) -> &str {
    match &expr.node {
        Expr::Literal(Literal::String(s)) => s,
        other => panic!("expected String literal, got {other:?}"),
    }
}

// -- Simple strings --

#[test]
fn single_quote_basic() {
    let expr = parse_expr("'hello'");
    assert_eq!(str_text(&expr), "hello");
}

#[test]
fn double_quote_basic() {
    let expr = parse_expr(r#""hello""#);
    assert_eq!(str_text(&expr), "hello");
}

#[test]
fn empty_single_quote() {
    let expr = parse_expr("''");
    assert_eq!(str_text(&expr), "");
}

#[test]
fn empty_double_quote() {
    let expr = parse_expr(r#""""#);
    assert_eq!(str_text(&expr), "");
}

// -- Escape sequences --

#[test]
fn escape_newline() {
    let expr = parse_expr(r"'hello\nworld'");
    assert_eq!(str_text(&expr), "hello\nworld");
}

#[test]
fn escape_tab() {
    let expr = parse_expr(r"'tab\there'");
    assert_eq!(str_text(&expr), "tab\there");
}

#[test]
fn escape_carriage_return() {
    let expr = parse_expr(r"'cr\rhere'");
    assert_eq!(str_text(&expr), "cr\rhere");
}

#[test]
fn escape_backslash() {
    let expr = parse_expr(r"'back\\slash'");
    assert_eq!(str_text(&expr), "back\\slash");
}

#[test]
fn escape_null_byte() {
    let expr = parse_expr(r"'null\0byte'");
    assert_eq!(str_text(&expr), "null\0byte");
}

#[test]
fn escape_single_quote_in_single() {
    let expr = parse_expr(r"'it\'s'");
    assert_eq!(str_text(&expr), "it's");
}

#[test]
fn escape_double_quote_in_double() {
    let expr = parse_expr(r#""say \"hi\"""#);
    assert_eq!(str_text(&expr), "say \"hi\"");
}

#[test]
fn escape_unicode() {
    let expr = parse_expr(r"'omega\u{3a9}'");
    assert_eq!(str_text(&expr), "omega\u{3a9}");
}

// -- Unicode escapes --
// A String is text, so it is valid by construction: every escape names a whole
// Unicode scalar, so there is no byte-level escape to leave a literal invalid.

#[test]
fn unicode_escape_produces_a_multibyte_character() {
    // One escape names the scalar directly, rather than spelling its UTF-8 bytes.
    let expr = parse_expr(r"'\u{e9}'");
    assert_eq!(str_text(&expr), "é");
}

#[test]
fn unicode_escape_rejects_a_surrogate() {
    // char::from_u32 rejects the surrogate range: no such scalar exists.
    let err = parse_err(r"'\u{d800}'");
    assert!(err.contains("scalar value"), "error was: {err}");
}

#[test]
fn unicode_escape_rejects_out_of_range() {
    // Above U+10FFFF is not a scalar value.
    let err = parse_err(r"'\u{110000}'");
    assert!(err.contains("scalar value"), "error was: {err}");
}

#[test]
fn unicode_escape_rejects_an_empty_body() {
    assert_eq!(parse_err_message(r"'\u{}'"), r"invalid `\u` escape `\u{}`");
}

#[test]
fn unicode_escape_rejects_more_than_six_digits() {
    // Rust's `\u{...}` takes 1 to 6 digits; a leading-zero overlong form is rejected
    // even though its value is in range.
    let err = parse_err(r"'\u{0000041}'");
    assert!(err.contains("6 hex digits"), "error was: {err}");
}

#[test]
fn unicode_escape_rejects_a_leading_plus() {
    // `u32::from_str_radix` would tolerate `+41`; the body must be hex digits only.
    assert_eq!(
        parse_err_message(r"'\u{+41}'"),
        r"invalid `\u` escape `\u{+41}`"
    );
}

#[test]
fn unicode_escape_rejects_a_missing_brace() {
    let err = parse_err(r"'\u41'");
    assert!(err.contains("must be followed by"), "error was: {err}");
}

#[test]
fn unicode_escape_rejects_an_unterminated_body() {
    let err = parse_err(r"'\u{41'");
    assert!(err.contains("unterminated"), "error was: {err}");
}

#[test]
fn source_multibyte_characters_pass_through() {
    let expr = parse_expr("'héllo'");
    assert_eq!(str_text(&expr), "héllo");
}

#[test]
fn multiple_escapes() {
    let expr = parse_expr(r"'a\nb\tc\\'");
    assert_eq!(str_text(&expr), "a\nb\tc\\");
}

// -- Escape errors --

#[test]
fn error_invalid_escape() {
    let err = parse_err(r"'bad\q'");
    assert!(err.contains("invalid escape"), "error was: {err}");
}

#[test]
fn error_single_quote_escape_in_double() {
    let err = parse_err(r#""can\'t""#);
    assert!(err.contains("invalid escape"), "error was: {err}");
}

#[test]
fn error_double_quote_escape_in_single() {
    let err = parse_err(r#"'say \"hi\"'"#);
    assert!(err.contains("invalid escape"), "error was: {err}");
}

#[test]
fn x_escape_is_no_longer_valid() {
    // There is no `\x` escape in String literals; Bytes literals use hex-pair syntax (`x'..'`) instead.
    let err = parse_err(r"'\x0a'");
    assert!(err.contains("invalid escape"), "error was: {err}");
}

// -- Raw strings --

#[test]
fn raw_string_single_quote() {
    let expr = parse_expr("R'(hello\\nworld)'");
    assert_eq!(str_text(&expr), "hello\\nworld");
}

#[test]
fn raw_string_double_quote() {
    let expr = parse_expr(r#"R"(hello\nworld)""#);
    assert_eq!(str_text(&expr), "hello\\nworld");
}

#[test]
fn raw_string_empty() {
    let expr = parse_expr("R'()'");
    assert_eq!(str_text(&expr), "");
}

#[test]
fn raw_string_preserves_backslashes() {
    let expr = parse_expr(r"R'(a\b\c\d)'");
    assert_eq!(str_text(&expr), "a\\b\\c\\d");
}

// -- Quoted Strings are single-line --

#[test]
fn error_line_break_in_quoted_string() {
    let single = r"
        'a
        b'
    ";
    let double = r#"
        "a
        b"
    "#;
    for source in [single, double] {
        parse_err(source);
    }
}

// A backslash does not continue a quoted String onto the next line.
#[test]
fn error_escaped_line_break_in_quoted_string() {
    for source in ["'a\\\nb'", "\"a\\\nb\""] {
        parse_err(source);
    }
}

// An unclosed quote does not run on into later lines to find a closer.
#[test]
fn error_quote_unclosed_on_its_line() {
    let source = r"
        def a = 'unclosed
        def b = 'closed'
    ";
    parse_err(source);
}

// -- Multiline strings --

#[test]
fn multiline_basic() {
    let expr = parse_expr("\"\"\"\n    hello\n    world\n    \"\"\"");
    assert_eq!(str_text(&expr), "hello\nworld");
}

#[test]
fn multiline_preserves_extra_indent() {
    let expr = parse_expr("\"\"\"\n  if true:\n    nested\n  \"\"\"");
    assert_eq!(str_text(&expr), "if true:\n  nested");
}

#[test]
fn multiline_empty_lines_preserved() {
    let expr = parse_expr("\"\"\"\n    hello\n\n    world\n    \"\"\"");
    assert_eq!(str_text(&expr), "hello\n\nworld");
}

#[test]
fn multiline_single_line_content() {
    let expr = parse_expr("\"\"\"\n    hello\n    \"\"\"");
    assert_eq!(str_text(&expr), "hello");
}

#[test]
fn multiline_empty() {
    let expr = parse_expr("\"\"\"\n    \"\"\"");
    assert_eq!(str_text(&expr), "");
}

#[test]
fn multiline_no_indent() {
    let expr = parse_expr("\"\"\"\nhello\nworld\n\"\"\"");
    assert_eq!(str_text(&expr), "hello\nworld");
}

#[test]
fn multiline_tab_indent() {
    let expr = parse_expr("\"\"\"\n\thello\n\tworld\n\t\"\"\"");
    assert_eq!(str_text(&expr), "hello\nworld");
}

#[test]
fn multiline_escape_tab() {
    let expr = parse_expr("\"\"\"\n\\t\n\"\"\"");
    assert_eq!(str_text(&expr), "\t");
}

#[test]
fn multiline_escape_backslash() {
    let expr = parse_expr("\"\"\"\n\\\\\n\"\"\"");
    assert_eq!(str_text(&expr), "\\");
}

#[test]
fn multiline_escape_unicode() {
    let expr = parse_expr("\"\"\"\n\\u{3a9}\n\"\"\"");
    assert_eq!(str_text(&expr), "\u{3a9}");
}

// -- Multiline errors --

#[test]
fn error_multiline_content_less_indented() {
    let err = parse_err("\"\"\"\noops\n  \"\"\"");
    assert!(err.contains("indented less"), "error was: {err}");
}

#[test]
fn error_multiline_multibyte_at_indent_boundary() {
    // A body line starting with a multibyte character under a nonzero closing indent
    // is an under-indentation error; slicing the indent prefix must not panic mid-char.
    let err = parse_err("\"\"\"\n\u{20ac}x\n  \"\"\"");
    assert!(err.contains("indented less"), "error was: {err}");
}

#[test]
fn error_multiline_closing_not_own_line() {
    let err = parse_err("\"\"\"\nhello\nworld\"\"\"");
    assert!(err.contains("own line"), "error was: {err}");
}

// -- Multiline whitespace and indentation --
//
// These sources are written with escapes, not raw literals, so that the trailing
// whitespace and the tabs under test stay visible and survive editors trimming them.

/// Each multiline String delimiter.
const DELIMITERS: [&str; 2] = ["'''", "\"\"\""];

/// The text of the multiline String `{q}` + `body` + `{q}`, for each delimiter `q`,
/// asserting it is `expected`, under LF and CRLF line breaks alike.
fn assert_multiline_text(body: &str, expected: &str) {
    for q in DELIMITERS {
        let source = format!("{q}{body}{q}");
        for source in [source.clone(), source.replace('\n', "\r\n")] {
            assert_eq!(
                str_text(&parse_expr(&source)),
                expected,
                "source: {source:?}"
            );
        }
    }
}

/// The error message for the multiline String `{q}` + `body` + `{q}`, for each
/// delimiter `q`, asserting it is `expected`.
fn assert_multiline_error(body: &str, expected: &str) {
    for q in DELIMITERS {
        let source = format!("{q}{body}{q}");
        assert_eq!(parse_err_message(&source), expected, "source: {source:?}");
    }
}

const LESS_INDENTED: &str = "multiline String content is indented less than the closing delimiter";

const DIFFERENTLY_INDENTED: &str =
    "multiline String content is indented with different whitespace than the closing delimiter";

#[test]
fn multiline_trailing_spaces_and_tabs_are_stripped() {
    assert_multiline_text("\n    a  \n    b\t\n    c \t \n    ", "a\nb\nc");
}

#[test]
fn multiline_trailing_whitespace_on_an_unindented_line_is_stripped() {
    assert_multiline_text("\na  \nb\t\n", "a\nb");
}

#[test]
fn multiline_whitespace_only_lines_are_empty_lines() {
    // Indented past the closing delimiter, as far as it, and less than it
    assert_multiline_text("\n    a\n        \n    b\n    ", "a\n\nb");
    assert_multiline_text("\n    a\n    \n    b\n    ", "a\n\nb");
    assert_multiline_text("\n    a\n  \n    b\n    ", "a\n\nb");
    // Whitespace of the other kind than the closing delimiter's indentation
    assert_multiline_text("\n    a\n\t\n    b\n    ", "a\n\nb");
    assert_multiline_text("\n\ta\n    \n\tb\n\t", "a\n\nb");
}

#[test]
fn multiline_escaped_trailing_whitespace_is_kept() {
    assert_multiline_text(
        "\n    a\\t\n    b\\u{20}\n    c\\u{9}  \n    ",
        "a\t\nb \nc\t",
    );
}

#[test]
fn multiline_whitespace_after_the_opener_is_ignored() {
    assert_multiline_text("  \n    a\n    ", "a");
    assert_multiline_text("\t\n    a\n    ", "a");
    assert_multiline_text(" \t \n    a\n    ", "a");
    assert_multiline_text(" \n    ", "");
}

#[test]
fn error_multiline_text_after_the_opener_and_its_whitespace() {
    let message = "multiline String must begin with a newline after the opening delimiter";
    assert_multiline_error("  a\n    ", message);
    assert_multiline_error(" \t# not a comment\n    ", message);
}

#[test]
fn multiline_extra_indentation_of_either_kind_is_kept() {
    assert_multiline_text("\n  \tx\n    y\n  ", "\tx\n  y");
    assert_multiline_text("\n\t\tx\n\t  y\n\t", "\tx\n  y");
}

#[test]
fn multiline_mixed_indentation_matching_the_closer_exactly_is_stripped() {
    assert_multiline_text("\n\t  x\n\t    y\n\t  ", "x\n  y");
}

#[test]
fn error_multiline_tab_indentation_under_space_indentation() {
    // However many spaces a tab is taken to stand for
    assert_multiline_error("\n\tx\n    ", DIFFERENTLY_INDENTED);
    assert_multiline_error("\n\t\tx\n    ", DIFFERENTLY_INDENTED);
    assert_multiline_error("\n    a\n\tb\n    ", DIFFERENTLY_INDENTED);
}

#[test]
fn error_multiline_space_indentation_under_tab_indentation() {
    // Whether the spaces are fewer bytes than the closer's indentation or more
    assert_multiline_error("\n x\n\t", DIFFERENTLY_INDENTED);
    assert_multiline_error("\n  x\n\t", DIFFERENTLY_INDENTED);
    assert_multiline_error("\n    x\n\t", DIFFERENTLY_INDENTED);
    assert_multiline_error("\n\ta\n    b\n\t", DIFFERENTLY_INDENTED);
}

#[test]
fn error_multiline_mixed_indentation_in_another_order() {
    assert_multiline_error("\n  \tx\n\t  ", DIFFERENTLY_INDENTED);
    assert_multiline_error("\n \t x\n\t  ", DIFFERENTLY_INDENTED);
}

#[test]
fn error_multiline_a_prefix_of_the_closers_indentation_is_less_indented() {
    assert_multiline_error("\n  x\n    ", LESS_INDENTED);
    assert_multiline_error("\nx\n\t", LESS_INDENTED);
    assert_multiline_error("\n\tx\n\t\t", LESS_INDENTED);
    assert_multiline_error("\n\t x\n\t  ", LESS_INDENTED);
}

// -- Multiline closing delimiters --
//
// The first delimiter not broken up by an escape closes a multiline String, and it
// must start its line. Each case is written for `'''`; the same case for `"""` swaps
// every `'` for `"`.

/// `source` as written, with `'''`, and with every `'` swapped for `"`, for `"""`.
fn in_both_delimiters(source: &str) -> [String; 2] {
    [source.to_owned(), source.replace('\'', "\"")]
}

#[test]
fn multiline_an_escaped_quote_breaks_up_the_delimiter() {
    // Each a String's one line, and the text it reads as
    let lines = [
        // At a line's start, mid-line, and ending a line
        (r"\'''", "'''"),
        (r"it\'''s", "it'''s"),
        (r"end\'''", "end'''"),
        // Any one of the three quotes escaped, or all of them
        (r"'\''", "'''"),
        (r"''\'", "'''"),
        (r"\'\'\'", "'''"),
    ];
    for (line, expected) in lines {
        let source = format!("'''\n    {line}\n    '''");
        let sources = in_both_delimiters(&source);
        for (source, expected) in sources.iter().zip(in_both_delimiters(expected)) {
            assert_eq!(str_text(&parse_expr(source)), expected, "{source:?}");
        }
    }
}

#[test]
fn multiline_a_delimiter_of_the_other_kind_needs_no_escape() {
    let source = r#"'''
        a """ b
        """
        '''"#;
    assert_eq!(str_text(&parse_expr(source)), "a \"\"\" b\n\"\"\"");
}

/// Asserts that `source` fails with a closer not on its own line, labeled at the
/// closer starting at byte `closer` and helped toward escaping it.
fn assert_closer_not_on_its_own_line(source: &str, closer: usize) {
    let err = parse_program("test.frst", source).expect_err(source);
    let delimiter = &source[closer..closer + 3];
    assert_eq!(
        err.message(),
        "closing delimiter of multiline String must be on its own line",
        "{source:?}"
    );
    let span = err.primary_span();
    assert_eq!((span.start, span.end), (closer, closer + 3), "{source:?}");
    assert_eq!(
        err.help(),
        Some(format!("to write `{delimiter}` as text, escape a quote: `\\{delimiter}`").as_str()),
        "{source:?}"
    );
}

#[test]
fn error_multiline_a_delimiter_after_text_closes_the_string_mid_line() {
    // Lines after the misplaced closer read as code if not rejected at the closer.
    let cases = [
        r"'''
            it'''s
            '''",
        r"'''
            a
            b'''",
        r"'''
            x''' + 1",
        // After an escaped backslash, the delimiter is not escaped.
        r"'''
            a\\'''
            '''",
        r"'''
            \\'''
            '''",
    ];
    for source in cases {
        for source in in_both_delimiters(source) {
            for source in [source.clone(), source.replace('\n', "\r\n")] {
                let closer = 3 + source[3..].find(&source[..3]).expect("a closer");
                assert_closer_not_on_its_own_line(&source, closer);
            }
        }
    }
}

// Text on the opening line is the earlier fault, so it is the one reported.
#[test]
fn error_multiline_text_on_the_opening_line_outranks_a_misplaced_closer() {
    let source = r"'''a
        b'''";
    for source in in_both_delimiters(source) {
        assert_eq!(
            parse_err_message(&source),
            "multiline String must begin with a newline after the opening delimiter",
            "{source:?}"
        );
    }
}

// -- Additional escape edge cases --

#[test]
fn escape_unicode_null() {
    // NUL is an ordinary scalar, spellable as `\u{0}` or the shorter `\0`.
    let expr = parse_expr(r"'\u{0}'");
    assert_eq!(str_text(&expr), "\0");
}

#[test]
fn error_unicode_invalid_digits() {
    assert_eq!(
        parse_err_message(r"'\u{ZZ}'"),
        r"invalid `\u` escape `\u{ZZ}`"
    );
}

#[test]
fn error_dollar_escape_in_regular_string() {
    // \$ is only valid in format strings, not regular strings
    let err = parse_err(r"'literal \$ here'");
    assert!(err.contains("invalid escape"), "error was: {err}");
}

// -- Simple strings with embedded quotes --

#[test]
fn single_quote_with_double_quotes() {
    let expr = parse_expr(r#"'has "double" quotes'"#);
    assert_eq!(str_text(&expr), "has \"double\" quotes");
}

#[test]
fn double_quote_with_single_quotes() {
    let expr = parse_expr(r#""has 'single' quotes""#);
    assert_eq!(str_text(&expr), "has 'single' quotes");
}

// -- Raw string edge cases --

#[test]
fn raw_string_with_opposite_quotes() {
    let expr = parse_expr(r#"R'(has "quotes" inside)'"#);
    assert_eq!(str_text(&expr), r#"has "quotes" inside"#);
}

#[test]
fn raw_string_with_backslash_sequences() {
    let expr = parse_expr(r"R'(\n\t\r\0\xff)'");
    assert_eq!(str_text(&expr), r"\n\t\r\0\xff");
}

// -- Multiline single-quote form --

#[test]
fn multiline_single_quote() {
    let expr = parse_expr("'''\n    hello\n    '''");
    assert_eq!(str_text(&expr), "hello");
}

#[test]
fn multiline_single_quote_with_double_quotes() {
    let expr = parse_expr("'''\n    has \"quotes\"\n    '''");
    assert_eq!(str_text(&expr), "has \"quotes\"");
}

// -- Multiline with only empty lines --

#[test]
fn multiline_only_empty_lines() {
    let expr = parse_expr("\"\"\"\n\n\n\"\"\"");
    assert_eq!(str_text(&expr), "\n");
}

// -- Strings in expressions --

#[test]
fn string_in_def() {
    let program = parse("def x = 'hello'");
    assert_eq!(program.statements.len(), 1);
    match &program.statements[0].node {
        Statement::Def { expr, .. } => {
            assert_eq!(str_text(expr), "hello");
        }
        other => panic!("expected Def, got {other:?}"),
    }
}

#[test]
fn string_concatenation_parse() {
    let expr = parse_expr("'hello' + ' ' + 'world'");
    assert!(matches!(
        &expr.node,
        Expr::BinOp {
            op: Spanned {
                node: BinOp::Add,
                ..
            },
            ..
        }
    ));
}
