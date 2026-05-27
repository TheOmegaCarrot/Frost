mod helpers;

use frost_parse::ast::*;
use helpers::*;

fn str_bytes(expr: &Expr) -> &[u8] {
    match &expr.kind {
        ExprKind::Literal(Literal::String(s)) => s,
        other => panic!("expected String literal, got {other:?}"),
    }
}

// -- Simple strings --

#[test]
fn single_quote_basic() {
    let expr = parse_expr("'hello'");
    assert_eq!(str_bytes(&expr), b"hello");
}

#[test]
fn double_quote_basic() {
    let expr = parse_expr(r#""hello""#);
    assert_eq!(str_bytes(&expr), b"hello");
}

#[test]
fn empty_single_quote() {
    let expr = parse_expr("''");
    assert_eq!(str_bytes(&expr), b"");
}

#[test]
fn empty_double_quote() {
    let expr = parse_expr(r#""""#);
    assert_eq!(str_bytes(&expr), b"");
}

// -- Escape sequences --

#[test]
fn escape_newline() {
    let expr = parse_expr(r"'hello\nworld'");
    assert_eq!(str_bytes(&expr), b"hello\nworld");
}

#[test]
fn escape_tab() {
    let expr = parse_expr(r"'tab\there'");
    assert_eq!(str_bytes(&expr), b"tab\there");
}

#[test]
fn escape_carriage_return() {
    let expr = parse_expr(r"'cr\rhere'");
    assert_eq!(str_bytes(&expr), b"cr\rhere");
}

#[test]
fn escape_backslash() {
    let expr = parse_expr(r"'back\\slash'");
    assert_eq!(str_bytes(&expr), b"back\\slash");
}

#[test]
fn escape_null_byte() {
    let expr = parse_expr(r"'null\0byte'");
    assert_eq!(str_bytes(&expr), b"null\0byte");
}

#[test]
fn escape_single_quote_in_single() {
    let expr = parse_expr(r"'it\'s'");
    assert_eq!(str_bytes(&expr), b"it's");
}

#[test]
fn escape_double_quote_in_double() {
    let expr = parse_expr(r#""say \"hi\"""#);
    assert_eq!(str_bytes(&expr), b"say \"hi\"");
}

#[test]
fn escape_hex() {
    let expr = parse_expr(r"'byte\x0a'");
    assert_eq!(str_bytes(&expr), b"byte\x0a");
}

#[test]
fn escape_hex_ff() {
    let expr = parse_expr(r"'\xff'");
    assert_eq!(str_bytes(&expr), &[0xff]);
}

#[test]
fn multiple_escapes() {
    let expr = parse_expr(r"'a\nb\tc\\'");
    assert_eq!(str_bytes(&expr), b"a\nb\tc\\");
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
fn error_incomplete_hex() {
    let err = parse_err(r"'\x0'");
    assert!(err.contains("\\x escape"), "error was: {err}");
}

// -- Raw strings --

#[test]
fn raw_string_single_quote() {
    let expr = parse_expr("R'(hello\\nworld)'");
    assert_eq!(str_bytes(&expr), b"hello\\nworld");
}

#[test]
fn raw_string_double_quote() {
    let expr = parse_expr(r#"R"(hello\nworld)""#);
    assert_eq!(str_bytes(&expr), b"hello\\nworld");
}

#[test]
fn raw_string_empty() {
    let expr = parse_expr("R'()'");
    assert_eq!(str_bytes(&expr), b"");
}

#[test]
fn raw_string_preserves_backslashes() {
    let expr = parse_expr(r"R'(a\b\c\d)'");
    assert_eq!(str_bytes(&expr), b"a\\b\\c\\d");
}

// -- Multiline strings --

#[test]
fn multiline_basic() {
    let expr = parse_expr("\"\"\"\n    hello\n    world\n    \"\"\"");
    assert_eq!(str_bytes(&expr), b"hello\nworld");
}

#[test]
fn multiline_preserves_extra_indent() {
    let expr = parse_expr("\"\"\"\n  if true:\n    nested\n  \"\"\"");
    assert_eq!(str_bytes(&expr), b"if true:\n  nested");
}

#[test]
fn multiline_empty_lines_preserved() {
    let expr = parse_expr("\"\"\"\n    hello\n\n    world\n    \"\"\"");
    assert_eq!(str_bytes(&expr), b"hello\n\nworld");
}

#[test]
fn multiline_single_line_content() {
    let expr = parse_expr("\"\"\"\n    hello\n    \"\"\"");
    assert_eq!(str_bytes(&expr), b"hello");
}

#[test]
fn multiline_empty() {
    let expr = parse_expr("\"\"\"\n    \"\"\"");
    assert_eq!(str_bytes(&expr), b"");
}

#[test]
fn multiline_no_indent() {
    let expr = parse_expr("\"\"\"\nhello\nworld\n\"\"\"");
    assert_eq!(str_bytes(&expr), b"hello\nworld");
}

#[test]
fn multiline_tab_indent() {
    let expr = parse_expr("\"\"\"\n\thello\n\tworld\n\t\"\"\"");
    assert_eq!(str_bytes(&expr), b"hello\nworld");
}

#[test]
fn multiline_escape_tab() {
    let expr = parse_expr("\"\"\"\n\\t\n\"\"\"");
    assert_eq!(str_bytes(&expr), b"\t");
}

#[test]
fn multiline_escape_backslash() {
    let expr = parse_expr("\"\"\"\n\\\\\n\"\"\"");
    assert_eq!(str_bytes(&expr), b"\\");
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

// -- Strings in expressions --

#[test]
fn string_in_def() {
    let program = parse("def x = 'hello'");
    assert_eq!(program.statements.len(), 1);
    match &program.statements[0].kind {
        StatementKind::Def { expr, .. } => {
            assert_eq!(str_bytes(expr), b"hello");
        }
        other => panic!("expected Def, got {other:?}"),
    }
}

#[test]
fn string_concatenation_parse() {
    let expr = parse_expr("'hello' + ' ' + 'world'");
    assert!(matches!(&expr.kind, ExprKind::BinOp { op: BinOp::Add, .. }));
}
