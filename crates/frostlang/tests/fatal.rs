//! Tests for `fatal(value)`, which ends a run uncatchably.
//!
//! `fatal` latches the run's abort, as fuel exhaustion and cancellation do: no
//! more Frost code runs, no Frost code or native can catch it, and the run
//! fails with [`AbortReason::Fatal`], carrying `value` as its error's value for
//! the host to interpret. Like every abort, it stops at an import: a module's
//! `fatal` ends only the module's run.
//!
//! Each behavior is checked under every set of optimizations
//! [`optimization_permutations`] gives.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use frostlang::compile::{CompilerOptions, OptimizationOptions, compile_in_scope};
use frostlang::native::Function;
use frostlang::{
    AbortReason, CancelToken, Closure, FrostError, FrostOpaque, ImportCtx, ImportResolver,
    ImporterBuilder, NativeCtx, ProgramResult, RunError, RunErrorKind, Value, Vm, VmFactory,
    VmRuntimeConfiguration, frost_closure,
};

use crate::script::optimization_permutations;

// ============================================================
// Harness
// ============================================================

/// A host running scripts compiled with one set of optimizations: it keeps
/// what they print, and may serve `import('module')` by running a module's
/// source in a child Vm.
struct Host {
    optimization: OptimizationOptions,
    printed: Arc<Mutex<Vec<String>>>,
    module: Option<&'static str>,
}

/// A host for each set of optimizations the tests run under.
fn hosts() -> impl Iterator<Item = Host> {
    optimization_permutations().map(|optimization| Host {
        optimization,
        printed: Arc::default(),
        module: None,
    })
}

impl Host {
    /// This host, serving every import by running `source` as a module.
    fn serving_module(mut self, source: &'static str) -> Self {
        self.module = Some(source);
        self
    }

    /// `source` compiled with this host's optimizations, and closed over
    /// `extra`, each in scope.
    fn compile(&self, source: &str, extra: &[(&str, Value)]) -> Arc<Closure> {
        compile(source, extra, self.optimization)
    }

    /// A factory whose Vms print to this host and import from it.
    fn factory(&self) -> VmFactory {
        let printed = Arc::clone(&self.printed);
        let sink = move |text: &str| printed.lock().unwrap().push(text.to_string());
        let mut importer = ImporterBuilder::new();
        if let Some(source) = self.module {
            importer = importer.append_resolver(Arc::new(Module {
                source,
                optimization: self.optimization,
            }));
        }
        Vm::factory()
            .configuration(VmRuntimeConfiguration::default().with_print_sink(Arc::new(sink)))
            .with_importer(importer.build())
    }

    /// Run `source`, with each of `extra` in its scope.
    #[allow(clippy::result_large_err)]
    fn run_with(&self, source: &str, extra: &[(&str, Value)]) -> Result<ProgramResult, RunError> {
        self.factory().build(self.compile(source, extra)).run()
    }

    #[allow(clippy::result_large_err)]
    fn run(&self, source: &str) -> Result<ProgramResult, RunError> {
        self.run_with(source, &[])
    }

    fn printed(&self) -> Vec<String> {
        self.printed.lock().unwrap().clone()
    }
}

/// `source` compiled with `optimization`, and closed over `extra`, each in
/// scope.
fn compile(
    source: &str,
    extra: &[(&str, Value)],
    optimization: OptimizationOptions,
) -> Arc<Closure> {
    let captures: BTreeMap<String, Value> = extra
        .iter()
        .map(|(name, value)| (name.to_string(), value.clone()))
        .collect();
    let scope: Vec<&str> = captures.keys().map(String::as_str).collect();
    let options = CompilerOptions::new().with_optimization(optimization);
    compile_in_scope("fatal.frst", source, options, &scope)
        .unwrap_or_else(|errors| panic!("{source:?} should compile:\n{}", errors.render_plain()))
        .code
        .close(captures)
        .expect("every capture is supplied")
}

/// Serves every import by running its source in a child Vm, reporting a
/// failed run as an ordinary error, as a resolver that loads modules does.
#[derive(Debug)]
struct Module {
    source: &'static str,
    optimization: OptimizationOptions,
}

impl ImportResolver for Module {
    fn resolve(&self, ctx: &ImportCtx, _module_spec: &str) -> Result<Option<Value>, FrostError> {
        let result = ctx
            .child_factory()
            .build(compile(self.source, &[], self.optimization))?
            .run()
            .map_err(RunError::into_error)?;
        Ok(Some(result.tail().clone()))
    }
}

/// Check that `outcome` is a run ended by `fatal`, and hand back the failure.
#[track_caller]
#[allow(clippy::result_large_err)]
fn assert_fatal(outcome: Result<ProgramResult, RunError>, context: &str) -> RunError {
    let failure = match outcome {
        Ok(result) => panic!(
            "{context}: the run should end with `fatal`, but finished with {:?}",
            result.tail()
        ),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.kind(),
        RunErrorKind::Aborted(AbortReason::Fatal),
        "{context}: the run should fail as fatal, but failed with: {}",
        failure.error().message()
    );
    failure
}

/// [`assert_fatal`], handing back the value `fatal` was given.
#[track_caller]
#[allow(clippy::result_large_err)]
fn fatal_value(outcome: Result<ProgramResult, RunError>, context: &str) -> Value {
    assert_fatal(outcome, context).into_error().into_value()
}

/// An Opaque that records being dropped.
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
        for host in hosts() {
            let context = format!("{source} under {:?}", host.optimization);
            assert_eq!(
                fatal_value(host.run(source), &context),
                expected,
                "{context}"
            );
        }
    }
}

#[test]
fn fatal_carries_values_no_literal_can_write() {
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

    for host in hosts() {
        let context = format!("{:?}", host.optimization);
        let value = fatal_value(host.run("fatal(fn x -> x)"), &context);
        assert!(matches!(value, Value::Closure(_)), "{context}: {value:?}");
        let value = fatal_value(
            host.run_with("fatal(token)", &[("token", Value::opaque(Token))]),
            &context,
        );
        assert!(
            value.downcast_opaque::<Token>().is_some(),
            "{context}: {value:?}"
        );
    }
}

#[test]
fn nothing_runs_after_fatal() {
    let source = r"
print('before')
fatal('stop')
print('after')
";
    for host in hosts() {
        let context = format!("{:?}", host.optimization);
        fatal_value(host.run(source), &context);
        assert_eq!(host.printed(), ["before"], "{context}");
    }
}

// ============================================================
// Nothing catches it
// ============================================================

#[test]
fn try_call_cannot_catch_fatal() {
    let source = r"
def caught = try_call(fatal, ['inner'])
print('after the try_call')
caught
";
    for host in hosts() {
        let context = format!("{:?}", host.optimization);
        assert_eq!(
            fatal_value(host.run(source), &context),
            Value::from("inner"),
            "{context}"
        );
        assert!(host.printed().is_empty(), "{context}: {:?}", host.printed());
    }
}

#[test]
fn fatal_in_a_callback_ends_the_whole_run() {
    for source in [
        "map([1, 2, 3], fn x -> if x == 2: fatal(x) else: x)",
        "each([1, 2, 3], fn x -> if x == 2: fatal(x) else: null)",
        "reduce([1, 2, 3], fn acc, x -> if x == 2: fatal(x) else: acc + x)",
        "try_call(fn -> map([1, 2], fn x -> if x == 2: fatal(x) else: x), [])",
    ] {
        for host in hosts() {
            let context = format!("{source} under {:?}", host.optimization);
            assert_eq!(
                fatal_value(host.run(source), &context),
                Value::Int(2),
                "{context}"
            );
        }
    }
}

#[test]
fn a_native_cannot_swallow_fatal() {
    // `swallow(f)` calls `f` and returns Null whatever it does, discarding any
    // error: a native that tries to carry on past a failure.
    let swallow = frost_closure!(
        "swallow",
        |#[frost(ctx)] ctx: &mut NativeCtx<'_>, function: Function| {
            let _ = ctx.invoke(&function, []);
        }
    );
    let source = r"
swallow(fn -> fatal('deep'))
print('after the native')
";
    for host in hosts() {
        let context = format!("{:?}", host.optimization);
        let value = fatal_value(
            host.run_with(source, &[("swallow", swallow.clone())]),
            &context,
        );
        assert_eq!(value, Value::from("deep"), "{context}");
        assert!(host.printed().is_empty(), "{context}: {:?}", host.printed());
    }
}

/// What a native saw once a function it called back into had called `fatal`.
#[derive(Debug)]
struct Seen {
    reason: Option<AbortReason>,
    check: Result<(), FrostError>,
    second_call: Result<Value, FrostError>,
}

#[test]
fn a_native_sees_the_run_is_aborting_as_fatal() {
    for host in hosts() {
        let context = format!("{:?}", host.optimization);
        let seen: Arc<Mutex<Option<Seen>>> = Arc::default();
        let observe = {
            let seen = Arc::clone(&seen);
            // `observe(aborting, other)`: calls `aborting`, notes what it can
            // see of the run, then tries to call `other`.
            frost_closure!(
                "observe",
                move |#[frost(ctx)] ctx: &mut NativeCtx<'_>,
                      aborting: Function,
                      other: Function| {
                    let _ = ctx.invoke(&aborting, []);
                    let reason = ctx.abort_reason();
                    let check = ctx.check_abort();
                    let second_call = ctx.invoke(&other, []);
                    *seen.lock().unwrap() = Some(Seen {
                        reason,
                        check,
                        second_call,
                    });
                }
            )
        };
        // Were `other` to run, it would print to the host.
        let source = "observe(fn -> fatal('payload'), fn -> print('ran'))";
        let value = fatal_value(host.run_with(source, &[("observe", observe)]), &context);
        assert_eq!(value, Value::from("payload"), "{context}");
        assert!(
            host.printed().is_empty(),
            "{context}: the second call is refused, not run: {:?}",
            host.printed()
        );
        let seen = seen
            .lock()
            .unwrap()
            .take()
            .expect("the native ran to its end");
        assert_eq!(seen.reason, Some(AbortReason::Fatal), "{context}: {seen:?}");
        let check = seen.check.expect_err("`check_abort` fails while aborting");
        assert_eq!(check.into_value(), Value::from("payload"), "{context}");
        let refused = seen
            .second_call
            .expect_err("a native cannot call back into Frost once aborting");
        assert_eq!(refused.into_value(), Value::from("payload"), "{context}");
    }
}

#[test]
fn the_first_abort_wins() {
    // A native that, once the callback has called `fatal`, cancels the run:
    // the run is already aborting, so it stays fatal.
    for host in hosts() {
        let context = format!("{:?}", host.optimization);
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
        let closure = host.compile(
            "cancel_after(fn -> fatal('first'))",
            &[("cancel_after", cancel_after)],
        );
        let outcome = host.factory().build(closure).with_cancel_token(token).run();
        assert_eq!(
            fatal_value(outcome, &context),
            Value::from("first"),
            "{context}"
        );
    }
}

// ============================================================
// Unwinding, reuse, folding, and imports
// ============================================================

#[test]
fn a_fatal_run_drops_its_values() {
    let source = r"
def held = [flag]
fatal('stop')
held
";
    for host in hosts() {
        let context = format!("{:?}", host.optimization);
        let dropped = Arc::new(AtomicBool::new(false));
        let flag = Value::opaque(DropFlag(Arc::clone(&dropped)));
        let failure = assert_fatal(host.run_with(source, &[("flag", flag)]), &context);
        drop(failure);
        assert!(
            dropped.load(Ordering::SeqCst),
            "{context}: everything the run held is dropped with it"
        );
    }
}

#[test]
fn a_vm_runs_again_after_a_fatal_run() {
    for host in hosts() {
        let context = format!("{:?}", host.optimization);
        let failure = assert_fatal(host.run("fatal('first')"), &context);
        let second = r"
print('second')
1 + 1
";
        let outcome = failure
            .into_idle_vm()
            .build(host.compile(second, &[]))
            .run();
        match outcome {
            Ok(result) => assert_eq!(result.tail(), &Value::Int(2), "{context}"),
            Err(failure) => panic!(
                "{context}: the recycled Vm should finish, but failed with {:?}: {}",
                failure.kind(),
                failure.error()
            ),
        }
        assert_eq!(host.printed(), ["second"], "{context}");
    }
}

#[test]
fn a_fatal_folding_would_reach_stays_a_runtime_abort() {
    // Each is an expression folding would evaluate around `fatal`; the abort
    // still happens when the program runs, with its value.
    for source in [
        "fatal(1 + 1)",
        r"
def f = fn -> fatal(2)
f()
",
        "[1, fatal(2)][0]",
    ] {
        for host in hosts() {
            let context = format!("{source:?} under {:?}", host.optimization);
            assert_eq!(
                fatal_value(host.run(source), &context),
                Value::Int(2),
                "{context}"
            );
        }
    }
}

#[test]
fn a_modules_fatal_ends_only_the_modules_run() {
    // The importer sees the resolver's report of the failed module run: an
    // ordinary error, which it can catch.
    let source = r"
def attempt = try_call(import, ['module'])
print('importer carries on')
attempt.ok
";
    for host in hosts() {
        let host = host.serving_module("fatal('module gave up')");
        let context = format!("{:?}", host.optimization);
        match host.run(source) {
            Ok(result) => assert_eq!(result.tail(), &Value::Bool(false), "{context}"),
            Err(failure) => panic!(
                "{context}: the importer should carry on, but failed: {}",
                failure.error()
            ),
        }
        assert_eq!(host.printed(), ["importer carries on"], "{context}");
    }

    // Uncaught, it fails the importer as a raised error, not an abort.
    for host in hosts() {
        let host = host.serving_module("fatal('module gave up')");
        let context = format!("{:?}", host.optimization);
        let failure = host.run("import('module')").expect_err("the import fails");
        assert_eq!(failure.kind(), RunErrorKind::Raised, "{context}");
        assert_eq!(failure.error().message(), "module gave up", "{context}");
    }
}

// ============================================================
// Arity and types
// ============================================================

#[test]
fn fatal_takes_exactly_one_argument_of_any_type() {
    for host in hosts() {
        for (call, count) in [("fatal()", 0), ("fatal(1, 2)", 2)] {
            let context = format!("{call} under {:?}", host.optimization);
            let failure = host.run(call).expect_err("a wrong arity raises");
            assert_eq!(
                failure.kind(),
                RunErrorKind::Raised,
                "{context}: an ordinary error"
            );
            assert_eq!(
                failure.error().message(),
                format!("Function fatal expects 1 argument, but was called with {count}"),
                "{context}"
            );
        }
        // A wrong arity is ordinary: it can be caught.
        let context = format!("{:?}", host.optimization);
        let result = host
            .run("try_call(fatal, []).ok")
            .unwrap_or_else(|failure| panic!("{context}: catchable: {}", failure.error()));
        assert_eq!(result.tail(), &Value::Bool(false), "{context}");
    }
}
