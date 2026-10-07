//! The render styles of `ParseError`: each says the same thing. The graphical
//! styles differ only in the characters they may use; the narrated style draws
//! nothing.

use frostlang_parse::{ParseError, parse_program};

/// The error for a script with a dangling operator, which every style shows.
fn error() -> ParseError {
    parse_program("script.frst", "dangling + ").expect_err("the script is incomplete")
}

/// Whether `text` holds a terminal escape, as color does.
fn has_escape(text: &str) -> bool {
    text.contains('\x1b')
}

#[test]
fn every_style_names_the_file_and_shows_the_snippet_and_label() {
    let error = error();
    let label = &error.labels()[0].text;
    for (style, rendered) in [
        ("narrated", error.render_narrated()),
        ("plain", error.render_plain()),
        ("unicode", error.render_unicode()),
        ("pretty", error.render_pretty()),
        ("adaptive", error.render()),
    ] {
        assert!(rendered.contains("script.frst"), "{style}:\n{rendered}");
        assert!(rendered.contains("dangling +"), "{style}:\n{rendered}");
        assert!(rendered.contains(error.message()), "{style}:\n{rendered}");
        assert!(rendered.contains(label.as_str()), "{style}:\n{rendered}");
    }
}

#[test]
fn every_style_shows_the_help() {
    let error = parse_program("script.frst", "let x = 5").expect_err("`let` is not Frost");
    let help = error
        .help()
        .expect("a habit from another language earns help");
    for (style, rendered) in [
        ("narrated", error.render_narrated()),
        ("plain", error.render_plain()),
        ("unicode", error.render_unicode()),
        ("pretty", error.render_pretty()),
    ] {
        assert!(rendered.contains(help), "{style}:\n{rendered}");
    }
}

#[test]
fn narrated_draws_nothing() {
    let rendered = error().render_narrated();
    assert!(!has_escape(&rendered), "{rendered:?}");
    for drawing in ["|", "^", "`--", "\u{256d}", "\u{2502}"] {
        assert!(!rendered.contains(drawing), "no `{drawing}`:\n{rendered}");
    }
}

#[test]
fn narrated_locates_each_label_by_line_and_column() {
    let error =
        parse_program("script.frst", "def x = 1\nlet y = x +").expect_err("`let` is not Frost");
    let rendered = error.render_narrated();
    assert!(
        rendered.contains("label at line 2, columns 1 to 3: this is a complete statement"),
        "{rendered}"
    );
    assert!(
        rendered.contains("label at line 2, column 5: unexpected"),
        "{rendered}"
    );
}

#[test]
fn display_is_the_adaptive_render() {
    let error = error();
    assert_eq!(error.to_string(), error.render());
}

#[test]
fn plain_is_ascii_without_color() {
    let rendered = error().render_plain();
    assert!(rendered.is_ascii(), "{rendered}");
    assert!(!has_escape(&rendered), "{rendered:?}");
}

#[test]
fn unicode_draws_with_unicode_but_has_no_color() {
    let rendered = error().render_unicode();
    assert!(!rendered.is_ascii(), "box-drawing expected:\n{rendered}");
    assert!(!has_escape(&rendered), "{rendered:?}");
}

#[test]
fn pretty_draws_with_unicode_and_color() {
    let rendered = error().render_pretty();
    assert!(!rendered.is_ascii(), "box-drawing expected:\n{rendered}");
    assert!(has_escape(&rendered), "color expected:\n{rendered:?}");
}
