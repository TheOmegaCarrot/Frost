//! The compiler parses its own source, so a parse failure surfaces through the
//! compiler's error channel: the parser's diagnostic is lifted into a
//! `Diagnostics` and rendered with the compiler's own formatting.
//!
//! These drive that path black-box through the public compile API. They assert
//! structural facts (one diagnostic, filename attached, the offending snippet
//! shown, the label preserved) rather than the parser's exact wording, which is
//! free to change.

mod common;

use common::Script;
use frostlang_compile::Diagnostics;

/// Compile `source`, expecting a failure, and return the diagnostics.
fn errors(source: &str) -> Diagnostics {
    Script::new(source).filename("script.frst").compile_errors()
}

#[test]
fn parse_failure_is_a_single_diagnostic() {
    // The parser stops at the first error, so exactly one diagnostic surfaces.
    assert_eq!(errors("1 +").len(), 1);
}

#[test]
fn parse_error_attaches_filename_and_snippet() {
    let rendered = errors("1 +").render_plain();
    assert!(
        rendered.contains("script.frst"),
        "the filename is attached so miette can frame the snippet:\n{rendered}"
    );
    assert!(
        rendered.contains("1 +"),
        "the offending source line is shown:\n{rendered}"
    );
    assert!(
        !rendered.contains('\u{1b}'),
        "render_plain is escape-free:\n{rendered}"
    );
}

#[test]
fn parse_error_keeps_its_message() {
    // Whatever the parser said, the lift carries it through verbatim as the
    // diagnostic headline (rendered on the `x` line).
    let rendered = errors("1 +").render_plain();
    assert!(
        rendered.contains("expected an expression, but found the end of input"),
        "the parser's message is preserved:\n{rendered}"
    );
}

#[test]
fn an_interpolation_error_renders_a_single_diagram() {
    // The error inside `${ ... }` labels the whole source once, not as a
    // diagram of its own nested inside the outer one.
    let rendered = errors("$'${x +}'").render_pretty();
    assert_eq!(
        rendered.matches("╭─[").count(),
        1,
        "expected exactly one source diagram; rendered:\n{rendered}"
    );
}

#[test]
fn parse_error_keeps_its_help() {
    let rendered = errors("let x = 5").render_plain();
    assert!(
        rendered.contains("help: Frost has no `let`; bind a name with `def`: `def x = ...`"),
        "the parser's help is preserved:\n{rendered}"
    );
}

#[test]
fn parse_error_preserves_the_labeled_span() {
    // `def 5 = 1`: the parser labels the offending token. The lift keeps that
    // label, so it renders pointing at the `5` in the snippet.
    let rendered = errors("def 5 = 1").render_plain();
    assert!(rendered.contains("def 5 = 1"), "snippet shown:\n{rendered}");
    assert!(
        rendered.contains("unexpected"),
        "the parser's label text is preserved:\n{rendered}"
    );
    assert!(
        rendered.contains("`--"),
        "the label points at a span (an underline is drawn):\n{rendered}"
    );
}

#[test]
fn multiple_labels_survive_the_lift() {
    // Some parser diagnostics attach more than one label; the lift makes the
    // first primary and keeps the rest. This just asserts the render is
    // well-formed and framed for a multi-token error.
    let rendered = errors("def 5 = 1").render_plain();
    assert!(rendered.contains("script.frst"));
    assert!(
        rendered.contains("`----"),
        "the snippet frame closes:\n{rendered}"
    );
}

#[test]
fn an_unterminated_string_is_a_single_diagnostic() {
    // A lexer-level failure (no closing quote) is lifted the same way as a
    // grammar-level one.
    let diagnostics = errors(r#""unterminated"#);
    assert_eq!(diagnostics.len(), 1);
    assert!(
        diagnostics.render_plain().contains("script.frst"),
        "the filename is attached:\n{}",
        diagnostics.render_plain()
    );
}

#[test]
fn a_stray_closing_bracket_is_a_parse_error() {
    let diagnostics = errors("]");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn an_error_on_a_later_line_points_at_that_line() {
    let source = r"
        def x = 1
        1 +
    ";
    let rendered = Script::new(source)
        .filename("script.frst")
        .compile_errors()
        .render_plain();
    assert!(
        rendered.contains("1 +"),
        "the snippet frames the offending line, not the first one:\n{rendered}"
    );
}

#[test]
fn valid_source_compiles() {
    // A well-formed program is not rejected: it compiles and runs.
    assert_eq!(Script::new("42").run(), frostlang_runtime::Value::Int(42));
}
