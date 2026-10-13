//! `fuel_consumed`: the number of function calls a run made, as a host reads it
//! from a [`ProgramResult`] or a [`RunError`].
//!
//! Optimization may change how many calls a script makes (by folding or
//! inlining one), so every script here compiles with no optimization: the calls
//! counted are exactly the calls the source spells out.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use frostlang::compile::{CompilerOptions, OptimizationOptions, compile_in_scope};
use frostlang::{
    Closure, FrostError, ImportCtx, ImportResolver, Importer, ImporterBuilder, ProgramResult,
    RunError, Value, Vm, frost_fn,
};

/// `source`, compiled with no optimization, with `captures` in scope.
fn compile(source: &str, captures: &[(&str, Value)]) -> Arc<Closure> {
    let options = CompilerOptions::new().with_optimization(OptimizationOptions::NONE);
    let scope: Vec<&str> = captures.iter().map(|(name, _)| *name).collect();
    let captures: BTreeMap<String, Value> = captures
        .iter()
        .map(|(name, value)| (name.to_string(), value.clone()))
        .collect();
    compile_in_scope("fuel.frst", source, options, &scope)
        .unwrap_or_else(|errors| panic!("{source:?} should compile:\n{}", errors.render_plain()))
        .code
        .close(captures)
        .expect("every capture is supplied")
}

#[allow(clippy::result_large_err)]
fn run_with(source: &str, captures: &[(&str, Value)]) -> Result<ProgramResult, RunError> {
    Vm::factory().build(compile(source, captures)).run()
}

/// The calls a run of `source`, which must succeed, made.
fn calls(source: &str) -> u64 {
    run_with(source, &[])
        .unwrap_or_else(|failure| panic!("{source:?} should run, but raised: {failure:?}"))
        .fuel_consumed()
}

#[frost_fn]
fn noop() {}

// ============================================================
// What counts as a call
// ============================================================

#[test]
fn a_run_with_no_calls_consumes_none() {
    // Running the top level is not itself a call, and operators are not calls.
    assert_eq!(calls("1 + 2 * 3"), 0);
}

#[test]
fn each_call_to_a_frost_function_counts() {
    let source = r"
        defn one() -> 1
        one()
        one()
        one()
    ";
    assert_eq!(calls(source), 3);
}

#[test]
fn each_call_to_a_native_function_counts() {
    let source = r"
        len([1])
        len([1, 2])
    ";
    assert_eq!(calls(source), 2, "the global `len`");
    let result = run_with(
        r"
        noop()
        noop()
        noop()
        ",
        &[("noop", noop_frost().build())],
    )
    .unwrap();
    assert_eq!(result.fuel_consumed(), 3, "a host's native");
}

#[test]
fn each_call_a_native_makes_back_into_frost_counts() {
    // One call to `transform`, then one for each of its three invocations of
    // the lambda.
    assert_eq!(calls("transform([1, 2, 3], fn x -> x)"), 4);
}

#[test]
fn each_tail_call_counts() {
    // The call from the top level, then five tail calls, for `n` from 4 to 0.
    let source = r"
        defn countdown(n) -> if n == 0: 0 else: countdown(n - 1)
        countdown(5)
    ";
    assert_eq!(calls(source), 6);
}

// ============================================================
// Across outcomes and runs
// ============================================================

#[test]
fn a_failed_run_reports_the_calls_it_made() {
    // Two calls to `one`, then the call to `error` that raises.
    let source = r"
        defn one() -> 1
        one()
        one()
        error('stop')
    ";
    let failure = run_with(source, &[]).unwrap_err();
    assert_eq!(
        failure.error().message(),
        "stop",
        "the run failed as written"
    );
    assert_eq!(failure.fuel_consumed(), 3);
}

#[test]
fn a_recycled_vm_counts_each_run_afresh() {
    let program = compile(
        r"
        noop()
        noop()
        ",
        &[("noop", noop_frost().build())],
    );
    let first = Vm::factory().build(Arc::clone(&program)).run().unwrap();
    assert_eq!(first.fuel_consumed(), 2);
    let second = first.into_idle_vm().build(program).run().unwrap();
    assert_eq!(
        second.fuel_consumed(),
        2,
        "the second run's count excludes the first's"
    );
}

// ============================================================
// Imports
// ============================================================

/// Serves every module by running, on a child Vm, a module that makes two
/// calls, and records the child's count.
#[derive(Debug, Default)]
struct CountingModule {
    child_calls: Mutex<Vec<u64>>,
}

impl ImportResolver for CountingModule {
    fn resolve(&self, ctx: &ImportCtx, _module_spec: &str) -> Result<Option<Value>, FrostError> {
        let module = compile(
            r"
            len([1])
            len([2])
            ",
            &[],
        );
        let result = ctx
            .child_factory()
            .build(module)?
            .run()
            .map_err(RunError::into_error)?;
        self.child_calls
            .lock()
            .unwrap()
            .push(result.fuel_consumed());
        Ok(Some(Value::Null))
    }
}

#[test]
fn an_imported_modules_calls_do_not_count() {
    let resolver = Arc::new(CountingModule::default());
    let importer: Arc<Importer> = ImporterBuilder::new()
        .append_resolver(Arc::clone(&resolver) as Arc<dyn ImportResolver>)
        .build();
    let result = Vm::factory()
        .with_importer(importer)
        .build(compile("import('counted')", &[]))
        .run()
        .unwrap();
    assert_eq!(
        *resolver.child_calls.lock().unwrap(),
        [2],
        "the module ran once, making its two calls"
    );
    assert_eq!(
        result.fuel_consumed(),
        1,
        "the importing script's only call is to `import`, without the module's"
    );
}
