//! `compile_in_scope` and `implicit_export`, the two pieces an embedder (a REPL)
//! needs: a top-level compiled against an enclosing scope captures the names it
//! uses from that scope, and implicit export lets the caller harvest every
//! top-level binding afterward.
//!
//! Expressions are kept to name lookups and defs, since arithmetic lowering is
//! not implemented yet; the concern here is capture and export wiring, not
//! evaluation.

use std::collections::BTreeMap;

use frost_compile::{CompilerOptions, OptimizationOptions, compile_in_scope};
use frost_runtime::{ProgramResult, Value, Vm};

fn options(implicit_export: bool) -> CompilerOptions {
    CompilerOptions {
        optimization_options: OptimizationOptions {
            constant_fold: false,
            constant_propagate: false,
        },
        implicit_export,
    }
}

/// Compile `script` against `outer_scope`, close it with `captures`, and run it.
fn run_in_scope(
    script: &str,
    outer_scope: &[&str],
    captures: Vec<(&str, Value)>,
    implicit_export: bool,
) -> ProgramResult {
    let output = compile_in_scope("repl.frst", script, options(implicit_export), outer_scope)
        .expect("source should compile");
    let map: BTreeMap<String, Value> = captures
        .into_iter()
        .map(|(name, value)| (name.to_string(), value))
        .collect();
    let closure = output.code.close(map).expect("captures should close");
    Vm::factory()
        .build(closure)
        .expect("closure builds")
        .run()
        .map_err(frost_runtime::RunError::into_error)
        .expect("program should run")
}

/// The exported bindings of a result, as a map.
fn exports(result: &ProgramResult) -> BTreeMap<String, Value> {
    result
        .exports()
        .map(|(name, value)| (name.to_string(), value.clone()))
        .collect()
}

#[test]
fn an_outer_binding_is_captured() {
    // `x` is free and named in the outer scope, so it becomes a capture supplied
    // at close time.
    let result = run_in_scope("x", &["x"], vec![("x", Value::Int(5))], false);
    assert_eq!(result.tail(), &Value::Int(5));
}

#[test]
fn a_captured_name_shadows_a_global() {
    // `len` is a global, but here it is also an outer-scope binding, so the
    // capture wins: the lookup yields the captured value, not the builtin.
    let result = run_in_scope("len", &["len"], vec![("len", Value::Int(42))], false);
    assert_eq!(
        result.tail(),
        &Value::Int(42),
        "the captured `len` shadows the global function"
    );
}

#[test]
fn an_unused_outer_name_is_not_captured() {
    // The script never mentions `x`, so it is not captured: closing with no
    // captures still succeeds (a spurious capture would make close fail).
    let result = run_in_scope("42", &["x"], vec![], false);
    assert_eq!(result.tail(), &Value::Int(42));
}

#[test]
fn implicit_export_exports_every_top_level_binding() {
    // `def y = x` is not written `export`, but implicit export makes it harvestable.
    let result = run_in_scope("def y = x\ny", &["x"], vec![("x", Value::Int(5))], true);
    assert_eq!(result.tail(), &Value::Int(5));
    assert_eq!(
        exports(&result).get("y"),
        Some(&Value::Int(5)),
        "the top-level binding is exported"
    );
}

#[test]
fn without_implicit_export_a_plain_def_is_not_exported() {
    // The same program, implicit export off: `y` runs but is not harvested.
    let result = run_in_scope("def y = x\ny", &["x"], vec![("x", Value::Int(5))], false);
    assert_eq!(result.tail(), &Value::Int(5));
    assert!(
        exports(&result).is_empty(),
        "a plain def is not exported without the option"
    );
}

#[test]
fn a_free_name_absent_from_the_outer_scope_is_a_compile_error() {
    // Neither a local, nor an outer-scope name, nor a global: an unbound name.
    let errors = compile_in_scope("repl.frst", "nope", options(false), &[])
        .expect_err("an unbound free name should not compile");
    assert!(
        errors.render_plain().contains("nope"),
        "the diagnostic names the offending identifier:\n{}",
        errors.render_plain()
    );
}
