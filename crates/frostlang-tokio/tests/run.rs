//! `Runtime::run`: a native waiting for one future, and every way that wait
//! can end.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use frostlang::{AbortReason, CancelToken, FrostResult, Value, frost_closure};
use frostlang_tokio::Runtime;

use crate::common::{DropSignal, Ended, expect_message, run, vm, within_deadline};

/// A script calling `app.wait()` and nothing else.
const WAIT: &str = "import('app').wait()";

/// The error a native named `app.wait` fails with when its runtime has shut
/// down.
const SHUT_DOWN: &str = "Function app.wait could not finish: its async runtime has shut down";

/// `app.wait`: runs a future that reports, on `started`, that it is running,
/// then never finishes, reporting on `dropped` when it is dropped.
fn wait_forever(runtime: &Runtime, started: mpsc::Sender<()>, dropped: mpsc::Sender<()>) -> Value {
    let runtime = runtime.clone();
    frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
        let started = started.clone();
        let dropped = DropSignal(dropped.clone());
        runtime.run(ctx, async move {
            let _dropped = dropped;
            started.send(()).expect("the test is listening");
            std::future::pending::<()>().await;
        })?;
        Ok(Value::Null)
    })
}

#[test]
fn a_native_gets_its_futures_output() {
    let runtime = Runtime::new();
    let wait = frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
        let answer = runtime.run(ctx, async { 6 * 7 })?;
        Ok(Value::Int(answer))
    });
    let ended = within_deadline(|| run(vm(WAIT, vec![("wait", wait)], None)));
    assert_eq!(ended, Ended::Finished(Value::Int(42)));
}

#[test]
fn a_future_runs_on_the_runtimes_threads_with_tokios_timers_and_io() {
    // The Vm runs on `within_deadline`'s unnamed thread; the runtime's
    // threads have a name of their own.
    let runtime = Runtime::new();
    let wait = frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
        let vm_thread = thread::current().id();
        let (thread_name, same_thread, port) = runtime.run(ctx, async move {
            tokio::time::sleep(Duration::from_millis(1)).await;
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("a future can open a socket");
            let port = listener
                .local_addr()
                .expect("a bound socket has an address")
                .port();
            let current = thread::current();
            (
                current.name().map(str::to_string),
                current.id() == vm_thread,
                port,
            )
        })?;
        assert_eq!(
            thread_name.as_deref(),
            Some("frostlang-tokio"),
            "the future's thread"
        );
        assert!(!same_thread, "the future must not run on the Vm's thread");
        assert_ne!(port, 0, "the future's socket is bound to a real port");
        Ok(Value::Null)
    });
    let ended = within_deadline(|| run(vm(WAIT, vec![("wait", wait)], None)));
    assert_eq!(ended, Ended::Finished(Value::Null));
}

#[test]
fn one_runtime_serves_many_runs_and_clones() {
    let runtime = Runtime::new();
    let clone = runtime.clone();
    let double = frost_closure!("app.double", move |#[frost(ctx)] ctx,
                                                    n: i64|
          -> FrostResult {
        Ok(Value::Int(runtime.run(ctx, async move { n * 2 })?))
    });
    let add = frost_closure!("app.add", move |#[frost(ctx)] ctx, n: i64| -> FrostResult {
        Ok(Value::Int(clone.run(ctx, async move { n + 1 })?))
    });
    let source = r"
    def app = import('app')
    [1, 2, 3] @ map(fn n -> app.add(app.double(n)))
    ";
    let ended =
        within_deadline(move || run(vm(source, vec![("double", double), ("add", add)], None)));
    assert_eq!(
        ended,
        Ended::Finished(Value::from(vec![
            Value::Int(3),
            Value::Int(5),
            Value::Int(7)
        ]))
    );
}

#[test]
fn a_run_without_a_cancel_token_still_waits_for_its_future() {
    let runtime = Runtime::new();
    let wait = frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
        assert!(ctx.cancel_token().is_none(), "the run has no token");
        let text = runtime.run(ctx, async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            "slept"
        })?;
        Ok(Value::from(text))
    });
    let ended = within_deadline(|| run(vm(WAIT, vec![("wait", wait)], None)));
    assert_eq!(ended, Ended::Finished(Value::from("slept")));
}

#[test]
fn a_run_cancelled_before_the_wait_never_starts_the_future() {
    let runtime = Runtime::new();
    let token = CancelToken::new();
    let (started_sender, started) = mpsc::channel();
    let wait = {
        let token = token.clone();
        frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
            token.cancel();
            let started = started_sender.clone();
            let outcome = runtime.run(ctx, async move {
                let _ = started.send(());
            });
            let err = outcome.expect_err("a cancelled run's wait fails at once");
            assert_eq!(err.message(), "Execution was cancelled");
            Ok(Value::Null)
        })
    };
    let ended = within_deadline(move || run(vm(WAIT, vec![("wait", wait)], Some(&token))));
    assert_eq!(ended, Ended::Aborted(AbortReason::Cancelled));
    assert_eq!(
        started.try_recv(),
        Err(mpsc::TryRecvError::Disconnected),
        "the future must never have run, and must have been dropped"
    );
}

#[test]
fn cancelling_during_the_wait_ends_the_run_and_drops_the_future() {
    let runtime = Runtime::new();
    let token = CancelToken::new();
    let (started_sender, started) = mpsc::channel();
    let (dropped_sender, dropped) = mpsc::channel();
    let wait = wait_forever(&runtime, started_sender, dropped_sender);
    let vm = vm(WAIT, vec![("wait", wait)], Some(&token));
    let canceller = {
        let token = token.clone();
        thread::spawn(move || {
            expect_message(&started, "the future started");
            token.cancel();
        })
    };
    let ended = within_deadline(move || run(vm));
    canceller.join().expect("the canceller finishes");
    assert_eq!(ended, Ended::Aborted(AbortReason::Cancelled));
    expect_message(&dropped, "the cancelled future was dropped");
}

#[test]
fn a_native_that_catches_the_cancellation_still_ends_the_run_as_cancelled() {
    // The native ignores the error and returns a value; the run is cancelled
    // all the same.
    let runtime = Runtime::new();
    let token = CancelToken::new();
    let (started_sender, started) = mpsc::channel();
    let wait = {
        let token = token.clone();
        frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
            let token = token.clone();
            let started = started_sender.clone();
            let outcome = runtime.run(ctx, async move {
                started.send(()).expect("the test is listening");
                std::future::pending::<()>().await;
            });
            assert!(outcome.is_err(), "the wait fails once cancelled");
            assert!(token.is_cancelled());
            Ok(Value::from("ignored"))
        })
    };
    let vm = vm(WAIT, vec![("wait", wait)], Some(&token));
    let canceller = thread::spawn(move || {
        expect_message(&started, "the future started");
        token.cancel();
    });
    let ended = within_deadline(move || run(vm));
    canceller.join().expect("the canceller finishes");
    assert_eq!(ended, Ended::Aborted(AbortReason::Cancelled));
}

// That the wait unregisters its cancel callback once done has no observable
// effect but memory, so no test here can see it.
#[test]
fn cancelling_after_the_wait_does_not_hang() {
    let runtime = Runtime::new();
    let token = CancelToken::new();
    let wait = {
        let token = token.clone();
        frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
            let answer = runtime.run(ctx, async { 42 })?;
            assert_eq!(answer, 42, "the finished future's output");
            token.cancel();
            Ok(Value::Int(answer))
        })
    };
    let ended = within_deadline(move || run(vm(WAIT, vec![("wait", wait)], Some(&token))));
    assert_eq!(
        ended,
        Ended::Aborted(AbortReason::Cancelled),
        "the run notices the cancellation as the native returns"
    );
}

#[test]
fn a_panicking_future_fails_its_native_and_leaves_the_runtime_usable() {
    let runtime = Runtime::new();
    let explode = {
        let runtime = runtime.clone();
        frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
            runtime.run(ctx, async {
                panic!("the future exploded");
            })?;
            Ok(Value::Null)
        })
    };
    let ended = within_deadline(|| run(vm(WAIT, vec![("wait", explode)], None)));
    assert_eq!(
        ended,
        Ended::Raised(String::from(
            "Function app.wait failed: its async task panicked: the future exploded"
        ))
    );

    let formatted = frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
        runtime.run(ctx, async {
            let reason = "formatted";
            panic!("a {reason} message");
        })?;
        Ok(Value::Null)
    });
    let ended = within_deadline(|| run(vm(WAIT, vec![("wait", formatted)], None)));
    assert_eq!(
        ended,
        Ended::Raised(String::from(
            "Function app.wait failed: its async task panicked: a formatted message"
        )),
        "the runtime keeps working after a panic, and a formatted message is reported"
    );
}

#[test]
fn a_panic_without_a_text_message_still_fails_its_native() {
    let runtime = Runtime::new();
    let wait = frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
        runtime.run(ctx, async {
            std::panic::panic_any(7_i32);
        })?;
        Ok(Value::Null)
    });
    let ended = within_deadline(|| run(vm(WAIT, vec![("wait", wait)], None)));
    assert_eq!(
        ended,
        Ended::Raised(String::from(
            "Function app.wait failed: its async task panicked: a panic without a message"
        ))
    );
}

#[test]
fn shutting_down_during_the_wait_fails_the_native_and_drops_the_future() {
    let runtime = Runtime::new();
    let (started_sender, started) = mpsc::channel();
    let (dropped_sender, dropped) = mpsc::channel();
    let wait = wait_forever(&runtime, started_sender, dropped_sender);
    let vm = vm(WAIT, vec![("wait", wait)], None);
    let stopper = thread::spawn(move || {
        expect_message(&started, "the future started");
        runtime.shutdown();
    });
    let ended = within_deadline(move || run(vm));
    stopper.join().expect("the shutdown finishes");
    assert_eq!(ended, Ended::Raised(String::from(SHUT_DOWN)));
    expect_message(&dropped, "the future was dropped as the runtime shut down");
}

#[test]
fn a_shut_down_runtime_fails_at_once_without_starting_the_future() {
    for started_first in [false, true] {
        let runtime = Runtime::new();
        if started_first {
            let warm_up = {
                let runtime = runtime.clone();
                frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
                    runtime.run(ctx, async {})?;
                    Ok(Value::Null)
                })
            };
            let ended = within_deadline(|| run(vm(WAIT, vec![("wait", warm_up)], None)));
            assert_eq!(ended, Ended::Finished(Value::Null), "the warm-up run");
        }
        // Twice: shutting down again is harmless.
        runtime.shutdown();
        runtime.shutdown();

        let (started_sender, started) = mpsc::channel();
        let wait = frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
            let started = started_sender.clone();
            runtime.run(ctx, async move {
                let _ = started.send(());
            })?;
            Ok(Value::Null)
        });
        let ended = within_deadline(|| run(vm(WAIT, vec![("wait", wait)], None)));
        assert_eq!(
            ended,
            Ended::Raised(String::from(SHUT_DOWN)),
            "after shutting down (having started first: {started_first})"
        );
        assert_eq!(
            started.try_recv(),
            Err(mpsc::TryRecvError::Disconnected),
            "the future must never have run (having started first: {started_first})"
        );
    }
}

#[test]
fn shutting_down_one_clone_shuts_down_every_clone() {
    let runtime = Runtime::new();
    let clone = runtime.clone();
    runtime.shutdown();
    let wait = frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
        clone.run(ctx, async {})?;
        Ok(Value::Null)
    });
    let ended = within_deadline(|| run(vm(WAIT, vec![("wait", wait)], None)));
    assert_eq!(ended, Ended::Raised(String::from(SHUT_DOWN)));
}

/// The state `runtime`'s `Debug` output reports.
fn state(runtime: &Runtime) -> String {
    let debug = format!("{runtime:?}");
    ["not started", "running", "shut down"]
        .into_iter()
        .find(|state| debug.contains(&format!("{state:?}")))
        .unwrap_or_else(|| panic!("the Debug output should give a state: {debug}"))
        .to_string()
}

#[test]
fn a_runtime_starts_its_threads_only_to_run_a_future() {
    let runtime = Runtime::new();
    let clone = runtime.clone();
    assert_eq!(state(&runtime), "not started", "a new runtime");

    // A run cancelled before it waits starts nothing.
    let token = CancelToken::new();
    let cancelled_first = {
        let runtime = runtime.clone();
        let token = token.clone();
        frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
            token.cancel();
            runtime.run(ctx, async {})?;
            Ok(Value::Null)
        })
    };
    let ended =
        within_deadline(move || run(vm(WAIT, vec![("wait", cancelled_first)], Some(&token))));
    assert_eq!(ended, Ended::Aborted(AbortReason::Cancelled));
    assert_eq!(
        state(&clone),
        "not started",
        "after a run cancelled before its wait"
    );

    let wait = {
        let runtime = runtime.clone();
        frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
            runtime.run(ctx, async {})?;
            Ok(Value::Null)
        })
    };
    let ended = within_deadline(|| run(vm(WAIT, vec![("wait", wait)], None)));
    assert_eq!(ended, Ended::Finished(Value::Null));
    assert_eq!(state(&clone), "running", "every clone, after a run");

    runtime.shutdown();
    assert_eq!(state(&clone), "shut down", "every clone, after a shutdown");

    let never_started = Runtime::new();
    never_started.shutdown();
    assert_eq!(
        state(&never_started),
        "shut down",
        "shut down before any run"
    );
}

#[test]
fn a_runtime_sized_by_its_builder_runs_futures() {
    let runtime = Runtime::builder()
        .worker_threads(std::num::NonZeroUsize::MIN)
        .build();
    let wait = frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
        let name = runtime.run(ctx, async { thread::current().name().map(str::to_string) })?;
        Ok(Value::from(name.unwrap_or_default()))
    });
    let ended = within_deadline(|| run(vm(WAIT, vec![("wait", wait)], None)));
    assert_eq!(ended, Ended::Finished(Value::from("frostlang-tokio")));
}
