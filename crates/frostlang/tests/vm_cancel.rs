//! Tests for cancelling a run with a [`CancelToken`].
//!
//! Cancellation is a kind of abort: once a Vm notices its token is cancelled,
//! no more Frost code runs, no Frost code or native can catch it, and the run
//! fails with [`AbortReason::Cancelled`].
//! A cancellation is scoped to the Vm the token is attached to and the Vms of the
//! modules it imports, and never outlives the Vm being recycled.
//!
//! A script cancels its own run by calling the host-supplied `cancel()`, which
//! makes most tests single-threaded and deterministic; a few cancel from
//! another thread, as a host would. A cancellation that goes unnoticed leaves a
//! script spinning forever, so runs that rely on it go through [`within_deadline`].

use std::collections::BTreeMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use frostlang::compile::{CompilerOptions, compile_in_scope};
use frostlang::native::Function;
use frostlang::{
    AbortReason, CancelToken, Closure, FrostError, ImportCtx, ImportResolver, Importer,
    ImporterBuilder, ProgramResult, RunError, RunErrorKind, Value, Vm, VmFactory,
    VmRuntimeConfiguration, frost_closure,
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
    frost_closure!("cancel", move || token.cancel())
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

/// A script that cancels its own run, which `cancel` returning notices.
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
    let started_native = frost_closure!("started", move || {
        started_sender.send(()).expect("the test is waiting");
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
    // `map` calls back into Frost for each element; the callback cancels
    // and then spins, inside the native's call.
    let host = Host::new();
    let source = format!(
        r#"{SPIN}
[1, 2] @ map(fn x -> {{
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
    let persist = frost_closure!(
        "persist",
        |#[frost(ctx)] ctx, first: Function, second: Function| {
            let _ = ctx.invoke(&first, []);
            let _ = ctx.invoke(&second, []);
            "swallowed"
        }
    );
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

// ============================================================
// A native checking for an abort
// ============================================================

/// What each check of `NativeCtx::abort_reason` saw, in order.
type Observed = Arc<Mutex<Vec<Option<AbortReason>>>>;

/// `check()`: records what `NativeCtx::abort_reason` reports.
fn check_native(observed: &Observed) -> Value {
    let observed = Arc::clone(observed);
    frost_closure!("check", move |#[frost(ctx)] ctx| {
        observed.lock().unwrap().push(ctx.abort_reason());
    })
}

/// `cancel_and_stop()`: checks, cancels `token`, checks again, and then stops
/// early, returning a value of its own.
fn cancel_and_stop_native(token: &CancelToken, observed: &Observed) -> Value {
    let (token, observed) = (token.clone(), Arc::clone(observed));
    frost_closure!("cancel_and_stop", move |#[frost(ctx)] ctx| {
        observed.lock().unwrap().push(ctx.abort_reason());
        token.cancel();
        observed.lock().unwrap().push(ctx.abort_reason());
        "stopped early"
    })
}

#[test]
fn a_native_sees_no_abort_while_the_run_continues() {
    let observed = Observed::default();
    let host = Host::new();
    let vm = host.vm_with("check()", &[("check", check_native(&observed))]);
    assert_finishes_with(vm.run(), Value::Null);
    assert_eq!(
        *observed.lock().unwrap(),
        [None],
        "with a token not cancelled"
    );

    let observed = Observed::default();
    let vm = host
        .factory()
        .build(host.program("check()", &[("check", check_native(&observed))]));
    assert_finishes_with(vm.run(), Value::Null);
    assert_eq!(*observed.lock().unwrap(), [None], "with no token");
}

#[test]
fn a_native_sees_a_cancellation_before_any_call_notices_it() {
    // The native cancels and checks again with no call in between, and then
    // returns normally. The run is cancelled however the native returns.
    let observed = Observed::default();
    let host = Host::new();
    let stop = cancel_and_stop_native(&host.token, &observed);
    let vm = host.vm_with("cancel_and_stop()", &[("cancel_and_stop", stop)]);
    assert_cancelled(vm.run());
    assert_eq!(
        *observed.lock().unwrap(),
        [None, Some(AbortReason::Cancelled)],
        "before and after cancelling"
    );
}

#[test]
fn a_native_that_raises_on_seeing_a_cancellation_still_ends_the_run_cancelled() {
    // Even caught by `try_call`, the native's own error gives way to the cancellation.
    let host = Host::new();
    let token = host.token.clone();
    let give_up = frost_closure!(
        "give_up",
        move |#[frost(ctx)] ctx| -> Result<Value, FrostError> {
            token.cancel();
            match ctx.abort_reason() {
                Some(_) => Err(FrostError::from_static("gave up")),
                None => Ok(Value::from("not cancelled")),
            }
        }
    );
    let vm = host.vm_with("try_call(give_up)", &[("give_up", give_up)]);
    assert_cancelled(vm.run());
}

#[test]
fn a_native_waiting_on_cancellation_from_another_thread_stops() {
    // The native reports that it is running, then waits, making no call, until
    // it sees the host's cancellation.
    let host = Host::new();
    let (started_sender, started) = mpsc::channel();
    let wait = frost_closure!("wait", move |#[frost(ctx)] ctx| {
        started_sender.send(()).expect("the test is waiting");
        while ctx.abort_reason().is_none() {
            thread::sleep(Duration::from_millis(1));
        }
    });
    let vm = host.vm_with("wait()", &[("wait", wait)]);

    let token = host.token.clone();
    let canceller = thread::spawn(move || {
        started.recv().expect("the native reports that it started");
        token.cancel();
    });
    assert_cancelled(within_deadline(vm));
    canceller.join().unwrap();
}

#[test]
fn a_native_sees_a_cancellation_made_inside_its_call_back_into_frost() {
    // The callback cancels, and `cancel` returning latches the cancellation,
    // so `outer`'s check after the callback sees it.
    let observed = Observed::default();
    let outer = {
        let observed = Arc::clone(&observed);
        frost_closure!("outer", move |#[frost(ctx)] ctx, callback: Function| {
            observed.lock().unwrap().push(ctx.abort_reason());
            let _ = ctx.invoke(&callback, []);
            observed.lock().unwrap().push(ctx.abort_reason());
            "finished"
        })
    };
    let host = Host::new();
    let vm = host.vm_with("outer(fn -> cancel())", &[("outer", outer)]);
    assert_cancelled(vm.run());
    assert_eq!(
        *observed.lock().unwrap(),
        [None, Some(AbortReason::Cancelled)],
        "before and after the callback"
    );
}

#[test]
fn a_native_nested_in_another_sees_a_cancellation_and_so_does_the_outer() {
    // `cancel_and_stop` runs inside the callback `outer` invokes. Once it has
    // seen the cancellation, `outer` sees it too.
    let observed = Observed::default();
    let outer = {
        let observed = Arc::clone(&observed);
        frost_closure!("outer", move |#[frost(ctx)] ctx, callback: Function| {
            let _ = ctx.invoke(&callback, []);
            observed.lock().unwrap().push(ctx.abort_reason());
            "finished"
        })
    };
    let host = Host::new();
    let stop = cancel_and_stop_native(&host.token, &observed);
    let vm = host.vm_with(
        "outer(fn -> cancel_and_stop())",
        &[("outer", outer), ("cancel_and_stop", stop)],
    );
    assert_cancelled(vm.run());
    assert_eq!(
        *observed.lock().unwrap(),
        [
            None,
            Some(AbortReason::Cancelled),
            Some(AbortReason::Cancelled)
        ],
        "the inner native before and after cancelling, then the outer"
    );
}

// ============================================================
// A native holding the run's token
// ============================================================

/// `has_token()`: whether the run has a cancel token, as a native sees it.
fn has_token_native() -> Value {
    frost_closure!("has_token", |#[frost(ctx)] ctx| ctx
        .cancel_token()
        .is_some())
}

#[test]
fn a_native_sees_whether_the_run_has_a_token() {
    let host = Host::new();
    let vm = host.vm_with("has_token()", &[("has_token", has_token_native())]);
    assert_finishes_with(vm.run(), Value::Bool(true));

    let vm = host
        .factory()
        .build(host.program("has_token()", &[("has_token", has_token_native())]));
    assert_finishes_with(vm.run(), Value::Bool(false));
}

#[test]
fn a_native_holds_the_runs_own_token() {
    // Cancelling the native's clone cancels the run, as the host's would.
    let cancel_own = frost_closure!("cancel_own", |#[frost(ctx)] ctx| {
        ctx.cancel_token()
            .expect("the run has a token")
            .clone()
            .cancel();
    });
    let host = Host::new();
    let vm = host.vm_with(
        r#"
        cancel_own()
        print("unreachable")
        "#,
        &[("cancel_own", cancel_own)],
    );
    assert_cancelled(vm.run());
    assert!(
        host.token.is_cancelled(),
        "the host's token is the same token"
    );
    assert!(host.printed().is_empty());
}

#[test]
fn a_native_can_hand_the_token_to_work_it_cannot_check_from() {
    // The native waits on a worker of its own, which only the token it was
    // handed can stop: as a call into a library that runs long might.
    let host = Host::new();
    let (started_sender, started) = mpsc::channel();
    let work = frost_closure!("work", move |#[frost(ctx)] ctx| {
        let token = ctx.cancel_token().expect("the run has a token").clone();
        let started_sender = started_sender.clone();
        let worker = thread::spawn(move || {
            started_sender.send(()).expect("the test is waiting");
            while !token.is_cancelled() {
                thread::sleep(Duration::from_millis(1));
            }
        });
        worker.join().expect("the worker ends");
    });
    // The native returning notices the cancellation, with no call after it.
    let vm = host.vm_with("work()", &[("work", work)]);

    let token = host.token.clone();
    let canceller = thread::spawn(move || {
        started.recv().expect("the worker reports that it started");
        token.cancel();
    });
    assert_cancelled(within_deadline(vm));
    canceller.join().unwrap();
}

#[test]
fn a_native_returning_notices_a_cancellation_made_while_it_ran() {
    // The native cancels and returns without checking, as the script's last
    // call: its return is where the run notices, whatever it returns.
    let host = Host::new();
    let returning_ok = {
        let token = host.token.clone();
        frost_closure!("returning_ok", move || {
            token.cancel();
            "finished"
        })
    };
    let vm = host.vm_with("returning_ok()", &[("returning_ok", returning_ok)]);
    assert_cancelled(vm.run());

    let host = Host::new();
    let returning_error = {
        let token = host.token.clone();
        frost_closure!("returning_error", move || -> Result<(), FrostError> {
            token.cancel();
            Err(FrostError::from_static("failed as it was interrupted"))
        })
    };
    let vm = host.vm_with(
        "try_call(returning_error)",
        &[("returning_error", returning_error)],
    );
    // The native's own error is overridden, and try_call cannot catch it.
    assert_cancelled(vm.run());

    let host = Host::new();
    let returning_error = {
        let token = host.token.clone();
        frost_closure!("returning_error", move || -> Result<(), FrostError> {
            token.cancel();
            Err(FrostError::from_static("failed as it was interrupted"))
        })
    };
    let vm = host.vm_with("returning_error()", &[("returning_error", returning_error)]);
    assert_cancelled(vm.run());
}

#[test]
fn a_native_in_an_imported_module_holds_the_importers_token() {
    // The module's `cancel` cancels through the token its own run holds,
    // which is the importer's.
    let cancel_own = frost_closure!("cancel", |#[frost(ctx)] ctx| {
        ctx.cancel_token()
            .expect("the module's run has a token")
            .clone()
            .cancel();
    });
    let mut host = Host::new();
    host.importer = ImporterBuilder::new()
        .append_resolver(Arc::new(Module {
            source: "cancel()",
            cancel: cancel_own,
        }))
        .build();
    let vm = host.vm("import('module')");
    assert_cancelled(vm.run());
    assert!(host.token.is_cancelled());
}

// ============================================================
// A native checking for an abort to stop
// ============================================================

/// `count_until_aborted(n)`: counts up to `n`, checking for an abort after
/// each count, and returns the count reached; records it in `reached`.
fn count_until_aborted_native(reached: &Arc<Mutex<Option<i64>>>) -> Value {
    let reached = Arc::clone(reached);
    frost_closure!("count_until_aborted", move |#[frost(ctx)] ctx,
                                                n: i64|
          -> Result<i64, FrostError> {
        for count in 1..=n {
            *reached.lock().unwrap() = Some(count);
            ctx.check_abort()?;
        }
        Ok(n)
    })
}

#[test]
fn check_abort_passes_while_the_run_goes_on() {
    let reached = Arc::default();
    let host = Host::new();
    let vm = host.vm_with(
        "count_until_aborted(5)",
        &[("count_until_aborted", count_until_aborted_native(&reached))],
    );
    assert_finishes_with(vm.run(), Value::Int(5));
    assert_eq!(*reached.lock().unwrap(), Some(5));
}

#[test]
fn check_abort_fails_once_the_run_is_cancelled() {
    // The native cancels as it counts 2, and stops at the check right after.
    let reached = Arc::new(Mutex::new(None));
    let host = Host::new();
    let cancel_at_two = {
        let (token, reached) = (host.token.clone(), Arc::clone(&reached));
        frost_closure!(
            "cancel_at_two",
            move |#[frost(ctx)] ctx| -> Result<i64, FrostError> {
                for count in 1..=5 {
                    *reached.lock().unwrap() = Some(count);
                    if count == 2 {
                        token.cancel();
                    }
                    ctx.check_abort()?;
                }
                Ok(5)
            }
        )
    };
    let vm = host.vm_with("cancel_at_two()", &[("cancel_at_two", cancel_at_two)]);
    assert_cancelled(vm.run());
    assert_eq!(
        *reached.lock().unwrap(),
        Some(2),
        "stopped at the check after cancelling"
    );
}

#[test]
fn check_abort_fails_with_the_error_the_run_fails_with() {
    let seen = Arc::new(Mutex::new(None));
    let observed = Arc::clone(&seen);
    let cancel_and_check = frost_closure!("cancel_and_check", move |#[frost(ctx)] ctx| {
        ctx.cancel_token().expect("the run has a token").cancel();
        *observed.lock().unwrap() =
            Some(ctx.check_abort().map_err(|err| err.message().into_owned()));
    });
    let host = Host::new();
    let vm = host.vm_with(
        "cancel_and_check()",
        &[("cancel_and_check", cancel_and_check)],
    );
    let failure = assert_cancelled(vm.run());
    assert_eq!(
        *seen.lock().unwrap(),
        Some(Err(failure.error().message().into_owned()))
    );
}

#[test]
fn check_abort_fails_with_fuel_exhaustion() {
    // A run out of fuel aborts at a call; a native it calls back from sees it.
    let seen = Arc::new(Mutex::new(None));
    let observed = Arc::clone(&seen);
    let after_callback = frost_closure!(
        "after_callback",
        move |#[frost(ctx)] ctx, callback: Function| {
            let _ = ctx.invoke(&callback, []);
            *observed.lock().unwrap() =
                Some(ctx.check_abort().map_err(|err| err.message().into_owned()));
        }
    );
    let host = Host::new();
    let program = host.program(
        r"
        defn spin(n) -> spin(n + 1)
        after_callback(fn -> spin(0))
        ",
        &[("after_callback", after_callback)],
    );
    let vm = Vm::factory()
        .configuration(VmRuntimeConfiguration::default().with_fuel(std::num::NonZeroU64::new(100)))
        .build(program);
    let failure = vm.run().expect_err("the run runs out of fuel");
    assert_eq!(
        failure.kind(),
        RunErrorKind::Aborted(AbortReason::FuelExhausted)
    );
    assert_eq!(
        *seen.lock().unwrap(),
        Some(Err(failure.error().message().into_owned())),
        "the native saw the error the run failed with"
    );
}
