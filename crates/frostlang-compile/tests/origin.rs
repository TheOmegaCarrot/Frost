//! Every function a compilation produces carries the base name of the file it
//! was compiled from as its `origin`.

use frostlang_compile::{CompilerOptions, OptimizationOptions, compile_program};
use frostlang_runtime::CompiledFunction;

/// The top-level function `source` compiles to as `filename`.
fn compiled(filename: &str, source: &str) -> CompiledFunction {
    let options = CompilerOptions::new().with_optimization(OptimizationOptions::NONE);
    compile_program(filename, source, options)
        .unwrap_or_else(|errors| panic!("compiles:\n{}", errors.render_plain()))
        .code
        .into_closure()
        .expect("captures nothing")
        .inner_fn()
        .clone()
}

#[test]
fn the_origin_is_the_base_name_of_the_filename() {
    let top = compiled("some/dir/util.frst", "1");
    assert_eq!(top.origin.as_deref(), Some("util.frst"));
}

#[test]
fn a_bare_filename_is_its_own_origin() {
    let top = compiled("util.frst", "1");
    assert_eq!(top.origin.as_deref(), Some("util.frst"));
}

#[test]
fn every_nested_function_shares_the_origin() {
    let source = r"
        export defn outer() -> fn x -> fn y -> x + y
    ";
    let top = compiled("lib/nested.frst", source);
    let outer = &top.child_fns[0];
    let middle = &outer.child_fns[0];
    let inner = &middle.child_fns[0];
    for function in [&top, outer.as_ref(), middle.as_ref(), inner.as_ref()] {
        assert_eq!(
            function.origin.as_deref(),
            Some("nested.frst"),
            "`{}` carries the compilation's origin",
            function.name
        );
    }
}

#[test]
fn a_filename_with_no_base_name_gives_no_origin() {
    for filename in ["", "/", ".."] {
        let top = compiled(filename, "1");
        assert_eq!(top.origin, None, "filename {filename:?}");
    }
}
