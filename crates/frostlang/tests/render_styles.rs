//! The fixed render styles of `Diagnostics`: each says the same thing. The
//! graphical styles differ only in the characters they may use; the narrated
//! style draws nothing.

use crate::script;

use frostlang::compile::{Diagnostic, Diagnostics};
use script::Script;

/// Diagnostics for a script with an unbound name, whose snippet every style shows.
fn errors() -> Diagnostics {
    Script::new("undefined_name + 1")
        .filename("script.frst")
        .compile_errors()
}

/// Whether `text` holds a terminal escape, as color does.
fn has_escape(text: &str) -> bool {
    text.contains('\x1b')
}

#[test]
fn every_style_names_the_file_and_shows_the_snippet() {
    let errors = errors();
    for (style, rendered) in [
        ("narrated", errors.render_narrated()),
        ("plain", errors.render_plain()),
        ("unicode", errors.render_unicode()),
        ("pretty", errors.render_pretty()),
    ] {
        assert!(rendered.contains("script.frst"), "{style}:\n{rendered}");
        assert!(rendered.contains("undefined_name"), "{style}:\n{rendered}");
    }
}

#[test]
fn narrated_draws_nothing() {
    let rendered = errors().render_narrated();
    assert!(!has_escape(&rendered), "{rendered:?}");
    for drawing in ["|", "^", "`--", "\u{256d}", "\u{2502}"] {
        assert!(!rendered.contains(drawing), "no `{drawing}`:\n{rendered}");
    }
}

#[test]
fn narrated_includes_related_diagnostics() {
    let source = r"
        def x = 1
        def x = 2
    ";
    let rendered = Script::new(source).compile_errors().render_narrated();
    assert!(rendered.contains("`x` is already bound"), "{rendered}");
    assert!(rendered.contains("`x` was first bound here"), "{rendered}");
}

#[test]
fn every_diagnostic_in_the_set_is_narrated() {
    let errors = errors();
    let each: Vec<String> = errors.iter().map(Diagnostic::render_narrated).collect();
    assert_eq!(errors.render_narrated(), each.join("\n"));
}

#[test]
fn plain_is_ascii_without_color() {
    let rendered = errors().render_plain();
    assert!(rendered.is_ascii(), "{rendered}");
    assert!(!has_escape(&rendered), "{rendered:?}");
}

#[test]
fn unicode_draws_with_unicode_but_has_no_color() {
    let rendered = errors().render_unicode();
    assert!(!rendered.is_ascii(), "box-drawing expected:\n{rendered}");
    assert!(!has_escape(&rendered), "{rendered:?}");
}

#[test]
fn pretty_draws_with_unicode_and_color() {
    let rendered = errors().render_pretty();
    assert!(!rendered.is_ascii(), "box-drawing expected:\n{rendered}");
    assert!(has_escape(&rendered), "color expected:\n{rendered:?}");
}

#[test]
fn unicode_is_pretty_without_its_color() {
    // The two share a layout; only the color escapes differ.
    let errors = errors();
    let pretty = errors.render_pretty();
    let unicode = errors.render_unicode();
    assert_eq!(
        strip_escapes(&pretty),
        unicode,
        "pretty, with its escapes removed, is unicode"
    );
}

/// `text` without its ANSI SGR escapes (`ESC [ ... m`), the only kind color uses.
fn strip_escapes(text: &str) -> String {
    let mut stripped = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            chars.by_ref().find(|&c| c == 'm');
        } else {
            stripped.push(c);
        }
    }
    stripped
}
