//! Building `CompilerOptions`: the starting point and each `with_*` method.

use std::num::NonZeroU64;

use frostlang::compile::{
    CompilerOptions, Optimization, OptimizationOptions, ParseLimits, compile_program,
};

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
fn new_turns_lints_on() {
    assert!(CompilerOptions::new().lints);
    assert!(CompilerOptions::default().lints, "Default agrees");
}

#[test]
fn with_lints_sets_only_lints() {
    let fold_only = OptimizationOptions::NONE.with(Optimization::ConstantFold, true);
    let options = CompilerOptions::new()
        .with_optimization(fold_only)
        .with_implicit_export(true)
        .with_lints(false);
    assert!(!options.lints);
    assert_eq!(
        options.optimization_options, fold_only,
        "the optimizations are left as they were"
    );
    assert!(options.implicit_export, "implicit export is left as it was");
    assert!(
        options.with_lints(true).lints,
        "lints can be turned back on"
    );
}

#[test]
fn with_lints_leaves_the_other_options_when_they_are_set_afterward() {
    let options = CompilerOptions::new()
        .with_lints(false)
        .with_implicit_export(true)
        .with_optimization(OptimizationOptions::NONE);
    assert!(!options.lints, "setting other options keeps lints off");
}

#[test]
fn lints_can_be_set_in_a_constant() {
    const OPTIONS: CompilerOptions = CompilerOptions::new().with_lints(false);
    let lints = OPTIONS.lints;
    assert!(!lints, "lints are off");
    assert_eq!(OPTIONS, CompilerOptions::default().with_lints(false));
    assert_ne!(
        OPTIONS,
        CompilerOptions::new(),
        "lints take part in equality"
    );
}

#[test]
fn new_has_the_default_parse_limits() {
    assert_eq!(CompilerOptions::new().parse_limits, ParseLimits::new());
    assert_eq!(CompilerOptions::default().parse_limits, ParseLimits::new());
}

#[test]
fn with_parse_limits_sets_only_the_parse_limits() {
    let options = CompilerOptions::new()
        .with_optimization(OptimizationOptions::NONE)
        .with_implicit_export(true)
        .with_lints(false)
        .with_parse_limits(ParseLimits::UNLIMITED);
    assert_eq!(options.parse_limits, ParseLimits::UNLIMITED);
    assert_eq!(
        options.optimization_options,
        OptimizationOptions::NONE,
        "the optimizations are left as they were"
    );
    assert!(options.implicit_export, "implicit export is left as it was");
    assert!(!options.lints, "lints are left as they were");
    assert_ne!(
        options,
        options.with_parse_limits(ParseLimits::new()),
        "parse limits take part in equality"
    );
}

#[test]
fn compiling_applies_the_parse_limits() {
    let three_levels = ParseLimits::UNLIMITED.with_max_recursion_depth(NonZeroU64::new(3));
    let options = CompilerOptions::new().with_parse_limits(three_levels);
    assert!(compile_program("test.frst", "[[1]]", options).is_ok());
    let errors = compile_program("test.frst", "[[[1]]]", options).expect_err("past the limit");
    let rendered = errors.render_narrated();
    assert!(
        rendered.contains("nested too deeply: the limit is 3 levels"),
        "{rendered}"
    );

    let three_deep = ParseLimits::UNLIMITED.with_max_ast_depth(NonZeroU64::new(3));
    let options = CompilerOptions::new().with_parse_limits(three_deep);
    assert!(compile_program("test.frst", "1 + 1 + 1", options).is_ok());
    let errors =
        compile_program("test.frst", "1 + 1 + 1 + 1", options).expect_err("past the limit");
    let rendered = errors.render_narrated();
    assert!(
        rendered.contains("expression too deep: the limit is 3 levels"),
        "{rendered}"
    );
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
