//! Tests for name-lookup lowering, specifically the fold-eligibility a global
//! reference carries. A pure global is foldable (so a call to it can fold); an
//! impure one is not. Local and constant-propagation lookups are covered end to
//! end in `tests/constant_propagation.rs`.

use frost_parse::ast::SourceSpan;
use frost_runtime::Arity;

use crate::lower::locals::Locals;
use crate::lower::{FunctionBuilder, Label};
use crate::{CompilerOptions, OptimizationOptions};

fn options() -> CompilerOptions {
    CompilerOptions {
        optimization_options: OptimizationOptions {
            constant_fold: false,
            constant_propagate: false,
            branch_eliminate: false,
            capture_hoist: false,
        },
        implicit_export: false,
    }
}

fn builder(options: &CompilerOptions) -> FunctionBuilder<'_> {
    // A name lookup touches neither the source nor the fold VM.
    FunctionBuilder {
        locals: Locals::new(),
        next_label: Label(0),
        name: "<test>".to_string(),
        arity: Arity::Exact(0),
        source: "",
        filename: "",
        options,
        fold_vm: None,
        top_level: false,
        effectful: false,
    }
}

#[test]
fn a_pure_global_lookup_is_foldable() {
    let options = options();
    let fragment = builder(&options)
        .compile_name_lookup("transform", SourceSpan::default())
        .expect("`transform` is a global");
    assert!(fragment.foldable, "a pure global is fold-eligible");
}

#[test]
fn an_impure_global_lookup_is_not_foldable() {
    let options = options();
    let fragment = builder(&options)
        .compile_name_lookup("print", SourceSpan::default())
        .expect("`print` is a global");
    assert!(
        !fragment.foldable,
        "an impure global must never be fold-eligible"
    );
}

#[test]
fn loading_an_impure_global_makes_the_function_effectful() {
    let options = options();
    let mut builder = builder(&options);
    assert!(!builder.effectful, "a fresh function is not effectful");

    builder
        .compile_name_lookup("transform", SourceSpan::default())
        .expect("`transform` is a global");
    assert!(!builder.effectful, "a pure global adds no effect");

    builder
        .compile_name_lookup("print", SourceSpan::default())
        .expect("`print` is a global");
    assert!(builder.effectful, "an impure global is an effect");

    builder
        .compile_name_lookup("transform", SourceSpan::default())
        .expect("`transform` is a global");
    assert!(builder.effectful, "once effectful, a function stays so");
}

#[test]
fn an_unbound_name_lookup_reports_its_canonical_form() {
    // `$` is only ever produced by the parser inside an abbreviated lambda,
    // where it is always defined as a parameter; an unbound `$` is unreachable
    // through real source, so this drives `compile_name_lookup` directly to
    // pin that the error names the canonical `$1`, not the literal `$`.
    let options = options();
    let mut builder = builder(&options);
    let errors = builder
        .compile_name_lookup("$", SourceSpan::default())
        .expect_err("nothing defines `$1` here");
    let rendered = errors.render_plain();
    assert!(
        rendered.contains("`$1` is not defined"),
        "the error names the canonical form: {rendered}"
    );
}

#[test]
fn every_impure_global_makes_the_function_effectful() {
    // The effect sources: output, mutable state, and imports.
    let options = options();
    for name in ["print", "mprint", "mutable_cell", "import", "imported"] {
        let mut builder = builder(&options);
        builder
            .compile_name_lookup(name, SourceSpan::default())
            .unwrap_or_else(|_| panic!("`{name}` is a global"));
        assert!(builder.effectful, "`{name}` is an effect");
    }
}
