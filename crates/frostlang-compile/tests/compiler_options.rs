//! Building `CompilerOptions`: the starting point and each `with_*` method.

use frostlang_compile::{CompilerOptions, Optimization, OptimizationOptions};

#[test]
fn new_turns_every_optimization_on_and_exports_nothing_implicitly() {
    let options = CompilerOptions::new();
    assert_eq!(options.optimization_options, OptimizationOptions::ALL);
    assert!(!options.implicit_export);
}

#[test]
fn default_is_new() {
    assert_eq!(CompilerOptions::default(), CompilerOptions::new());
}

#[test]
fn with_optimization_sets_only_the_optimizations() {
    let fold_only = OptimizationOptions::NONE.with(Optimization::ConstantFold, true);
    let options = CompilerOptions::new()
        .with_implicit_export(true)
        .with_optimization(fold_only);
    assert_eq!(options.optimization_options, fold_only);
    assert!(options.implicit_export, "implicit export is left as it was");
}

#[test]
fn with_implicit_export_sets_only_implicit_export() {
    let options = CompilerOptions::new()
        .with_optimization(OptimizationOptions::NONE)
        .with_implicit_export(true);
    assert!(options.implicit_export);
    assert_eq!(
        options.optimization_options,
        OptimizationOptions::NONE,
        "the optimizations are left as they were"
    );
    assert!(!options.with_implicit_export(false).implicit_export);
}

#[test]
fn options_can_be_built_in_a_constant() {
    const OPTIONS: CompilerOptions = CompilerOptions::new()
        .with_optimization(OptimizationOptions::NONE)
        .with_implicit_export(true);
    let built_at_runtime = CompilerOptions::default()
        .with_optimization(OptimizationOptions::NONE)
        .with_implicit_export(true);
    assert_eq!(OPTIONS, built_at_runtime);
}
