//! Tests for callbacks registered with [`CancelToken::on_cancel`].
//!
//! A callback runs exactly once if its token is cancelled while it is registered,
//! or at once if the token already was, and never once its registration is dropped.
//! A cancellation returns only once every callback has finished.
//! Races between cancelling and registering are repeated many times on separate
//! threads; every interleaving must uphold the same counts. Tests that a deadlock
//! would hang go through [`within_deadline`].
//! How a native uses a callback to wake when its run is cancelled is tested in
//! `vm_cancel.rs`.

use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex, mpsc};
use std::thread::{self, ThreadId};
use std::time::Duration;

use frostlang::{CancelCallback, CancelToken};

// ============================================================
// Harness
// ============================================================

/// How long a test that a deadlock would hang may take before it fails.
/// Generous: it only bounds a hang, and a passing test takes milliseconds.
const DEADLINE: Duration = Duration::from_secs(30);

/// Runs `body` on its own thread, failing the test if it does not finish within
/// [`DEADLINE`].
fn within_deadline(body: impl FnOnce() + Send + 'static) {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        body();
        sender.send(()).expect("the test is waiting");
    });
    match receiver.recv_timeout(DEADLINE) {
        Ok(()) => {}
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("the test did not finish within {DEADLINE:?}: it deadlocked")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("the test panicked"),
    }
}

/// Counts how many times the callbacks it makes have run.
#[derive(Clone, Default)]
struct Counter(Arc<AtomicUsize>);

impl Counter {
    /// A callback that counts one run.
    fn callback(&self) -> impl FnOnce() + Send + 'static {
        let count = Arc::clone(&self.0);
        move || {
            count.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn runs(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

/// How many times each race is repeated: enough to reach both orders of a
/// cancellation and a registration.
const RACES: usize = 1000;

// ============================================================
// Running on cancellation
// ============================================================

#[test]
fn a_callback_waits_until_the_token_is_cancelled() {
    let token = CancelToken::new();
    let counter = Counter::default();
    let _callback = token.on_cancel(counter.callback());
    assert_eq!(counter.runs(), 0, "registering does not run the callback");

    token.cancel();
    assert_eq!(counter.runs(), 1, "cancelling runs the callback");
}

#[test]
fn a_callback_runs_once_however_often_the_token_is_cancelled() {
    let token = CancelToken::new();
    let counter = Counter::default();
    let _callback = token.on_cancel(counter.callback());
    token.cancel();
    token.cancel();
    token.clone().cancel();
    assert_eq!(counter.runs(), 1);
}

#[test]
fn cancelling_a_clone_runs_callbacks_registered_on_another_clone() {
    let token = CancelToken::new();
    let counter = Counter::default();
    let _callback = token.clone().on_cancel(counter.callback());
    token.clone().cancel();
    assert_eq!(counter.runs(), 1);
}

#[test]
fn a_callback_runs_on_the_cancelling_thread() {
    let token = CancelToken::new();
    let ran_on: Arc<Mutex<Option<ThreadId>>> = Arc::default();
    let _callback = {
        let ran_on = Arc::clone(&ran_on);
        token.on_cancel(move || *ran_on.lock().unwrap() = Some(thread::current().id()))
    };
    let remote = token.clone();
    let canceller = thread::spawn(move || remote.cancel());
    let cancelling_thread = canceller.thread().id();
    canceller.join().unwrap();
    assert_eq!(*ran_on.lock().unwrap(), Some(cancelling_thread));
}

#[test]
fn callbacks_run_before_cancel_returns() {
    let token = CancelToken::new();
    let counter = Counter::default();
    let _first = token.on_cancel(counter.callback());
    let _second = token.on_cancel(counter.callback());
    token.cancel();
    assert_eq!(counter.runs(), 2, "both ran before `cancel` returned");
}

#[test]
fn a_cancellation_waits_for_callbacks_another_cancellation_is_running() {
    within_deadline(|| {
        let token = CancelToken::new();
        let (started_sender, started) = mpsc::channel();
        let (release_sender, release) = mpsc::channel::<()>();
        let finished = Arc::new(AtomicBool::new(false));
        let _callback = {
            let finished = Arc::clone(&finished);
            token.on_cancel(move || {
                started_sender.send(()).expect("the test is waiting");
                release.recv().expect("the test releases the callback");
                finished.store(true, Ordering::SeqCst);
            })
        };
        let first = {
            let token = token.clone();
            thread::spawn(move || token.cancel())
        };
        started
            .recv()
            .expect("the first cancellation runs the callback");

        let second = {
            let (token, finished) = (token.clone(), Arc::clone(&finished));
            thread::spawn(move || {
                token.cancel();
                finished.load(Ordering::SeqCst)
            })
        };
        // Time for the second cancellation to reach its wait. Too short a pause
        // could only let a wrong implementation pass, never fail a right one.
        thread::sleep(Duration::from_millis(50));
        release_sender.send(()).expect("the callback is waiting");
        assert!(
            second.join().unwrap(),
            "the second cancellation returned before the callback finished"
        );
        first.join().unwrap();
    });
}

#[test]
fn every_one_of_many_callbacks_runs() {
    let token = CancelToken::new();
    let counter = Counter::default();
    let _callbacks: Vec<CancelCallback> = (0..10_000)
        .map(|_| token.on_cancel(counter.callback()))
        .collect();
    token.cancel();
    assert_eq!(counter.runs(), 10_000);
}

// ============================================================
// Registering on a cancelled token
// ============================================================

#[test]
fn registering_on_a_cancelled_token_runs_the_callback_at_once() {
    let token = CancelToken::new();
    token.cancel();
    let counter = Counter::default();
    let _callback = token.on_cancel(counter.callback());
    assert_eq!(counter.runs(), 1, "ran before `on_cancel` returned");

    token.cancel();
    assert_eq!(counter.runs(), 1, "cancelling again does not run it again");
}

#[test]
fn a_callback_registered_on_a_cancelled_token_runs_on_the_registering_thread() {
    let token = CancelToken::new();
    token.cancel();
    let ran_on: Arc<Mutex<Option<ThreadId>>> = Arc::default();
    let _callback = {
        let ran_on = Arc::clone(&ran_on);
        token.on_cancel(move || *ran_on.lock().unwrap() = Some(thread::current().id()))
    };
    assert_eq!(*ran_on.lock().unwrap(), Some(thread::current().id()));
}

// ============================================================
// Unregistering
// ============================================================

#[test]
fn a_dropped_registration_never_runs() {
    let token = CancelToken::new();
    let counter = Counter::default();
    drop(token.on_cancel(counter.callback()));
    token.cancel();
    assert_eq!(counter.runs(), 0);
}

#[test]
fn dropping_one_registration_leaves_the_others() {
    let token = CancelToken::new();
    let counter = Counter::default();
    let _first = token.on_cancel(counter.callback());
    drop(token.on_cancel(counter.callback()));
    let _third = token.on_cancel(counter.callback());
    token.cancel();
    assert_eq!(counter.runs(), 2, "the first and third");
}

#[test]
fn dropping_a_registration_after_its_callback_ran_does_nothing() {
    let token = CancelToken::new();
    let counter = Counter::default();
    let callback = token.on_cancel(counter.callback());
    token.cancel();
    drop(callback);
    token.cancel();
    assert_eq!(counter.runs(), 1);
}

#[test]
fn dropping_a_registration_releases_its_callback() {
    // Repeated registrations over one long-lived token do not accumulate: each
    // dropped registration releases its callback, and so everything it captured.
    let token = CancelToken::new();
    let captured = Arc::new(());
    for _ in 0..10_000 {
        let held = Arc::clone(&captured);
        drop(token.on_cancel(move || drop(held)));
    }
    assert_eq!(
        Arc::strong_count(&captured),
        1,
        "every dropped callback released its capture"
    );
}

#[test]
fn a_callback_that_ran_is_released() {
    let token = CancelToken::new();
    let captured = Arc::new(());
    let _callback = {
        let held = Arc::clone(&captured);
        token.on_cancel(move || drop(held))
    };
    token.cancel();
    assert_eq!(Arc::strong_count(&captured), 1);
}

// ============================================================
// Callbacks acting on tokens
// ============================================================

#[test]
fn a_callback_can_register_another_callback() {
    // The token is cancelled by then, so the new callback runs at once.
    let token = CancelToken::new();
    let counter = Counter::default();
    let registered: Arc<Mutex<Option<CancelCallback>>> = Arc::default();
    let _callback = {
        let (token, counter, registered) =
            (token.clone(), counter.clone(), Arc::clone(&registered));
        token.clone().on_cancel(move || {
            *registered.lock().unwrap() = Some(token.on_cancel(counter.callback()));
        })
    };
    token.cancel();
    assert_eq!(counter.runs(), 1);
    assert!(registered.lock().unwrap().is_some());
}

#[test]
fn a_callback_can_unregister_a_callback_that_has_not_run() {
    // Each callback counts its run, then drops the other's registration.
    // Whichever runs first unregisters the other, which then never runs.
    let token = CancelToken::new();
    let counter = Counter::default();
    let slots: [Arc<Mutex<Option<CancelCallback>>>; 2] = Default::default();
    for (own, other) in [(0, 1), (1, 0)] {
        let (count, other) = (counter.callback(), Arc::clone(&slots[other]));
        let registration = token.on_cancel(move || {
            count();
            let unregistered = other.lock().unwrap().take();
            drop(unregistered);
        });
        *slots[own].lock().unwrap() = Some(registration);
    }
    token.cancel();
    assert_eq!(
        counter.runs(),
        1,
        "the callback that ran first unregistered the other"
    );
}

#[test]
fn a_callback_can_cancel_its_own_token() {
    // The inner cancellation must not wait for the callback that makes it.
    within_deadline(|| {
        let token = CancelToken::new();
        let counter = Counter::default();
        let _first = {
            let token = token.clone();
            token.clone().on_cancel(move || token.cancel())
        };
        let _second = token.on_cancel(counter.callback());
        token.cancel();
        assert_eq!(counter.runs(), 1, "the second still runs exactly once");
    });
}

#[test]
fn callbacks_cancelling_their_own_token_on_several_threads_do_not_deadlock() {
    // A cancellation from inside a callback does not wait for the callbacks other
    // threads are running, which may be waiting the same way. Each callback waits
    // until every canceller is inside one, so they all cancel from inside at once:
    // a canceller held in a callback cannot take another, so each takes exactly one.
    within_deadline(|| {
        const CANCELLERS: usize = 4;
        let token = CancelToken::new();
        let counter = Counter::default();
        let all_inside = Arc::new(Barrier::new(CANCELLERS));
        let _callbacks: Vec<CancelCallback> = (0..CANCELLERS)
            .map(|_| {
                let (token, count) = (token.clone(), counter.callback());
                let all_inside = Arc::clone(&all_inside);
                token.clone().on_cancel(move || {
                    all_inside.wait();
                    token.cancel();
                    count();
                })
            })
            .collect();
        let cancellers: Vec<_> = (0..CANCELLERS)
            .map(|_| {
                let token = token.clone();
                thread::spawn(move || token.cancel())
            })
            .collect();
        cancellers
            .into_iter()
            .for_each(|canceller| canceller.join().unwrap());
        assert_eq!(counter.runs(), CANCELLERS);
    });
}

#[test]
fn tokens_cancelling_each_other_from_two_threads_do_not_deadlock() {
    // Each token's callback cancels the other token, once both callbacks are
    // running, one on each thread.
    within_deadline(|| {
        let tokens = [CancelToken::new(), CancelToken::new()];
        let counter = Counter::default();
        let both_inside = Arc::new(Barrier::new(2));
        let _callbacks: Vec<CancelCallback> = [(0, 1), (1, 0)]
            .into_iter()
            .map(|(own, other)| {
                let (other, count) = (tokens[other].clone(), counter.callback());
                let both_inside = Arc::clone(&both_inside);
                tokens[own].on_cancel(move || {
                    both_inside.wait();
                    other.cancel();
                    count();
                })
            })
            .collect();
        let cancellers: Vec<_> = tokens
            .iter()
            .map(|token| {
                let token = token.clone();
                thread::spawn(move || token.cancel())
            })
            .collect();
        cancellers
            .into_iter()
            .for_each(|canceller| canceller.join().unwrap());
        assert_eq!(counter.runs(), 2);
    });
}

#[test]
fn a_callback_can_cancel_another_token() {
    let first = CancelToken::new();
    let second = CancelToken::new();
    let counter = Counter::default();
    let _chain = {
        let second = second.clone();
        first.on_cancel(move || second.cancel())
    };
    let _callback = second.on_cancel(counter.callback());
    first.cancel();
    assert!(second.is_cancelled());
    assert_eq!(counter.runs(), 1);
}

// ============================================================
// Races
// ============================================================

#[test]
fn a_registration_racing_a_cancellation_runs_exactly_once() {
    for race in 0..RACES {
        let token = CancelToken::new();
        let counter = Counter::default();
        let start = Arc::new(Barrier::new(2));
        let canceller = {
            let (token, start) = (token.clone(), Arc::clone(&start));
            thread::spawn(move || {
                start.wait();
                token.cancel();
            })
        };
        start.wait();
        let callback = token.on_cancel(counter.callback());
        canceller.join().unwrap();
        assert_eq!(counter.runs(), 1, "race {race}: the callback ran once");
        drop(callback);
    }
}

#[test]
fn an_unregistration_racing_a_cancellation_leaves_no_callback_behind() {
    // Either the drop unregisters the callback or the cancellation runs it; both
    // release what it captured.
    for race in 0..RACES {
        let token = CancelToken::new();
        let counter = Counter::default();
        let captured = Arc::new(());
        let callback = {
            let (held, count) = (Arc::clone(&captured), counter.callback());
            token.on_cancel(move || {
                drop(held);
                count();
            })
        };
        let start = Arc::new(Barrier::new(2));
        let canceller = {
            let (token, start) = (token.clone(), Arc::clone(&start));
            thread::spawn(move || {
                start.wait();
                token.cancel();
            })
        };
        start.wait();
        drop(callback);
        canceller.join().unwrap();
        assert!(
            counter.runs() <= 1,
            "race {race}: ran {} times",
            counter.runs()
        );
        assert_eq!(
            Arc::strong_count(&captured),
            1,
            "race {race}: the callback was neither unregistered nor run"
        );
    }
}

#[test]
fn registrations_from_many_threads_racing_a_cancellation_each_run_once() {
    const THREADS: usize = 8;
    const PER_THREAD: usize = 100;
    for race in 0..RACES / 10 {
        let token = CancelToken::new();
        let counter = Counter::default();
        let start = Arc::new(Barrier::new(THREADS + 1));
        let registrars: Vec<_> = (0..THREADS)
            .map(|_| {
                let (token, counter, start) = (token.clone(), counter.clone(), Arc::clone(&start));
                thread::spawn(move || {
                    start.wait();
                    (0..PER_THREAD)
                        .map(|_| token.on_cancel(counter.callback()))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        start.wait();
        token.cancel();
        let callbacks: Vec<Vec<CancelCallback>> = registrars
            .into_iter()
            .map(|registrar| registrar.join().unwrap())
            .collect();
        assert_eq!(
            counter.runs(),
            THREADS * PER_THREAD,
            "race {race}: every callback ran exactly once"
        );
        drop(callbacks);
    }
}

#[test]
fn concurrent_cancellations_run_each_callback_once() {
    within_deadline(|| {
        const CANCELLERS: usize = 4;
        for race in 0..RACES / 10 {
            let token = CancelToken::new();
            let counter = Counter::default();
            let _callbacks: Vec<CancelCallback> = (0..100)
                .map(|_| token.on_cancel(counter.callback()))
                .collect();
            let start = Arc::new(Barrier::new(CANCELLERS));
            let cancellers: Vec<_> = (0..CANCELLERS)
                .map(|_| {
                    let (token, start) = (token.clone(), Arc::clone(&start));
                    let counter = counter.clone();
                    thread::spawn(move || {
                        start.wait();
                        token.cancel();
                        counter.runs()
                    })
                })
                .collect();
            for canceller in cancellers {
                assert_eq!(
                    canceller.join().unwrap(),
                    100,
                    "race {race}: a cancellation returned before every callback ran"
                );
            }
        }
    });
}

// ============================================================
// Panicking callbacks
// ============================================================

#[test]
fn a_panicking_callback_leaves_the_rest_for_the_next_cancellation() {
    within_deadline(|| {
        let token = CancelToken::new();
        let counter = Counter::default();
        let _before = token.on_cancel(counter.callback());
        let _panicking = token.on_cancel(|| panic!("a callback panics"));
        let _after = token.on_cancel(counter.callback());

        let outcome = panic::catch_unwind(AssertUnwindSafe(|| token.cancel()));
        assert!(outcome.is_err(), "the panic propagates out of `cancel`");
        assert!(token.is_cancelled(), "the token stays cancelled");

        token.cancel();
        assert_eq!(
            counter.runs(),
            2,
            "the next cancellation runs what the panic left registered"
        );
    });
}

#[test]
fn a_panicking_callback_on_a_cancelled_token_leaves_the_token_working() {
    let token = CancelToken::new();
    token.cancel();
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        token.on_cancel(|| panic!("a callback panics"))
    }));
    assert!(outcome.is_err(), "the panic propagates out of `on_cancel`");

    let counter = Counter::default();
    let _callback = token.on_cancel(counter.callback());
    assert_eq!(counter.runs(), 1, "later registrations still run at once");
}
