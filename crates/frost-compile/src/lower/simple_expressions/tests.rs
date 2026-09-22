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
