//! A hand-built tour of the diagnostic API.
//!
//! This is a demonstration, not a behavioral spec: it constructs one
//! [`CompilerErrors`] set that exercises every builder and every consumption
//! path, then prints it both ways. To see the two renderings:
//!
//! ```text
//! cargo test -p frost-compile --lib error::demo -- --nocapture
//! ```
//!
//! The `plain` block is escape-free ASCII (what a snapshot test would assert
//! against); the `pretty` block carries ANSI color and unicode box-drawing (what
//! a user sees at a terminal).
//!
//! The Frost snippets are illustrative: they show where labels land, not exact
//! grammar.

use miette::Severity;

use super::{CompilerError, CompilerErrors};
use frost_parse::ast::SourceSpan;

/// The span of the `occurrence`-th (0-based) match of `needle` in `src`.
/// Computing spans by search keeps the demo readable and free of hand-counted
/// byte offsets that rot the moment a snippet is edited.
fn span(src: &str, needle: &str, occurrence: usize) -> SourceSpan {
    let start = src
        .match_indices(needle)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("`{needle}` (occurrence {occurrence}) not found in snippet"))
        .0;
    SourceSpan {
        start,
        end: start + needle.len(),
    }
}

/// Build the demonstration set: three diagnostics that between them use every
/// part of the API.
fn demo_errors() -> CompilerErrors {
    let main = r"def {foo} = import('other')
def x = 1
def x = 2
def y = 3
print(foo)
";
    let other = "def foo = 1";

    let mut errors = CompilerErrors::new();

    // 1. A hard error with a same-file related note. The related diagnostic
    //    omits its own source, so it inherits `main.frst` from the parent.
    errors.push(
        CompilerError::error("`x` is already bound".to_string())
            .code("frost::compile::duplicate_binding".to_string())
            .source("main.frst".to_string(), main.to_string())
            .label_primary(span(main, "x", 1), "`x` redefined here".to_string())
            .help("bindings are immutable; choose a different name".to_string())
            .related(
                CompilerError::advice("the first binding of `x`".to_string())
                    .label(span(main, "x", 0), "originally bound here".to_string()),
            ),
    );

    // 2. A warning: one label, a code and a help, no related block.
    errors.push(
        CompilerError::warning("`y` is never used".to_string())
            .code("frost::compile::unused_binding".to_string())
            .source("main.frst".to_string(), main.to_string())
            .label(span(main, "y", 0), "bound but never read".to_string())
            .help("remove the binding if it is not needed".to_string()),
    );

    // 3. A hard error whose related note lives in a *different* file: the
    //    related diagnostic carries its own `source`, so it renders its own
    //    snippet from `other.frst`.
    errors.push(
        CompilerError::new(
            Severity::Error,
            "`foo` is not exported by module `other`".to_string(),
        )
        .code("frost::compile::unexported_import".to_string())
        .source("main.frst".to_string(), main.to_string())
        .label_primary(span(main, "foo", 0), "required by this destructuring".to_string())
        .help("add `export` to the definition in `other`".to_string())
        .related(
            CompilerError::advice("`foo` is defined here, but not exported".to_string())
                .source("other.frst".to_string(), other.to_string())
                .label(span(other, "foo", 0), "add `export` before this".to_string()),
        ),
    );

    errors
}

#[test]
fn every_part_of_the_api() {
    let errors = demo_errors();

    // -- CompilerErrors inspection --
    assert!(!errors.is_empty(), "the set holds diagnostics");
    assert_eq!(errors.len(), 3, "three top-level diagnostics");
    assert_eq!(errors.iter().count(), 3, "iter walks all three");

    // -- Single-diagnostic rendering paths, on the first diagnostic --
    let first = errors.iter().next().expect("at least one diagnostic");
    let plain = first.render_plain();
    assert!(plain.contains("already bound"), "headline present");
    assert!(plain.contains("originally bound here"), "related block present");
    // `render` (auto) and `Display` agree.
    assert_eq!(first.render(), first.to_string(), "Display delegates to render");
    // `render_pretty` carries color; `render_plain` does not.
    assert!(
        first.render_pretty().contains('\u{1b}'),
        "pretty rendering contains ANSI escapes"
    );
    assert!(
        !plain.contains('\u{1b}'),
        "plain rendering is escape-free"
    );

    // -- Whole-set rendering, and the plural render wrappers --
    let plain_all = errors.render_plain();
    for needle in [
        "already bound",              // diagnostic 1
        "is never used",              // diagnostic 2
        "not exported",               // diagnostic 3
        "add `export` before this",   // cross-file related snippet
        "other.frst",                 // the related block's own source
    ] {
        assert!(plain_all.contains(needle), "plain set is missing: {needle}");
    }
    assert_eq!(errors.render(), errors.to_string(), "set Display delegates to render");

    // -- IntoIterator consumes the set by value --
    let severities_seen = errors.into_iter().count();
    assert_eq!(severities_seen, 3, "IntoIterator yields every diagnostic");

    // -- Print both renderings for the human reviewer (visible with --nocapture) --
    let errors = demo_errors();
    println!("\n===== render_plain (ascii, monochrome) =====\n");
    println!("{}", errors.render_plain());
    println!("\n===== render_pretty (unicode, colored) =====\n");
    println!("{}", errors.render_pretty());
}
