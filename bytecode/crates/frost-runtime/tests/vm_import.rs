//! Executing `import` through the VM: the `Import` opcode (the inlined form the
//! compiler emits for `import("literal")`) and the `import` global (the first-class
//! function). Registry resolution itself is covered by the runtime's internal
//! `resolve_tests`; here the concern is the VM wiring: the opcode's stack effect and
//! argument errors, and that the global is an ordinary callable `Function` value.

use std::sync::Arc;

mod common;

use common::{Pop, global_slot};
use frost_runtime::{
    Arity, Bytecode, CompiledFunction, Extension, FormatVersion, FrostError, HostComponent,
    Importer, ImporterBuilder, Value, Vm,
};

use Bytecode::*;

fn leaf(n: i64) -> Value {
    Value::from(n)
}

/// A small importer: `ext.sqlite = { open: 1 }` and top-level `myapp = 2`.
/// (Stdlib is crate-constructed only, so it is exercised in `resolve_tests`, not here.)
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
        code: body,
        child_fns: Vec::new(),
        constants,
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
        .map_err(|e| e.into_error())
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
    let err = run(vec![], vec![PushInt(5), Import]).unwrap_err();
    assert!(err.message().contains("String"), "{}", err.message());
}

#[test]
fn opcode_errors_on_a_non_utf8_spec() {
    // A String constant whose bytes are not valid UTF-8.
    let err = run(
        vec![Value::from(&[0x80u8, 0xff][..])],
        vec![LoadConst(0), Import],
    )
    .unwrap_err();
    assert!(err.message().contains("UTF-8"), "{}", err.message());
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
    assert!(err.message().contains("String"), "{}", err.message());
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
        code: vec![Pop, LoadConst(0), Import],
        child_fns: Vec::new(),
        constants: vec![Value::from("ext.sqlite")],
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let closure = program.assert_trusted().into_closure().unwrap();
    let result = Vm::factory().build(closure).unwrap().run();
    assert!(result.is_err());
}
