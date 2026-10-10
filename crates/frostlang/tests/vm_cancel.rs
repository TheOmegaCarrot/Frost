//! Tests for cancelling a run with a [`CancelToken`].
//!
//! Cancellation is a kind of abort: once a Vm notices its token is cancelled,
//! no more Frost code runs, no Frost code or native can catch it, and the run
//! fails with [`AbortReason::Cancelled`].
//! A cancellation is scoped to the Vm the token is attached to and the Vms of the
//! modules it imports, and never outlives the Vm being recycled.
//!
//! A script cancels its own run by calling the host-supplied `cancel()`, which
//! makes most tests single-threaded and deterministic; one test cancels from
//! another thread, as a host would. A cancellation that goes unnoticed leaves a
//! script spinning forever, so runs that rely on it go through [`within_deadline`].

use std::collections::BTreeMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use frostlang::compile::{CompilerOptions, compile_in_scope};
use frostlang::{
    AbortReason, Arity, CancelToken, Closure, FrostError, ImportCtx, ImportResolver, Importer,
    ImporterBuilder, ProgramResult, RunError, RunErrorKind, Value, Vm, VmFactory,
    VmRuntimeConfiguration,
};

// ============================================================
// Harness
// ============================================================

/// How long a run that should be cancelled may take before the test fails.
/// Generous: it only bounds a hang, and a passing run takes milliseconds.
const DEADLINE: Duration = Duration::from_secs(30);

/// Recurses forever: a run that reaches it ends only by being cancelled.
const SPIN: &str = r"
defn spin(n) -> spin(n + 1)
";

/// Run `vm` on its own thread, failing the test if the run does not end within
/// [`DEADLINE`]: a cancellation that is never noticed would otherwise hang it.
#[allow(clippy::result_large_err)]
fn within_deadline(vm: Vm) -> Result<ProgramResult, RunError> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || sender.send(vm.run()));
    match receiver.recv_timeout(DEADLINE) {
        Ok(outcome) => outcome,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("the run did not end within {DEADLINE:?}: the cancellation went unnoticed")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("the run panicked"),
    }
}

/// Check that `outcome` is a cancelled run, and hand back the failure.
#[track_caller]
#[allow(clippy::result_large_err)]
fn assert_cancelled(outcome: Result<ProgramResult, RunError>) -> RunError {
    let failure = match outcome {
        Ok(result) => panic!(
            "the run should be cancelled, but finished with {:?}",
            result.tail()
        ),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.kind(),
        RunErrorKind::Aborted(AbortReason::Cancelled),
        "the run should fail as cancelled, but failed with: {}",
        failure.error().message()
    );
    assert_eq!(
        failure.error().message(),
        "Execution was cancelled",
        "a cancelled run's error message"
    );
    failure
}

/// Check that `outcome` is a run that finished with `tail`.
#[track_caller]
#[allow(clippy::result_large_err)]
fn assert_finishes_with(outcome: Result<ProgramResult, RunError>, tail: Value) {
    match outcome {
        Ok(result) => assert_eq!(result.tail(), &tail, "the run's tail value"),
        Err(failure) => panic!(
            "the run should finish, but failed with: {}",
            failure.error().message()
        ),
    }
}

/// A host embedding Frost: it owns a token, keeps what scripts print,
/// and gives every script it compiles a `cancel()` that cancels its token.
struct Host {
    token: CancelToken,
    printed: Arc<Mutex<Vec<String>>>,
    importer: Arc<Importer>,
}

impl Host {
    fn new() -> Self {
        Self {
            token: CancelToken::new(),
            printed: Arc::default(),
            importer: Arc::default(),
        }
    }

    /// This host, serving `import('module')` by running `source` as a module
    /// in a child Vm. The module may call `cancel()` too.
    fn serving_module(mut self, source: &'static str) -> Self {
        let module = Module {
            source,
            cancel: cancel_native(&self.token),
        };
        self.importer = ImporterBuilder::new()
            .append_resolver(Arc::new(module))
            .build();
        self
    }

    /// `source` compiled with `cancel` and each of `extra` in scope.
    fn program(&self, source: &str, extra: &[(&str, Value)]) -> Arc<Closure> {
        let mut captures: BTreeMap<String, Value> = extra
            .iter()
            .map(|(name, value)| (name.to_string(), value.clone()))
            .collect();
        captures.insert("cancel".to_string(), cancel_native(&self.token));
        compile(source, captures)
    }

    /// A factory whose Vms print to this host and import from it.
    fn factory(&self) -> VmFactory {
        let printed = Arc::clone(&self.printed);
        let sink = move |text: &str| printed.lock().unwrap().push(text.to_string());
        Vm::factory()
            .configuration(VmRuntimeConfiguration::default().with_print_sink(Arc::new(sink)))
            .with_importer(Arc::clone(&self.importer))
    }

    /// A Vm to run `source`, cancellable by this host's token.
    fn vm(&self, source: &str) -> Vm {
        self.vm_with(source, &[])
    }

    /// [`vm`](Self::vm), with each of `extra` also in the script's scope.
    fn vm_with(&self, source: &str, extra: &[(&str, Value)]) -> Vm {
        self.factory()
            .build(self.program(source, extra))
            .with_cancel_token(self.token.clone())
    }

    /// Everything the host's scripts have printed so far.
    fn printed(&self) -> Vec<String> {
        self.printed.lock().unwrap().clone()
    }
}

/// `source` compiled and closed over `captures`, with each capture in scope.
fn compile(source: &str, captures: BTreeMap<String, Value>) -> Arc<Closure> {
    let scope: Vec<&str> = captures.keys().map(String::as_str).collect();
    compile_in_scope("cancel.frst", source, CompilerOptions::new(), &scope)
        .unwrap_or_else(|errors| panic!("{source:?} should compile:\n{}", errors.render_plain()))
        .code
        .close(captures)
        .expect("every capture is supplied")
}

/// `cancel()`: cancels `token`.
fn cancel_native(token: &CancelToken) -> Value {
    let token = token.clone();
    Value::native("cancel", Arity::Exact(0), move |_, _: &mut [Value]| {
        token.cancel();
        Ok(Value::Null)
    })
}

/// Serves every import by running `source` in a child Vm, as a resolver that
/// loads modules does.
#[derive(Debug)]
struct Module {
    source: &'static str,
    cancel: Value,
}

impl ImportResolver for Module {
    fn resolve(&self, ctx: &ImportCtx, _module_spec: &str) -> Result<Option<Value>, FrostError> {
        let closure = compile(
            self.source,
            BTreeMap::from([("cancel".to_string(), self.cancel.clone())]),
        );
        let result = ctx
            .child_factory()
            .build(closure)?
            .run()
            .map_err(RunError::into_error)?;
        Ok(Some(result.tail().clone()))
    }
}

/// A script that finishes on its own, for runs that must not be cancelled.
const FINISHES: &str = r"
defn add(a, b) -> a + b
add(1, 2)
";

/// A script that cancels its own run, which the call to `print` notices.
/// A run that makes no call after cancelling finishes normally.
const CANCELS: &str = r#"
cancel()
print("unreachable")
"#;

/// What [`FINISHES`] finishes with.
fn finished() -> Value {
    Value::Int(3)
}

// ============================================================
// Cancelling a run
// ============================================================

#[test]
fn a_token_never_cancelled_lets_the_run_finish() {
    assert_finishes_with(Host::new().vm(FINISHES).run(), finished());
}

#[test]
fn a_run_without_a_token_is_unaffected_by_cancellation() {
    // The token is cancelled, but never attached to this Vm.
    let host = Host::new();
    host.token.cancel();
    let vm = host.factory().build(host.program(FINISHES, &[]));
    assert_finishes_with(vm.run(), finished());
}

#[test]
fn cancelling_from_another_thread_stops_a_runaway_run() {
    // The script reports that it is running, then recurses forever.
    // The host cancels from this thread once the run is under way.
    let host = Host::new();
    let (started_sender, started) = mpsc::channel();
    let started_native = Value::native("started", Arity::Exact(0), move |_, _: &mut [Value]| {
        started_sender.send(()).expect("the test is waiting");
        Ok(Value::Null)
    });
    let source = format!("{SPIN}\nstarted()\nspin(0)");
    let vm = host.vm_with(&source, &[("started", started_native)]);

    let token = host.token.clone();
    let canceller = thread::spawn(move || {
        started.recv().expect("the script reports that it started");
        token.cancel();
    });
    assert_cancelled(within_deadline(vm));
    canceller.join().unwrap();
}

#[test]
fn a_cancelled_run_makes_no_further_calls() {
    let host = Host::new();
    let source = r#"
cancel()
print("after cancelling")
"#;
    assert_cancelled(host.vm(source).run());
    assert_eq!(
        host.printed(),
        Vec::<String>::new(),
        "the call to `print` after cancelling must not run"
    );
}

#[test]
fn cancellation_stops_frost_code_called_by_a_native() {
    // `transform` calls back into Frost for each element; the callback cancels
    // and then spins, inside the native's call.
    let host = Host::new();
    let source = format!(
        r#"{SPIN}
[1, 2] @ transform(fn x -> {{
    print($'element ${{x}}')
    cancel()
    spin(0)
}})
"#
    );
    let vm = host.vm(&source);
    assert_cancelled(within_deadline(vm));
    assert_eq!(
        host.printed(),
        ["element 1"],
        "the native must not call back for the second element"
    );
}

#[test]
fn try_call_cannot_catch_a_cancellation() {
    let host = Host::new();
    let source = format!(
        r"{SPIN}
try_call(fn -> {{
    cancel()
    spin(0)
}})
"
    );
    let vm = host.vm(&source);
    assert_cancelled(within_deadline(vm));
}

#[test]
fn a_native_that_swallows_a_cancellation_cannot_resume_frost() {
    // `persist(first, second)` calls `first`, swallows its error, then tries to
    // call `second` and returns a result of its own.
    // Once `first` is cancelled, the call to `second` is refused and the
    // native's result is ignored.
    let persist = Value::native("persist", Arity::Exact(2), |mut ctx, args| {
        let _ = ctx.invoke(&args[0], []);
        let _ = ctx.invoke(&args[1], []);
        Ok(Value::from("swallowed"))
    });
    let host = Host::new();
    let source = format!(
        r#"{SPIN}
persist(fn -> {{
    cancel()
    spin(0)
}}, fn -> print("resumed"))
"#
    );
    let vm = host.vm_with(&source, &[("persist", persist)]);
    assert_cancelled(within_deadline(vm));
    assert_eq!(
        host.printed(),
        Vec::<String>::new(),
        "the native must not resume Frost code after the cancellation"
    );
}

// ============================================================
// Cancelling before the run starts
// ============================================================

#[test]
fn a_run_cancelled_before_it_starts_runs_nothing() {
    let host = Host::new();
    host.token.cancel();
    let failure = assert_cancelled(host.vm(r#"print("ran")"#).run());
    assert_eq!(
        host.printed(),
        Vec::<String>::new(),
        "the script must not run"
    );
    assert_eq!(failure.fuel_consumed(), 0, "the script must make no calls");
}

#[test]
fn a_run_cancelled_before_it_starts_fails_even_if_it_makes_no_calls() {
    // Nothing in the script would notice the cancellation: the run must not start.
    let host = Host::new();
    host.token.cancel();
    assert_cancelled(host.vm("42").run());
}

#[test]
fn a_run_cancelled_before_it_starts_fails_when_run_with_arguments() {
    let host = Host::new();
    host.token.cancel();
    assert_cancelled(host.vm("42").run_with_args([]));
}

// ============================================================
// Reusing a Vm
// ============================================================

#[test]
fn a_cancelled_run_can_be_recycled() {
    // The token stays cancelled, but the recycled Vm no longer holds it.
    let host = Host::new();
    let failure = assert_cancelled(host.vm(CANCELS).run());
    let next = failure.into_idle_vm().build(host.program(FINISHES, &[]));
    assert_finishes_with(next.run(), finished());
}

#[test]
fn a_finished_runs_token_does_not_reach_the_next_run() {
    let host = Host::new();
    let result = host.vm(FINISHES).run().expect("the first run finishes");
    host.token.cancel();
    let next = result.into_idle_vm().build(host.program(FINISHES, &[]));
    assert_finishes_with(next.run(), finished());
}

#[test]
fn a_vm_cancelled_before_it_ran_can_be_reset() {
    let host = Host::new();
    host.token.cancel();
    let next = host.vm(FINISHES).reset(host.program(FINISHES, &[]));
    assert_finishes_with(next.run(), finished());
}

#[test]
fn resetting_a_vm_that_never_ran_runs_the_new_script() {
    let host = Host::new();
    let vm = host.factory().build(host.program("1", &[]));
    assert_finishes_with(vm.reset(host.program("2", &[])).run(), Value::Int(2));
}

#[test]
fn a_recycled_vm_can_take_a_new_token() {
    let host = Host::new();
    let failure = assert_cancelled(host.vm(CANCELS).run());
    let fresh = CancelToken::new();
    fresh.cancel();
    let next = failure
        .into_idle_vm()
        .build(host.program(FINISHES, &[]))
        .with_cancel_token(fresh);
    assert_cancelled(next.run());
}

#[test]
fn attaching_a_token_replaces_the_previous_one() {
    let host = Host::new();
    host.token.cancel();
    let vm = host.vm(FINISHES).with_cancel_token(CancelToken::new());
    assert_finishes_with(vm.run(), finished());
}

// ============================================================
// Group cancellation
// ============================================================

#[test]
fn one_token_cancels_every_vm_it_is_attached_to() {
    // The first Vm's script cancels the shared token, so the second Vm's run is
    // cancelled before it starts.
    let host = Host::new();
    let first = host.vm(CANCELS);
    let second = host.vm(FINISHES);
    assert_cancelled(first.run());
    assert_cancelled(second.run());
}

#[test]
fn clones_of_a_token_share_its_cancellation() {
    let token = CancelToken::new();
    let clone = token.clone();
    assert!(!token.is_cancelled(), "a new token is not cancelled");
    clone.cancel();
    assert!(
        token.is_cancelled(),
        "cancelling a clone cancels the original"
    );
    assert!(clone.is_cancelled(), "cancelling a clone cancels the clone");
}

// ============================================================
// Imported modules
// ============================================================

#[test]
fn cancellation_reaches_an_imported_module() {
    // Only the module's own Vm can stop its spinning, so it must hold the token.
    let host = Host::new().serving_module(
        r"
defn spin(n) -> spin(n + 1)
cancel()
spin(0)
",
    );
    let vm = host.vm("import('module')");
    assert_cancelled(within_deadline(vm));
}

#[test]
fn an_importer_that_catches_its_cancelled_import_is_still_cancelled() {
    // The module's failure reaches the importer as an ordinary error, which
    // `try_call` would catch; the importer must notice the cancellation itself.
    let host = Host::new().serving_module(
        r"
defn spin(n) -> spin(n + 1)
cancel()
spin(0)
",
    );
    let vm = host.vm("try_call(fn -> import('module'))");
    assert_cancelled(within_deadline(vm));
}

#[test]
fn an_import_that_finishes_after_cancelling_still_cancels_the_importer() {
    // The module cancels and then finishes, making no call that would notice;
    // the importer makes no further call either.
    let host = Host::new().serving_module("cancel()");
    assert_cancelled(host.vm("import('module')").run());
}
