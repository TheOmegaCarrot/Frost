//! Tests for [`PrintSink`]: where a Vm's printed text goes.
//!
//! The `print` global's own behavior is covered by `library_output.rs`; these
//! cover the host's side, the sink it configures.

use std::sync::{Arc, Mutex};

use crate::common;

use common::{Pop, global_slot};
use frostlang::bytecode::{Bytecode, CompiledFunction, FormatVersion};
use frostlang::{
    Arity, FrostError, ImportCtx, ImportResolver, ImporterBuilder, PrintSink, Value, Vm,
    VmRuntimeConfiguration,
};

use Bytecode::*;

/// A sink that keeps what it receives, and a handle to read it back.
fn capturing_sink() -> (Arc<dyn PrintSink>, Arc<Mutex<Vec<String>>>) {
    let printed = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let printed = Arc::clone(&printed);
        move |text: &str| printed.lock().unwrap().push(text.to_string())
    };
    (Arc::new(sink), printed)
}

/// A top-level program with `constants`, running `code`.
fn program(constants: Vec<Value>, code: Vec<Bytecode>) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        origin: None,
        code: [Pop].into_iter().chain(code).collect(),
        child_fns: Vec::new(),
        constants,
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    })
}

/// Code that prints constant `index`, leaving `print`'s result.
fn print_const(index: usize) -> [Bytecode; 3] {
    [LoadGlobal(global_slot("print")), LoadConst(index), Call(1)]
}

#[test]
fn a_sink_receives_each_print_in_order() {
    let (sink, printed) = capturing_sink();
    let code = print_const(0)
        .into_iter()
        .chain([Pop])
        .chain(print_const(1))
        .collect();
    let closure = program(vec![Value::from("hello"), Value::Int(5)], code)
        .assert_trusted()
        .into_closure()
        .unwrap();
    let config = VmRuntimeConfiguration::default().with_print_sink(sink);
    let result = Vm::factory()
        .configuration(config)
        .build(closure)
        .run()
        .unwrap();
    assert_eq!(result.tail(), &Value::Null, "print returns Null");
    assert_eq!(*printed.lock().unwrap(), ["hello", "5"]);
}

/// Serves every module by running, in a child Vm, a module that prints.
#[derive(Debug)]
struct PrintingModule;

impl ImportResolver for PrintingModule {
    fn resolve(&self, ctx: &ImportCtx, _module_spec: &str) -> Result<Option<Value>, FrostError> {
        let closure = program(
            vec![Value::from("from the module")],
            print_const(0).to_vec(),
        )
        .assert_trusted()
        .into_closure()
        .expect("no captures");
        ctx.child_factory()
            .build(closure)?
            .run()
            .map_err(frostlang::RunError::into_error)?;
        Ok(Some(Value::Null))
    }
}

#[test]
fn a_module_prints_to_the_importing_vms_sink() {
    let (sink, printed) = capturing_sink();
    let closure = program(vec![Value::from("module")], vec![LoadConst(0), Import])
        .assert_trusted()
        .into_closure()
        .unwrap();
    let config = VmRuntimeConfiguration::default().with_print_sink(sink);
    Vm::factory()
        .with_importer(
            ImporterBuilder::new()
                .append_resolver(Arc::new(PrintingModule))
                .build(),
        )
        .configuration(config)
        .build(closure)
        .run()
        .unwrap();
    assert_eq!(*printed.lock().unwrap(), ["from the module"]);
}

#[test]
fn a_type_implementing_the_trait_is_a_sink() {
    #[derive(Default)]
    struct Counter(Mutex<usize>);
    impl PrintSink for Counter {
        fn print(&self, _text: &str) {
            *self.0.lock().unwrap() += 1;
        }
    }
    let counter = Arc::new(Counter::default());
    let config = VmRuntimeConfiguration::default().with_print_sink(counter.clone());
    let closure = program(vec![Value::Null], print_const(0).to_vec())
        .assert_trusted()
        .into_closure()
        .unwrap();
    Vm::factory()
        .configuration(config)
        .build(closure)
        .run()
        .unwrap();
    assert_eq!(*counter.0.lock().unwrap(), 1);
}

#[test]
fn a_configuration_with_a_sink_is_debuggable() {
    let rendered = format!("{:?}", VmRuntimeConfiguration::default());
    assert!(rendered.contains("print_sink"), "{rendered}");
}
