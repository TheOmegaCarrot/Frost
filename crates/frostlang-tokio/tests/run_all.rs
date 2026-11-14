//! `Runtime::run_all`: a native waiting for a batch of futures, in order,
//! with a bounded number in progress, under one cancellation.

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use frostlang::{AbortReason, CancelToken, FrostResult, Value, frost_closure};
use frostlang_tokio::Runtime;

use crate::common::{DEADLINE, DropSignal, Ended, expect_message, run, vm, within_deadline};

/// A script calling `app.batch()` and nothing else.
const BATCH: &str = "import('app').batch()";

/// `concurrency` as the non-zero count `run_all` takes.
fn limit(concurrency: usize) -> NonZeroUsize {
    NonZeroUsize::new(concurrency).expect("a test's concurrency is positive")
}

#[test]
fn outputs_come_in_input_order_whatever_order_futures_finish_in() {
    let runtime = Runtime::new();
    let batch = frost_closure!("app.batch", move |#[frost(ctx)] ctx| -> FrostResult {
        // Each later future finishes sooner.
        let futures = (0..5_u64).map(|n| async move {
            tokio::time::sleep(Duration::from_millis(50 - n * 10)).await;
            Value::Int(n.try_into().expect("small"))
        });
        Ok(Value::from(runtime.run_all(ctx, futures, limit(5))?))
    });
    let ended = within_deadline(|| run(vm(BATCH, vec![("batch", batch)], None)));
    let expected = (0..5).map(Value::Int).collect::<Vec<_>>();
    assert_eq!(ended, Ended::Finished(Value::from(expected)));
}

#[test]
fn a_slow_future_does_not_hold_back_those_after_it() {
    // With two at once, the first waits until the third starts, which it can
    // only once the second has finished and freed its place.
    let runtime = Runtime::new();
    let batch = frost_closure!("app.batch", move |#[frost(ctx)] ctx| -> FrostResult {
        let (release, released) = tokio::sync::oneshot::channel::<()>();
        let (mut release, mut released) = (Some(release), Some(released));
        let futures = (0..3).map(move |n| {
            let waits = if n == 0 { released.take() } else { None };
            let releases = if n == 2 { release.take() } else { None };
            async move {
                if let Some(released) = waits {
                    released.await.expect("the third future releases the first");
                }
                if let Some(release) = releases {
                    release.send(()).expect("the first future is waiting");
                }
                Value::Int(n)
            }
        });
        Ok(Value::from(runtime.run_all(ctx, futures, limit(2))?))
    });
    let ended = within_deadline(|| run(vm(BATCH, vec![("batch", batch)], None)));
    let expected = (0..3).map(Value::Int).collect::<Vec<_>>();
    assert_eq!(ended, Ended::Finished(Value::from(expected)));
}

#[test]
fn an_empty_batch_gives_no_outputs() {
    let runtime = Runtime::new();
    let batch = frost_closure!("app.batch", move |#[frost(ctx)] ctx| -> FrostResult {
        let futures = std::iter::empty::<std::future::Ready<Value>>();
        Ok(Value::from(runtime.run_all(ctx, futures, limit(3))?))
    });
    let ended = within_deadline(|| run(vm(BATCH, vec![("batch", batch)], None)));
    assert_eq!(ended, Ended::Finished(Value::from(Vec::<Value>::new())));
}

/// Counts the futures of a batch in progress at once, and the most ever.
#[derive(Default)]
struct InProgress {
    now: AtomicUsize,
    most: AtomicUsize,
}

impl InProgress {
    fn start(&self) {
        let now = self.now.fetch_add(1, Ordering::SeqCst) + 1;
        self.most.fetch_max(now, Ordering::SeqCst);
    }

    fn finish(&self) {
        self.now.fetch_sub(1, Ordering::SeqCst);
    }
}

#[test]
fn exactly_concurrency_futures_are_in_progress_at_once() {
    // Each future waits at a barrier for `concurrency` parties, so a batch
    // running fewer at once never gets past it, and the count of futures in
    // progress catches one running more.
    for concurrency in [1, 2, 3] {
        let runtime = Runtime::new();
        let in_progress = Arc::new(InProgress::default());
        let batch = {
            let in_progress = Arc::clone(&in_progress);
            frost_closure!("app.batch", move |#[frost(ctx)] ctx| -> FrostResult {
                let barrier = Arc::new(tokio::sync::Barrier::new(concurrency));
                let in_progress = Arc::clone(&in_progress);
                let futures = (0..concurrency * 3).map(move |_| {
                    let barrier = Arc::clone(&barrier);
                    let in_progress = Arc::clone(&in_progress);
                    async move {
                        in_progress.start();
                        barrier.wait().await;
                        in_progress.finish();
                    }
                });
                let outputs = runtime.run_all(ctx, futures, limit(concurrency))?;
                Ok(Value::Int(outputs.len().try_into().expect("small")))
            })
        };
        let ended = within_deadline(|| run(vm(BATCH, vec![("batch", batch)], None)));
        let expected = i64::try_from(concurrency * 3).expect("small");
        assert_eq!(
            ended,
            Ended::Finished(Value::Int(expected)),
            "every future finishes (concurrency {concurrency})"
        );
        assert_eq!(
            in_progress.most.load(Ordering::SeqCst),
            concurrency,
            "the most futures in progress at once (concurrency {concurrency})"
        );
    }
}

#[test]
fn cancelling_a_batch_drops_every_future_in_progress_and_starts_no_more() {
    let runtime = Runtime::new();
    let token = CancelToken::new();
    let (started_sender, started) = mpsc::channel();
    let (dropped_sender, dropped) = mpsc::channel();
    let batch = frost_closure!("app.batch", move |#[frost(ctx)] ctx| -> FrostResult {
        let (started_sender, dropped_sender) = (started_sender.clone(), dropped_sender.clone());
        let futures = (0..6).map(move |_| {
            let started = started_sender.clone();
            let dropped = DropSignal(dropped_sender.clone());
            async move {
                let _dropped = dropped;
                started.send(()).expect("the test is listening");
                std::future::pending::<()>().await;
            }
        });
        runtime.run_all(ctx, futures, limit(3))?;
        Ok(Value::Null)
    });
    let vm = vm(BATCH, vec![("batch", batch)], Some(&token));
    let canceller = thread::spawn(move || {
        for _ in 0..3 {
            expect_message(&started, "a future started");
        }
        token.cancel();
        started
    });
    let ended = within_deadline(move || run(vm));
    let started = canceller.join().expect("the canceller finishes");
    assert_eq!(ended, Ended::Aborted(AbortReason::Cancelled));
    // The iterator makes each future only as the batch takes it, so exactly
    // the three in progress were made, and each was dropped.
    for _ in 0..3 {
        expect_message(&dropped, "a future in progress was dropped");
    }
    assert_eq!(
        dropped.recv_timeout(DEADLINE),
        Err(mpsc::RecvTimeoutError::Disconnected),
        "no future beyond the first three may be made"
    );
    assert_eq!(
        started.try_recv(),
        Err(mpsc::TryRecvError::Disconnected),
        "no future beyond the first three may start"
    );
}

#[test]
fn a_panic_in_one_future_fails_the_whole_batch() {
    let runtime = Runtime::new();
    let batch = frost_closure!("app.batch", move |#[frost(ctx)] ctx| -> FrostResult {
        let futures = (0..4).map(|n| async move {
            assert_ne!(n, 2, "the third future panics");
            n
        });
        runtime.run_all(ctx, futures, limit(2))?;
        Ok(Value::Null)
    });
    let ended = within_deadline(|| run(vm(BATCH, vec![("batch", batch)], None)));
    let Ended::Raised(message) = ended else {
        panic!("the batch should fail, but ended with {ended:?}");
    };
    assert!(
        message.starts_with("Function app.batch failed: its async task panicked: "),
        "the batch's error: {message}"
    );
    assert!(
        message.contains("the third future panics"),
        "the error carries the panic's message: {message}"
    );
}
