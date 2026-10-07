//! Executing `import` through the VM: the `Import` opcode (the inlined form the
//! compiler emits for `import("literal")`) and the `import` global (the first-class
//! function). Registry resolution itself is covered by the runtime's internal
//! `resolve_tests`; here the concern is the VM wiring: the opcode's stack effect and
//! argument errors, and that the global is an ordinary callable `Function` value.

use std::num::NonZeroUsize;
use std::sync::Arc;

mod common;

use common::{Pop, global_slot};
use frostlang::bytecode::{Bytecode, CompiledFunction, FormatVersion};
use frostlang::{
    Arity, Extension, FrostError, HostComponent, ImportCtx, ImportResolver, Importer,
    ImporterBuilder, ModuleId, Value, Vm, VmRuntimeConfiguration,
};

use Bytecode::*;

fn leaf(n: i64) -> Value {
    Value::from(n)
}

/// A small importer: `ext.sqlite = { open: 1 }` and top-level `myapp = 2`.
/// (`std` is exercised in `stdlib.rs` and the `std_*` tests, not here.)
fn importer() -> Arc<Importer> {
    ImporterBuilder::new()
        .with_extension(Extension::new("sqlite", Value::map([("open", leaf(1))])).unwrap())
        .unwrap()
        .with_component(HostComponent::new("myapp", leaf(2)).unwrap())
        .unwrap()
        .build()
}

/// Run a top-level program (with `constants`) under [`importer`], returning the tail
/// value or the raised error. Splices in the leading fn-value `Pop` the top-level needs.
fn run(constants: Vec<Value>, code: Vec<Bytecode>) -> Result<Value, FrostError> {
    let mut body = vec![Pop];
    body.extend(code);
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        origin: None,
        code: body,
        child_fns: Vec::new(),
        constants,
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let closure = program.assert_trusted().into_closure().unwrap();
    Vm::factory()
        .with_importer(importer())
        .build(closure)
        .unwrap()
        .run()
        .map_err(frostlang::RunError::into_error)
        .map(|r| r.tail().clone())
}

// ============================================================
// The `Import` opcode (the inlined `import("literal")` form)
// ============================================================

#[test]
fn opcode_resolves_an_extension_map() {
    let v = run(vec![Value::from("ext.sqlite")], vec![LoadConst(0), Import]).unwrap();
    assert_eq!(v.as_map().unwrap().get_str("open"), Some(&leaf(1)));
}

#[test]
fn opcode_resolves_a_fine_grained_leaf() {
    let v = run(
        vec![Value::from("ext.sqlite.open")],
        vec![LoadConst(0), Import],
    )
    .unwrap();
    assert_eq!(v, leaf(1));
}

#[test]
fn opcode_resolves_a_host_component() {
    let v = run(vec![Value::from("myapp")], vec![LoadConst(0), Import]).unwrap();
    assert_eq!(v, leaf(2));
}

#[test]
fn opcode_errors_on_an_unresolved_spec() {
    assert!(run(vec![Value::from("nope")], vec![LoadConst(0), Import]).is_err());
}

#[test]
fn opcode_errors_on_a_non_string_spec() {
    // The assertion names the operand type: the message's constant prefix already
    // contains "String", so matching that alone could never fail.
    let err = run(vec![], vec![PushInt(5), Import]).unwrap_err();
    assert!(err.message().contains("got Int"), "{}", err.message());
}

#[test]
fn opcode_errors_on_a_bytes_spec() {
    // Binary is not a module name. A String is UTF-8 by construction, so there is
    // no "String that fails to decode" case left: the only way to hand `Import`
    // arbitrary bytes is to hand it a Bytes, which is a type error.
    let err = run(
        vec![Value::from(&[0x80u8, 0xff][..])],
        vec![LoadConst(0), Import],
    )
    .unwrap_err();
    assert!(err.message().contains("got Bytes"), "{}", err.message());
}

// ============================================================
// The `import` global (first-class function)
// ============================================================

#[test]
fn global_resolves_like_the_opcode() {
    // `import("ext.sqlite.open")` through LoadGlobal + Call, not the inlined opcode.
    let v = run(
        vec![Value::from("ext.sqlite.open")],
        vec![LoadGlobal(global_slot("import")), LoadConst(0), Call(1)],
    )
    .unwrap();
    assert_eq!(v, leaf(1));
}

#[test]
fn global_errors_on_a_non_string_spec() {
    let err = run(
        vec![],
        vec![LoadGlobal(global_slot("import")), PushInt(5), Call(1)],
    )
    .unwrap_err();
    assert!(err.message().contains("got Int"), "{}", err.message());
}

#[test]
fn global_enforces_its_arity() {
    // The global is an `Exact(1)` closure: calling it with no args is an arity error.
    assert!(run(vec![], vec![LoadGlobal(global_slot("import")), Call(0)]).is_err());
}

// ============================================================
// Default importer (no importer configured)
// ============================================================

#[test]
fn a_default_vm_resolves_nothing() {
    // `Vm::factory()` with no `with_importer` builds the empty importer, so any
    // import errors. This mirrors the secure default (imports off until configured).
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        origin: None,
        code: vec![Pop, LoadConst(0), Import],
        child_fns: Vec::new(),
        constants: vec![Value::from("ext.sqlite")],
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let closure = program.assert_trusted().into_closure().unwrap();
    let result = Vm::factory().build(closure).unwrap().run();
    assert!(result.is_err());
}

// ============================================================
// Resolver chain through the VM
// ============================================================

/// A program that imports `spec` and yields the imported value.
fn importing_program(spec: &str) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "module".to_string(),
        origin: None,
        code: vec![Pop, LoadConst(0), Import],
        child_fns: Vec::new(),
        constants: vec![Value::from(spec)],
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    })
}

/// A resolver that runs every requested module in a child Vm, as a real one would.
/// The module it serves imports again, so resolving anything recurses until
/// something stops it.
#[derive(Debug)]
struct RecursiveResolver;

impl ImportResolver for RecursiveResolver {
    fn resolve(&self, ctx: &ImportCtx, module_spec: &str) -> Result<Option<Value>, FrostError> {
        let closure = importing_program(module_spec)
            .assert_trusted()
            .into_closure()
            .expect("no captures");
        let value = ctx
            .child_factory()
            .build(closure)
            .map_err(|e| FrostError::from_string(e.message().to_string()))?
            .with_module_id(ModuleId::new(module_spec))
            .run()
            .map_err(frostlang::RunError::into_error)?
            .tail()
            .clone();
        Ok(Some(value))
    }
}

/// A resolver that reports the importing module's id back as the imported value.
#[derive(Debug)]
struct EchoesImporter;

impl ImportResolver for EchoesImporter {
    fn resolve(&self, ctx: &ImportCtx, _module_spec: &str) -> Result<Option<Value>, FrostError> {
        Ok(Some(match ctx.importing_module() {
            Some(id) => Value::from(id.as_str()),
            None => Value::Null,
        }))
    }
}

fn run_with(
    resolver: Arc<dyn ImportResolver>,
    config: VmRuntimeConfiguration,
    module_id: Option<ModuleId>,
) -> Result<Value, FrostError> {
    let closure = importing_program("anything")
        .assert_trusted()
        .into_closure()
        .unwrap();
    let mut vm = Vm::factory()
        .with_importer(ImporterBuilder::new().append_resolver(resolver).build())
        .configuration(config)
        .build(closure)
        .unwrap();
    if let Some(id) = module_id {
        vm = vm.with_module_id(id);
    }
    vm.run()
        .map_err(frostlang::RunError::into_error)
        .map(|r| r.tail().clone())
}

#[test]
fn import_depth_limit_stops_runaway_import_recursion() {
    // The resolver never terminates on its own (every module it serves imports
    // again), so only the depth limit can end this. Without it the native stack
    // would be exhausted, which is not a catchable error.
    let config = VmRuntimeConfiguration::default().with_max_import_depth(NonZeroUsize::new(4));
    let err = run_with(Arc::new(RecursiveResolver), config, None).unwrap_err();
    assert!(
        err.message().contains("Import depth limit"),
        "got: {}",
        err.message()
    );
}

#[test]
fn a_resolver_sees_the_importing_modules_id() {
    let id = ModuleId::new("caller-module");
    let value = run_with(Arc::new(EchoesImporter), Default::default(), Some(id)).unwrap();
    assert_eq!(value, Value::from("caller-module"));
}

#[test]
fn an_unidentified_script_imports_with_no_module_id() {
    // A REPL or `-e` one-liner has no identity, and a resolver must cope.
    let value = run_with(Arc::new(EchoesImporter), Default::default(), None).unwrap();
    assert_eq!(value, Value::Null);
}

/// A Vm identified as `caller-module`, running `program` under [`EchoesImporter`].
fn identified_echo_vm(program: Arc<CompiledFunction>) -> Vm {
    let closure = program.assert_trusted().into_closure().unwrap();
    Vm::factory()
        .with_importer(
            ImporterBuilder::new()
                .append_resolver(Arc::new(EchoesImporter))
                .build(),
        )
        .build(closure)
        .unwrap()
        .with_module_id(ModuleId::new("caller-module"))
}

/// The importing module's id as the resolver sees it, imported by `vm`.
fn echoed_id(vm: Vm) -> Value {
    vm.run().unwrap().tail().clone()
}

fn echoing_closure() -> Arc<frostlang::Closure> {
    importing_program("anything")
        .assert_trusted()
        .into_closure()
        .unwrap()
}

#[test]
fn a_recycled_vm_forgets_the_previous_scripts_module_id() {
    // A recycled Vm runs a different script, so the previous script's identity
    // must not reach that script's imports. Every way of recycling is checked.
    let succeeded = identified_echo_vm(importing_program("anything"))
        .run()
        .unwrap();
    assert_eq!(succeeded.tail(), &Value::from("caller-module"));
    assert_eq!(
        echoed_id(succeeded.reset(echoing_closure())),
        Value::Null,
        "reset after success"
    );

    let succeeded = identified_echo_vm(importing_program("anything"))
        .run()
        .unwrap();
    assert_eq!(
        echoed_id(succeeded.into_idle_vm().build(echoing_closure())),
        Value::Null,
        "rebuilt after success"
    );

    let raises = || {
        Arc::new(CompiledFunction {
            code: vec![Pop, PushNull, ProduceError],
            ..(*importing_program("anything")).clone()
        })
    };
    let failed = identified_echo_vm(raises()).run().unwrap_err();
    assert_eq!(
        echoed_id(failed.reset(echoing_closure())),
        Value::Null,
        "reset after failure"
    );

    let failed = identified_echo_vm(raises()).run().unwrap_err();
    assert_eq!(
        echoed_id(failed.into_idle_vm().build(echoing_closure())),
        Value::Null,
        "rebuilt after failure"
    );
}

#[test]
fn a_recycled_vm_can_be_identified_again() {
    let succeeded = identified_echo_vm(importing_program("anything"))
        .run()
        .unwrap();
    let vm = succeeded
        .reset(echoing_closure())
        .with_module_id(ModuleId::new("next-module"));
    assert_eq!(echoed_id(vm), Value::from("next-module"));
}

// ============================================================
// The `imported()` global
// ============================================================

/// A module whose body is `imported()`: it reports whether the Vm running it is
/// at import depth above zero.
fn reports_imported() -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "module".to_string(),
        origin: None,
        code: vec![Pop, LoadGlobal(global_slot("imported")), Call(0)],
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    })
}

/// Serves every requested module by running [`reports_imported`] in a child Vm,
/// exactly as a real resolver runs an imported module.
#[derive(Debug)]
struct RunsImportedReporter;

impl ImportResolver for RunsImportedReporter {
    fn resolve(&self, ctx: &ImportCtx, _module_spec: &str) -> Result<Option<Value>, FrostError> {
        let closure = reports_imported()
            .assert_trusted()
            .into_closure()
            .expect("no captures");
        let value = ctx
            .child_factory()
            .build(closure)
            .map_err(|e| FrostError::from_string(e.message().to_string()))?
            .run()
            .map_err(frostlang::RunError::into_error)?
            .tail()
            .clone();
        Ok(Some(value))
    }
}

#[test]
fn imported_is_false_when_run_directly() {
    // A top-level script runs at depth 0, so it observes itself as not imported.
    let v = run(vec![], vec![LoadGlobal(global_slot("imported")), Call(0)]).unwrap();
    assert_eq!(v, Value::Bool(false));
}

#[test]
fn imported_is_true_inside_an_imported_module() {
    // The outer program imports a module whose body is `imported()`. That module
    // runs in a child Vm (depth 1), so it observes itself as imported.
    let v = run_with(Arc::new(RunsImportedReporter), Default::default(), None).unwrap();
    assert_eq!(v, Value::Bool(true));
}
