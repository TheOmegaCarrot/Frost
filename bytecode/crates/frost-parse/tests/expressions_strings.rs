mod helpers;

use frost_parse::ast::*;
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
    let err = parse_err(r"'\u{}'");
    assert!(err.contains("\\u escape"), "error was: {err}");
}

#[test]
fn unicode_escape_rejects_more_than_six_digits() {
    // Rust's `\u{...}` takes 1 to 6 digits; a leading-zero overlong form is rejected
    // even though its value is in range.
    let err = parse_err(r"'\u{0000041}'");
    assert!(err.contains("6 hex digits"), "error was: {err}");
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
    // Byte escapes belong to Bytes literals now; a String is text only.
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
fn error_multiline_closing_not_own_line() {
    let err = parse_err("\"\"\"\nhello\nworld\"\"\"");
    assert!(err.contains("own line"), "error was: {err}");
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
    let err = parse_err(r"'\u{ZZ}'");
    assert!(err.contains("\\u escape"), "error was: {err}");
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
