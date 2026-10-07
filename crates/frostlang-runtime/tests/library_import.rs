//! The `import` and `imported` globals, from Frost source.
//!
//! `imported()` is true while the script running it was imported by another,
//! and false while it was run directly.
//!
//! Modules here are Frost source, compiled and run by a resolver as a host would
//! serve script files. The harness runs every case under every optimization
//! permutation, so a call that could be folded is checked both ways.

mod source;

use std::collections::BTreeMap;
use std::sync::Arc;

use frostlang_compile::{CompilerOptions, OptimizationOptions, compile_in_scope};
use frostlang_runtime::{
    FrostError, HostComponent, ImportCtx, ImportResolver, ImporterBuilder, RunError, Value,
};
use source::Script;
use source::assertions::{Library, library_assertions};

library_assertions!(Library::GLOBALS);

/// Resolves each spec it holds by compiling that module's source and running it
/// in a child Vm, which yields the module's tail value.
#[derive(Debug)]
struct SourceModules(BTreeMap<&'static str, &'static str>);

impl ImportResolver for SourceModules {
    fn resolve(&self, ctx: &ImportCtx, module_spec: &str) -> Result<Option<Value>, FrostError> {
        let Some(source) = self.0.get(module_spec) else {
            return Ok(None);
        };
        // Every optimization, so the module's own calls are folded wherever they may be.
        let options = CompilerOptions {
            optimization_options: OptimizationOptions::ALL,
            implicit_export: false,
        };
        let program = compile_in_scope(module_spec, source, options, &[])
            .unwrap_or_else(|errors| panic!("{module_spec} compiles:\n{}", errors.render_plain()))
            .code
            .close(BTreeMap::new())
            .expect("a module captures nothing");
        let result = ctx
            .child_factory()
            .build(program)?
            .run()
            .map_err(RunError::into_error)?;
        Ok(Some(result.tail().clone()))
    }
}

/// `source`, run with `modules` importable by name.
fn with_modules(source: &str, modules: &[(&'static str, &'static str)]) -> Script {
    let resolver = SourceModules(modules.iter().copied().collect());
    let importer = ImporterBuilder::new()
        .append_resolver(Arc::new(resolver))
        .build();
    Script::new(source).importer(importer)
}

#[test]
fn imported_is_false_in_a_script_run_directly() {
    assert_values(&[("imported()", "false")]);
}

#[test]
fn imported_is_true_in_an_imported_module() {
    let script = with_modules(
        "[imported(), import('module')]",
        &[("module", "imported()")],
    );
    assert_eq!(script.run(), Value::array([false, true]));
}

#[test]
fn imported_is_true_in_a_module_imported_by_a_module() {
    let script = with_modules(
        "import('outer')",
        &[
            ("outer", "[imported(), import('inner')]"),
            ("inner", "imported()"),
        ],
    );
    assert_eq!(script.run(), Value::array([true, true]));
}

#[test]
fn imported_lets_a_script_run_only_when_run_directly() {
    let module = r"
        defn main() -> 'ran'
        if imported(): 'skipped' else: main()
    ";
    let script = with_modules(module, &[("module", module)]);
    assert_eq!(script.run(), Value::from("ran"));
    let script = with_modules("import('module')", &[("module", module)]);
    assert_eq!(script.run(), Value::from("skipped"));
}

#[test]
fn the_import_depth_limit_stops_a_module_nesting_too_deep() {
    // One level is allowed: the top level may import `outer`, but `outer` may
    // not import `inner`, which would run a level deeper.
    let script = with_modules(
        "import('outer')",
        &[("outer", "import('inner')"), ("inner", "1")],
    )
    .max_import_depth(1);
    assert_eq!(script.raises(), "Import depth limit of 1 exceeded");
}

#[test]
fn a_registry_import_succeeds_at_the_import_depth_limit() {
    // `outer` runs at the limit. Importing from the registry runs no module, so
    // it nests no deeper and is allowed.
    let resolver = SourceModules(BTreeMap::from([("outer", "import('host')")]));
    let importer = ImporterBuilder::new()
        .with_component(HostComponent::new("host", Value::Int(42)).expect("a valid name"))
        .expect("an unclaimed name")
        .append_resolver(Arc::new(resolver))
        .build();
    let script = Script::new("import('outer')")
        .importer(importer)
        .max_import_depth(1);
    assert_eq!(script.run(), Value::Int(42));
}

#[test]
fn imported_takes_no_arguments() {
    assert_arity("imported", 0, &[1, 2]);
}

#[test]
fn import_requires_a_string() {
    assert_raises(&[
        (
            "import(1)",
            "Function import requires String as argument 1, got Int",
        ),
        (
            "import(x'61')",
            "Function import requires String as argument 1, got Bytes",
        ),
    ]);
    assert_arity("import", 1, &[0, 2]);
}
