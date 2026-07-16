mod helpers;

use frost_parse::ast::*;
use helpers::*;

fn segments(expr: &Spanned<Expr>) -> &[FormatSegment] {
    match &expr.node {
        Expr::FormatString(segs) => segs,
        other => panic!("expected FormatString, got {other:?}"),
    }
}

fn assert_literal_seg(seg: &FormatSegment, expected: &[u8]) {
    match seg {
        FormatSegment::Literal(bytes) => assert_eq!(bytes, expected, "literal mismatch"),
        other => panic!("expected Literal segment, got {other:?}"),
    }
}

fn assert_interp_name(seg: &FormatSegment, expected: &str) {
    match seg {
        FormatSegment::Interpolation(expr) => {
            assert!(
                matches!(&expr.node, Expr::NameLookup(n) if n == expected),
                "expected NameLookup({expected}), got {:?}",
                expr.node
            );
        }
        other => panic!("expected Interpolation segment, got {other:?}"),
    }
}

// -- Basic format strings --

#[test]
fn format_no_interpolation() {
    let expr = parse_expr("$'hello'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    assert_literal_seg(&segs[0], b"hello");
}

#[test]
fn format_empty() {
    let expr = parse_expr("$''");
    let segs = segments(&expr);
    assert!(segs.is_empty());
}

#[test]
fn format_single_interpolation() {
    let expr = parse_expr("$'hello, ${name}'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 2);
    assert_literal_seg(&segs[0], b"hello, ");
    assert_interp_name(&segs[1], "name");
}

#[test]
fn format_interpolation_only() {
    let expr = parse_expr("$'${x}'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    assert_interp_name(&segs[0], "x");
}

#[test]
fn format_multiple_interpolations() {
    let expr = parse_expr("$'${a} and ${b}'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 3);
    assert_interp_name(&segs[0], "a");
    assert_literal_seg(&segs[1], b" and ");
    assert_interp_name(&segs[2], "b");
}

#[test]
fn format_adjacent_interpolations() {
    let expr = parse_expr("$'${a}${b}${c}'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 3);
    assert_interp_name(&segs[0], "a");
    assert_interp_name(&segs[1], "b");
    assert_interp_name(&segs[2], "c");
}

#[test]
fn format_trailing_literal() {
    let expr = parse_expr("$'${x}!'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 2);
    assert_interp_name(&segs[0], "x");
    assert_literal_seg(&segs[1], b"!");
}

// -- Double quote format strings --

#[test]
fn format_double_quote() {
    let expr = parse_expr("$\"hello, ${name}\"");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 2);
    assert_literal_seg(&segs[0], b"hello, ");
    assert_interp_name(&segs[1], "name");
}

// -- Expression interpolation --

#[test]
fn format_expression_interpolation() {
    let expr = parse_expr("$'result: ${1 + 2}'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 2);
    assert_literal_seg(&segs[0], b"result: ");
    match &segs[1] {
        FormatSegment::Interpolation(expr) => {
            assert!(matches!(&expr.node, Expr::BinOp { op: Spanned { node: BinOp::Add, .. }, .. }));
        }
        other => panic!("expected Interpolation, got {other:?}"),
    }
}

#[test]
fn format_call_in_interpolation() {
    let expr = parse_expr("$'upper: ${to_upper(name)}'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 2);
    assert_literal_seg(&segs[0], b"upper: ");
    match &segs[1] {
        FormatSegment::Interpolation(expr) => {
            assert!(matches!(&expr.node, Expr::Call { .. }));
        }
        other => panic!("expected Interpolation, got {other:?}"),
    }
}

// -- Escape sequences --

#[test]
fn format_escape_dollar() {
    // \$ prevents interpolation
    let expr = parse_expr("$'literal: \\${name}'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    assert_literal_seg(&segs[0], b"literal: ${name}");
}

#[test]
fn format_bare_dollar() {
    // $ not followed by { is literal
    let expr = parse_expr("$'price: $5'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    assert_literal_seg(&segs[0], b"price: $5");
}

#[test]
fn format_escape_newline() {
    let expr = parse_expr("$'line1\\nline2'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    assert_literal_seg(&segs[0], b"line1\nline2");
}

#[test]
fn format_escape_backslash() {
    let expr = parse_expr("$'back\\\\slash'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    assert_literal_seg(&segs[0], b"back\\slash");
}

#[test]
fn format_escape_tab() {
    let expr = parse_expr("$'tab\\there'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    assert_literal_seg(&segs[0], b"tab\there");
}

#[test]
fn format_escape_hex() {
    let expr = parse_expr("$'byte\\x0a'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    assert_literal_seg(&segs[0], b"byte\x0a");
}

#[test]
fn format_escape_single_quote_in_single() {
    let expr = parse_expr("$'it\\'s'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    assert_literal_seg(&segs[0], b"it's");
}

#[test]
fn format_escape_double_quote_in_double() {
    let expr = parse_expr("$\"say \\\"hi\\\"\"");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    assert_literal_seg(&segs[0], b"say \"hi\"");
}

// -- Escape errors --

#[test]
fn error_format_invalid_escape() {
    let err = parse_err("$'bad\\q'");
    assert!(err.contains("invalid escape"), "error was: {err}");
}

#[test]
fn error_format_single_quote_escape_in_double() {
    let err = parse_err("$\"can\\'t\"");
    assert!(err.contains("invalid escape"), "error was: {err}");
}

#[test]
fn error_format_double_quote_escape_in_single() {
    let err = parse_err("$'say \\\"hi\\\"'");
    assert!(err.contains("invalid escape"), "error was: {err}");
}

// -- Interpolation span correctness --

#[test]
fn interpolation_span_simple() {
    // $'hello ${name}' — name starts at byte 10, ends at 14
    let expr = parse_expr("$'hello ${name}'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 2);
    match &segs[1] {
        FormatSegment::Interpolation(expr) => {
            assert_eq!(expr.span.start, 10);
            assert_eq!(expr.span.end, 14);
        }
        other => panic!("expected Interpolation, got {other:?}"),
    }
}

#[test]
fn interpolation_span_with_expression() {
    // $'x=${1 + 2}' — `1` at byte 6, BinOp ends at byte 11
    let expr = parse_expr("$'x=${1 + 2}'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 2);
    match &segs[1] {
        FormatSegment::Interpolation(expr) => {
            assert_eq!(expr.span.start, 6);
            assert_eq!(expr.span.end, 11);
        }
        other => panic!("expected Interpolation, got {other:?}"),
    }
}

#[test]
fn interpolation_span_second_interp() {
    // $'${a} ${b}' — `a` at byte 4, `b` at byte 9
    let expr = parse_expr("$'${a} ${b}'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 3);
    match &segs[0] {
        FormatSegment::Interpolation(expr) => {
            assert_eq!(expr.span.start, 4);
            assert_eq!(expr.span.end, 5);
        }
        other => panic!("expected Interpolation, got {other:?}"),
    }
    match &segs[2] {
        FormatSegment::Interpolation(expr) => {
            assert_eq!(expr.span.start, 9);
            assert_eq!(expr.span.end, 10);
        }
        other => panic!("expected Interpolation, got {other:?}"),
    }
}

#[test]
fn interpolation_span_double_quote() {
    // $"hi ${x}" — `x` at byte 7
    let expr = parse_expr("$\"hi ${x}\"");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 2);
    match &segs[1] {
        FormatSegment::Interpolation(expr) => {
            assert_eq!(expr.span.start, 7);
            assert_eq!(expr.span.end, 8);
        }
        other => panic!("expected Interpolation, got {other:?}"),
    }
}

// -- Complex interpolation content --

#[test]
fn format_nested_call_in_interpolation() {
    // $'${f(g(1))}' — nested calls
    let expr = parse_expr("$'${f(g(1))}'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    match &segs[0] {
        FormatSegment::Interpolation(expr) => {
            assert!(matches!(&expr.node, Expr::Call { .. }));
        }
        other => panic!("expected Interpolation, got {other:?}"),
    }
}

#[test]
fn format_comparison_in_interpolation() {
    let expr = parse_expr("$'${a == b}'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    match &segs[0] {
        FormatSegment::Interpolation(expr) => {
            assert!(matches!(&expr.node, Expr::BinOp { op: Spanned { node: BinOp::Eq, .. }, .. }));
        }
        other => panic!("expected Interpolation, got {other:?}"),
    }
}

#[test]
fn format_only_literal_dollar() {
    // $'$$$' — three bare dollars
    let expr = parse_expr("$'$$$'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    assert_literal_seg(&segs[0], b"$$$");
}

#[test]
fn format_dollar_before_non_brace() {
    // $'$x $y $z' — bare dollars followed by non-brace chars
    let expr = parse_expr("$'$x $y $z'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 1);
    assert_literal_seg(&segs[0], b"$x $y $z");
}

#[test]
fn format_escape_then_interpolation() {
    // $'\n${x}' — escape followed by interpolation
    let expr = parse_expr("$'\\n${x}'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 2);
    assert_literal_seg(&segs[0], b"\n");
    assert_interp_name(&segs[1], "x");
}

#[test]
fn format_interpolation_then_escape() {
    // $'${x}\n' — interpolation followed by escape
    let expr = parse_expr("$'${x}\\n'");
    let segs = segments(&expr);
    assert_eq!(segs.len(), 2);
    assert_interp_name(&segs[0], "x");
    assert_literal_seg(&segs[1], b"\n");
}

// -- Format strings in expressions --

#[test]
fn format_string_concatenation() {
    let expr = parse_expr("$'hello' + $' world'");
    assert!(matches!(&expr.node, Expr::BinOp { op: Spanned { node: BinOp::Add, .. }, .. }));
}

#[test]
fn format_string_in_call() {
    let expr = parse_expr("print($'value: ${x}')");
    assert!(matches!(&expr.node, Expr::Call { .. }));
}

#[test]
fn format_string_in_def() {
    let program = parse("def msg = $'hello ${name}'");
    assert_eq!(program.statements.len(), 1);
    match &program.statements[0].node {
        Statement::Def { expr, .. } => {
            assert!(matches!(&expr.node, Expr::FormatString(_)));
        }
        other => panic!("expected Def, got {other:?}"),
    }
}

// -- Error cases --

#[test]
fn error_format_incomplete_hex() {
    let err = parse_err("$'\\x0'");
    assert!(err.contains("\\x escape"), "error was: {err}");
}

#[test]
fn error_points_at_format_string() {
    // Errors should reference the format String literal
    let err = parse_err("$'bad\\q'");
    assert!(err.contains("format String"), "error was: {err}");
}

// -- Interpolation diagnostics --
// Errors from inside `${ ... }` must render against the whole source in
// whole-source coordinates, keep their labels, and not double up the diagram.

#[test]
fn interpolation_error_points_into_the_string() {
    // The failure (running out of input mid-expression) sits inside the
    // interpolation, past the opening `$'` at offset 0 -- not at a
    // substring-relative offset near the start.
    let err = frost_parse::parse_program("test.frst", "$'${x +}'").unwrap_err();
    assert!(
        err.primary_span().start >= 2,
        "primary span should point inside the interpolation, was {:?}",
        err.primary_span()
    );
}

#[test]
fn interpolation_leftover_token_points_at_token() {
    // `x y` parses `x`, then `y` is unexpected. The label must land on `y`.
    let src = "$'${x y}'";
    let err = frost_parse::parse_program("test.frst", src).unwrap_err();
    let y = src.find('y').unwrap();
    assert_eq!(
        err.primary_span().start,
        y,
        "expected the leftover-token label at the `y`; error: {err}"
    );
}

#[test]
fn interpolation_error_labels_the_format_string() {
    let err = frost_parse::parse_program("test.frst", "$'${x +}'").unwrap_err();
    assert!(
        err.labels().iter().any(|l| l.text == "in this format String"),
        "expected a context label; labels: {:?}",
        err.labels()
    );
}

#[test]
fn interpolation_error_renders_a_single_diagram() {
    // The old code flattened an inner rendered report into the outer message,
    // producing two source diagrams and an "in interpolation:" prefix.
    let rendered = frost_parse::parse_program("test.frst", "$'${x +}'")
        .unwrap_err()
        .to_string();
    assert_eq!(
        rendered.matches("╭─[").count(),
        1,
        "expected exactly one source diagram; rendered:\n{rendered}"
    );
    assert!(
        !rendered.contains("in interpolation:"),
        "inner error should not be flattened into the message; rendered:\n{rendered}"
    );
}
