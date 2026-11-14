//! Waiting from each kind of thread a host might run a Vm on, including
//! threads of the host's own Tokio runtime: a wait neither panics nor
//! deadlocks, and stays cancellable.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use frostlang::{AbortReason, CancelToken, FrostResult, Value, Vm, frost_closure};
use frostlang_tokio::Runtime;

use crate::common::{Ended, expect_message, run, vm, within_deadline};

/// Where a test runs its Vm.
#[derive(Clone, Copy, Debug)]
enum Context {
    /// A thread of its own, outside any Tokio runtime.
    PlainThread,
    /// A `spawn_blocking` task of a host's multi-threaded runtime.
    SpawnBlocking,
    /// A task on a worker thread of a host's multi-threaded runtime.
    MultiThreadWorker,
    /// The thread inside `block_on` of a host's current-thread runtime.
    CurrentThreadBlockOn,
}

const CONTEXTS: [Context; 4] = [
    Context::PlainThread,
    Context::SpawnBlocking,
    Context::MultiThreadWorker,
    Context::CurrentThreadBlockOn,
];

/// A host's own runtime, built by `builder`.
fn host_runtime(mut builder: tokio::runtime::Builder) -> tokio::runtime::Runtime {
    builder
        .enable_all()
        .build()
        .expect("the host runtime starts")
}

/// A host's own multi-threaded runtime.
fn multi_thread_host() -> tokio::runtime::Runtime {
    host_runtime(tokio::runtime::Builder::new_multi_thread())
}

/// Runs `vm` in `context`, on a thread the test bounds with a deadline.
fn run_in(context: Context, vm: Vm) -> Ended {
    within_deadline(move || match context {
        Context::PlainThread => run(vm),
        Context::SpawnBlocking => {
            let host = multi_thread_host();
            host.block_on(host.spawn_blocking(move || run(vm)))
                .expect("the blocking task finishes")
        }
        Context::MultiThreadWorker => {
            let host = multi_thread_host();
            host.block_on(host.spawn(async move { run(vm) }))
                .expect("the worker task finishes")
        }
        Context::CurrentThreadBlockOn => {
            host_runtime(tokio::runtime::Builder::new_current_thread())
                .block_on(async move { run(vm) })
        }
    })
}

#[test]
fn a_wait_finishes_in_every_context() {
    for context in CONTEXTS {
        let runtime = Runtime::new();
        let wait = frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
            let answer = runtime.run(ctx, async {
                tokio::time::sleep(Duration::from_millis(5)).await;
                42
            })?;
            Ok(Value::Int(answer))
        });
        let ended = run_in(
            context,
            vm("import('app').wait()", vec![("wait", wait)], None),
        );
        assert_eq!(ended, Ended::Finished(Value::Int(42)), "in {context:?}");
    }
}

#[test]
fn a_wait_is_cancellable_in_every_context() {
    for context in CONTEXTS {
        let runtime = Runtime::new();
        let token = CancelToken::new();
        let (started_sender, started) = mpsc::channel();
        let wait = frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
            let started = started_sender.clone();
            runtime.run(ctx, async move {
                started.send(()).expect("the test is listening");
                std::future::pending::<()>().await;
            })?;
            Ok(Value::Null)
        });
        let vm = vm("import('app').wait()", vec![("wait", wait)], Some(&token));
        let canceller = thread::spawn(move || {
            expect_message(&started, "the future started");
            token.cancel();
        });
        let ended = run_in(context, vm);
        canceller.join().expect("the canceller finishes");
        assert_eq!(
            ended,
            Ended::Aborted(AbortReason::Cancelled),
            "in {context:?}"
        );
    }
}

#[test]
fn dropping_or_shutting_down_a_running_runtime_in_an_async_context_does_not_panic() {
    for shut_down_first in [false, true] {
        let runtime = Runtime::new();
        let warm_up = {
            let runtime = runtime.clone();
            frost_closure!("app.wait", move |#[frost(ctx)] ctx| -> FrostResult {
                runtime.run(ctx, async {})?;
                Ok(Value::Null)
            })
        };
        let ended = run_in(
            Context::PlainThread,
            vm("import('app').wait()", vec![("wait", warm_up)], None),
        );
        assert_eq!(ended, Ended::Finished(Value::Null), "the warm-up run");

        within_deadline(move || {
            let host = multi_thread_host();
            host.block_on(async move {
                if shut_down_first {
                    runtime.shutdown();
                }
                drop(runtime);
            });
        });
    }
}

#[test]
fn the_last_clone_may_drop_on_one_of_the_runtimes_own_threads() {
    // A task on the runtime holds the last clone, and drops it once the test
    // has dropped every other: the runtime then shuts down from its own thread.
    let runtime = Runtime::new();
    let (release_sender, release) = tokio::sync::oneshot::channel::<()>();
    let (dropped_sender, dropped) = mpsc::channel();
    let detach = {
        let runtime = runtime.clone();
        let parked = std::sync::Mutex::new(Some((release, dropped_sender)));
        frost_closure!("app.detach", move |#[frost(ctx)] ctx| -> FrostResult {
            let (release, dropped) = parked
                .lock()
                .expect("not poisoned")
                .take()
                .expect("called once");
            let last_clone = runtime.clone();
            runtime.run(ctx, async move {
                tokio::spawn(async move {
                    let _ = release.await;
                    drop(last_clone);
                    dropped.send(()).expect("the test is listening");
                });
            })?;
            Ok(Value::Null)
        })
    };
    let ended = run_in(
        Context::PlainThread,
        vm("import('app').detach()", vec![("detach", detach)], None),
    );
    assert_eq!(ended, Ended::Finished(Value::Null));
    drop(runtime);
    release_sender
        .send(())
        .expect("the detached task is waiting");
    expect_message(
        &dropped,
        "the detached task dropped the last clone without panicking",
    );
}
