//! Tests for `fatal(value)`, which ends a run uncatchably.
//!
//! `fatal` latches the run's abort, as fuel exhaustion and cancellation do: no
//! more Frost code runs, no Frost code or native can catch it, and the run
//! fails with [`AbortReason::Fatal`], carrying `value` as its error's value for
//! the host to interpret. Like every abort, it stops at an import: a module's
//! `fatal` ends only the module's run.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use frostlang::compile::{CompilerOptions, OptimizationOptions, compile_in_scope};
use frostlang::native::Function;
use frostlang::{
    AbortReason, CancelToken, Closure, FrostError, FrostOpaque, ImportCtx, ImportResolver,
    ImporterBuilder, NativeCtx, ProgramResult, RunError, RunErrorKind, Value, Vm,
    VmRuntimeConfiguration, frost_closure,
};

// ============================================================
// Harness
// ============================================================

/// A host running scripts: it keeps what they print, and serves
/// `import('module')` by running a module's source in a child Vm.
#[derive(Default)]
struct Host {
    printed: Arc<Mutex<Vec<String>>>,
    module: Option<&'static str>,
}

impl Host {
    fn serving_module(source: &'static str) -> Self {
        Self {
            module: Some(source),
            ..Self::default()
        }
    }

    /// Run `source`, with each of `extra` in its scope.
    #[allow(clippy::result_large_err)]
    fn run_with(&self, source: &str, extra: &[(&str, Value)]) -> Result<ProgramResult, RunError> {
        let printed = Arc::clone(&self.printed);
        let sink = move |text: &str| printed.lock().unwrap().push(text.to_string());
        let mut importer = ImporterBuilder::new();
        if let Some(module) = self.module {
            importer = importer.append_resolver(Arc::new(Module(module)));
        }
        Vm::factory()
            .configuration(VmRuntimeConfiguration::default().with_print_sink(Arc::new(sink)))
            .with_importer(importer.build())
            .build(compile(source, extra))
            .run()
    }

    #[allow(clippy::result_large_err)]
    fn run(&self, source: &str) -> Result<ProgramResult, RunError> {
        self.run_with(source, &[])
    }

    fn printed(&self) -> Vec<String> {
        self.printed.lock().unwrap().clone()
    }
}

/// `source` compiled and closed over `extra`, each in scope.
fn compile(source: &str, extra: &[(&str, Value)]) -> Arc<Closure> {
    let captures: BTreeMap<String, Value> = extra
        .iter()
        .map(|(name, value)| (name.to_string(), value.clone()))
        .collect();
    let scope: Vec<&str> = captures.keys().map(String::as_str).collect();
    compile_in_scope("fatal.frst", source, CompilerOptions::new(), &scope)
        .unwrap_or_else(|errors| panic!("{source:?} should compile:\n{}", errors.render_plain()))
        .code
        .close(captures)
        .expect("every capture is supplied")
}

/// Serves every import by running its source in a child Vm, reporting a
/// failed run as an ordinary error, as a resolver that loads modules does.
#[derive(Debug)]
struct Module(&'static str);

impl ImportResolver for Module {
    fn resolve(&self, ctx: &ImportCtx, _module_spec: &str) -> Result<Option<Value>, FrostError> {
        let result = ctx
            .child_factory()
            .build(compile(self.0, &[]))?
            .run()
            .map_err(RunError::into_error)?;
        Ok(Some(result.tail().clone()))
    }
}

/// Check that `outcome` is a run ended by `fatal`, and hand back the value it
/// was given.
#[track_caller]
#[allow(clippy::result_large_err)]
fn fatal_value(outcome: Result<ProgramResult, RunError>) -> Value {
    let failure = match outcome {
        Ok(result) => panic!(
            "the run should end with `fatal`, but finished with {:?}",
            result.tail()
        ),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.kind(),
        RunErrorKind::Aborted(AbortReason::Fatal),
        "the run should fail as fatal, but failed with: {}",
        failure.error().message()
    );
    failure.into_error().into_value()
}

/// `swallow(f)`: calls `f`, and returns Null whatever it does, discarding any
/// error: a native that tries to carry on past a failure.
fn swallow_native() -> Value {
    frost_closure!(
        "swallow",
        |#[frost(ctx)] ctx: &mut NativeCtx<'_>, function: Function| {
            let _ = ctx.invoke(&function, []);
        }
    )
}

// ============================================================
// The run ends, with the value
// ============================================================

#[test]
fn fatal_ends_the_run_with_its_value() {
    for (source, expected) in [
        ("fatal('stop')", Value::from("stop")),
        ("fatal(null)", Value::Null),
        ("fatal(42)", Value::Int(42)),
        (
            "fatal([1, 'two'])",
            Value::from_iter([Value::Int(1), Value::from("two")]),
        ),
        (
            "fatal({message: 'usage', code: 2})",
            Value::map([("code", Value::Int(2)), ("message", Value::from("usage"))]),
        ),
    ] {
        assert_eq!(
            fatal_value(Host::default().run(source)),
            expected,
            "{source}"
        );
    }
}

#[test]
fn fatal_carries_values_no_literal_can_write() {
    let host = Host::default();
    let value = fatal_value(host.run("fatal(fn x -> x)"));
    assert!(matches!(value, Value::Closure(_)), "{value:?}");

    #[derive(Debug)]
    struct Token;
    impl FrostOpaque for Token {
        fn type_name() -> &'static str {
            "Token"
        }

        fn try_to_string(&self) -> Option<String> {
            None
        }
    }
    let value = fatal_value(host.run_with("fatal(token)", &[("token", Value::opaque(Token))]));
    assert!(value.downcast_opaque::<Token>().is_some(), "{value:?}");
}

#[test]
fn nothing_runs_after_fatal() {
    let host = Host::default();
    fatal_value(host.run(
        r"
        print('before')
        fatal('stop')
        print('after')
        ",
    ));
    assert_eq!(host.printed(), ["before"]);
}

// ============================================================
// Nothing catches it
// ============================================================

#[test]
fn try_call_cannot_catch_fatal() {
    let host = Host::default();
    let value = fatal_value(host.run(
        r"
        def caught = try_call(fatal, ['inner'])
        print('after the try_call')
        caught
        ",
    ));
    assert_eq!(value, Value::from("inner"));
    assert!(host.printed().is_empty(), "{:?}", host.printed());
}

#[test]
fn fatal_in_a_callback_ends_the_whole_run() {
    for source in [
        "map([1, 2, 3], fn x -> if x == 2: fatal(x) else: x)",
        "each([1, 2, 3], fn x -> if x == 2: fatal(x) else: null)",
        "reduce([1, 2, 3], fn acc, x -> if x == 2: fatal(x) else: acc + x)",
        "try_call(fn -> map([1, 2], fn x -> if x == 2: fatal(x) else: x), [])",
    ] {
        assert_eq!(
            fatal_value(Host::default().run(source)),
            Value::Int(2),
            "{source}"
        );
    }
}

#[test]
fn a_native_cannot_swallow_fatal() {
    let host = Host::default();
    let value = fatal_value(host.run_with(
        r"
        swallow(fn -> fatal('deep'))
        print('after the native')
        ",
        &[("swallow", swallow_native())],
    ));
    assert_eq!(value, Value::from("deep"));
    assert!(host.printed().is_empty(), "{:?}", host.printed());
}

#[test]
fn the_first_abort_wins() {
    // A native that, once the callback has called `fatal`, cancels the run:
    // the run is already aborting, so it stays fatal.
    let token = CancelToken::new();
    let cancel_after = {
        let token = token.clone();
        frost_closure!(
            "cancel_after",
            move |#[frost(ctx)] ctx: &mut NativeCtx<'_>, function: Function| {
                let _ = ctx.invoke(&function, []);
                token.cancel();
            }
        )
    };
    let outcome = Vm::factory()
        .build(compile(
            "cancel_after(fn -> fatal('first'))",
            &[("cancel_after", cancel_after)],
        ))
        .with_cancel_token(token)
        .run();
    assert_eq!(fatal_value(outcome), Value::from("first"));
}

// ============================================================
// Unwinding, folding, and imports
// ============================================================

#[test]
fn a_fatal_run_drops_its_values() {
    /// Records being dropped.
    #[derive(Debug)]
    struct DropFlag(Arc<AtomicBool>);
    impl FrostOpaque for DropFlag {
        fn type_name() -> &'static str {
            "DropFlag"
        }

        fn try_to_string(&self) -> Option<String> {
            None
        }
    }
    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let dropped = Arc::new(AtomicBool::new(false));
    let flag = Value::opaque(DropFlag(Arc::clone(&dropped)));
    let outcome = Host::default().run_with(
        r"
        def held = [flag]
        fatal('stop')
        held
        ",
        &[("flag", flag)],
    );
    let failure = outcome.expect_err("the run ends with `fatal`");
    drop(failure);
    assert!(
        dropped.load(Ordering::SeqCst),
        "everything the run held is dropped with it"
    );
}

#[test]
fn constant_folding_never_evaluates_fatal() {
    // Each would fold, were `fatal` foldable; it must stay a runtime abort.
    for source in [
        "fatal(1 + 1)",
        "def f = fn -> fatal(2)\nf()",
        "[1, fatal(2)][0]",
    ] {
        let program = compile_in_scope(
            "fold.frst",
            source,
            CompilerOptions::new().with_optimization(OptimizationOptions::ALL),
            &[],
        )
        .unwrap_or_else(|errors| panic!("{source:?} compiles:\n{}", errors.render_plain()));
        let closure = program.code.into_closure().expect("captures nothing");
        let value = fatal_value(Vm::factory().build(closure).run());
        assert_eq!(value, Value::Int(2), "{source}");
    }
}

#[test]
fn a_modules_fatal_ends_only_the_modules_run() {
    // The importer sees the resolver's report of the failed module run: an
    // ordinary error, which it can catch.
    let host = Host::serving_module("fatal('module gave up')");
    let outcome = host.run(
        r"
        def attempt = try_call(import, ['module'])
        print('importer carries on')
        attempt.ok
        ",
    );
    match outcome {
        Ok(result) => assert_eq!(result.tail(), &Value::Bool(false)),
        Err(failure) => panic!(
            "the importer should carry on, but failed: {}",
            failure.error()
        ),
    }
    assert_eq!(host.printed(), ["importer carries on"]);

    // Uncaught, it fails the importer as a raised error, not an abort.
    let failure = Host::serving_module("fatal('module gave up')")
        .run("import('module')")
        .expect_err("the import fails");
    assert_eq!(failure.kind(), RunErrorKind::Raised);
    assert_eq!(failure.error().message(), "module gave up");
}

// ============================================================
// Arity and types
// ============================================================

#[test]
fn fatal_takes_exactly_one_argument_of_any_type() {
    for (call, count) in [("fatal()", 0), ("fatal(1, 2)", 2)] {
        let failure = Host::default().run(call).expect_err("a wrong arity raises");
        assert_eq!(
            failure.kind(),
            RunErrorKind::Raised,
            "{call}: an ordinary error"
        );
        let noun = "argument";
        assert_eq!(
            failure.error().message(),
            format!("Function fatal expects 1 {noun}, but was called with {count}"),
            "{call}"
        );
    }
    // A wrong arity is ordinary: it can be caught.
    let result = Host::default()
        .run("try_call(fatal, []).ok")
        .unwrap_or_else(|failure| panic!("catchable: {}", failure.error()));
    assert_eq!(result.tail(), &Value::Bool(false));
}
