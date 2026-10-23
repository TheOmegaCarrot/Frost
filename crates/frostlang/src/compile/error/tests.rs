//! The diagnostics of one compilation share one copy of its source. Which copy
//! a diagnostic holds is not observable through the public API, so these tests
//! read it directly.

use std::sync::Arc;

use crate::compile::{CompilerOptions, Diagnostic, compile_program};

/// The source text `diagnostic` holds.
fn source_of(diagnostic: &Diagnostic) -> &Arc<str> {
    diagnostic
        .0
        .source_code
        .as_ref()
        .expect("a compile diagnostic carries its source")
        .inner()
}

#[test]
fn every_lint_of_a_compilation_shares_one_copy_of_its_source() {
    // Lints from the top level, from a block, and from nested functions, which
    // compile in builders of their own.
    let source = r"
export def f = fn x -> {
    def unused = 1
    fn y -> {
        def also_unused = 2
        y
    }
}
export def g = do {
    def never_read = 3
    4
}
export def h = match 1 {
    String => String,
}
";
    let output = compile_program("lints.frst", source, CompilerOptions::default())
        .expect("the script compiles");
    let lints: Vec<&Diagnostic> = output.lints.iter().collect();
    assert_eq!(lints.len(), 4, "one lint per finding: {lints:?}");

    let first = source_of(lints[0]);
    assert_eq!(&**first, source, "the shared copy is the source");
    for lint in &lints[1..] {
        assert!(
            Arc::ptr_eq(first, source_of(lint)),
            "every lint holds the same copy: {lint:?}"
        );
    }
}

#[test]
fn a_compile_error_holds_the_source() {
    let source = r"
def x = 1
undefined_name
";
    let errors = compile_program("error.frst", source, CompilerOptions::default())
        .expect_err("`undefined_name` is not defined");
    let error = errors.iter().next().expect("one error");
    assert_eq!(&**source_of(error), source);
}

#[test]
fn a_parse_error_holds_the_source() {
    let source = r"
def = 1
";
    let errors = compile_program("parse.frst", source, CompilerOptions::default())
        .expect_err("`def` needs a pattern");
    let error = errors.iter().next().expect("one error");
    assert_eq!(&**source_of(error), source);
}
