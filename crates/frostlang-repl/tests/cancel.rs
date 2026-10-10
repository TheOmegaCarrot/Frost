//! Cancelling a `Repl`'s input with a `CancelHandle`: the input fails as
//! cancelled, changes nothing, and the session carries on.
//!
//! A cancellation that goes unnoticed leaves an input spinning forever, so
//! runs that rely on one go through [`within_deadline`].

use std::io;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use frostlang::{Arity, Value};
use frostlang_repl::{CancelHandle, Frontend, Repl, ReplError, ScriptedFrontend};

/// How long work that should be cancelled may take before the test fails.
/// Generous: it only bounds a hang, and a passing run takes milliseconds.
const DEADLINE: Duration = Duration::from_secs(30);

/// Run `body` on its own thread, failing the test if it does not finish within
/// [`DEADLINE`]: a cancellation that is never noticed would otherwise hang it.
fn within_deadline<T: Send + 'static>(body: impl FnOnce() -> T + Send + 'static) -> T {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || sender.send(body()));
    match receiver.recv_timeout(DEADLINE) {
        Ok(outcome) => outcome,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("the input did not end within {DEADLINE:?}: the cancellation went unnoticed")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("the input panicked"),
    }
}

/// Check that `outcome` is a cancelled input.
#[track_caller]
fn assert_cancelled(outcome: &Result<Value, ReplError>) {
    match outcome {
        Err(ReplError::Cancelled(error)) => assert_eq!(
            error.message(),
            "Execution was cancelled",
            "a cancelled input's error message"
        ),
        other => panic!("the input should be cancelled, but gave {other:?}"),
    }
}

/// The value of `source` evaluated on `repl`, which must succeed.
#[track_caller]
fn value(repl: &mut Repl, source: &str) -> Value {
    repl.evaluate(source)
        .unwrap_or_else(|error| panic!("{source:?} failed: {error:?}"))
}

/// An input that calls `signal()` and then recurses forever, after binding
/// `lost`: it ends only by being cancelled, and must leave nothing bound.
const RUNAWAY: &str = r"
def lost = 2
defn spin(n) -> spin(n + 1)
signal()
spin(0)
";

#[test]
fn cancelling_from_another_thread_stops_the_running_input() {
    let (started_sender, started) = mpsc::channel();
    let signal = Value::native("signal", Arity::Exact(0), move |_, _: &mut [Value]| {
        started_sender.send(()).expect("the test is waiting");
        Ok(Value::Null)
    });
    let mut repl = Repl::new().with_binding("signal", signal).unwrap();
    let handle = repl.cancel_handle();
    assert_eq!(value(&mut repl, "def kept = 1\n10"), Value::Int(10));

    let canceller = thread::spawn(move || {
        started.recv().expect("the input reports that it started");
        handle.cancel()
    });
    let (mut repl, outcome) = within_deadline(move || {
        let outcome = repl.evaluate(RUNAWAY);
        (repl, outcome)
    });
    assert!(
        canceller.join().unwrap(),
        "cancel should report that an input was running"
    );
    assert_cancelled(&outcome);

    let names: Vec<&str> = repl.bindings().map(|(name, _)| name).collect();
    assert_eq!(
        names,
        ["kept", "signal"],
        "the cancelled input must bind nothing"
    );
    assert_eq!(
        value(&mut repl, "results"),
        Value::from(vec![Value::Int(10)]),
        "the cancelled input must record no result"
    );
    assert_eq!(
        value(&mut repl, "kept + 1"),
        Value::Int(2),
        "the session carries on after a cancelled input"
    );
}

#[test]
fn cancelling_with_no_input_running_does_nothing() {
    let mut repl = Repl::new();
    let handle = repl.cancel_handle();
    assert!(
        !handle.cancel(),
        "cancel before any input should report that none was running"
    );
    assert_eq!(value(&mut repl, "1 + 1"), Value::Int(2));
    assert!(
        !handle.cancel(),
        "cancel between inputs should report that none was running"
    );
    assert_eq!(
        value(&mut repl, "defn add(a, b) -> a + b\nadd(2, 2)"),
        Value::Int(4),
        "a cancel between inputs must not reach the next one"
    );
}

/// A REPL whose `signal()` cancels the input calling it, through a handle
/// taken before the REPL was configured further.
fn self_cancelling_repl() -> Repl {
    let repl = Repl::new();
    let handle = repl.cancel_handle();
    let signal = Value::native("signal", Arity::Exact(0), move |_, _: &mut [Value]| {
        assert!(handle.cancel(), "cancel should report the input running");
        Ok(Value::Null)
    });
    repl.with_binding("signal", signal).unwrap()
}

#[test]
fn a_handle_taken_before_configuring_the_repl_still_cancels_its_inputs() {
    let mut repl = self_cancelling_repl();
    let outcome = within_deadline(move || repl.evaluate(RUNAWAY));
    assert_cancelled(&outcome);
}

#[test]
fn a_cancelled_input_displays_as_an_error() {
    let mut repl = self_cancelling_repl();
    let outcome = within_deadline(move || repl.evaluate(RUNAWAY));
    let error = outcome.expect_err("the input should be cancelled");
    let shown = error.to_string();
    assert!(
        shown.starts_with("Error: Execution was cancelled"),
        "a cancelled input should display as its error, but shows: {shown}"
    );
}

/// A [`ScriptedFrontend`] that, like a terminal watching for Ctrl-C, keeps the
/// handle its session gives it, for `handle` to cancel through.
struct InterruptibleFrontend {
    scripted: ScriptedFrontend,
    handle: Arc<Mutex<Option<CancelHandle>>>,
}

impl Frontend for InterruptibleFrontend {
    fn read_segment(&mut self) -> io::Result<Option<String>> {
        self.scripted.read_segment()
    }

    fn render(&mut self, outcome: Result<&Value, &ReplError>) -> io::Result<()> {
        self.scripted.render(outcome)
    }

    fn render_text(&mut self, text: &str) -> io::Result<()> {
        self.scripted.render_text(text)
    }

    fn set_cancel_handle(&mut self, handle: CancelHandle) {
        *self.handle.lock().unwrap() = Some(handle);
    }
}

#[test]
fn a_frontend_cancels_through_the_handle_its_session_gives_it() {
    // `signal()` stands in for the user interrupting while the input runs:
    // it cancels through whatever handle the frontend was given.
    let handle = Arc::new(Mutex::new(None::<CancelHandle>));
    let signal = {
        let handle = Arc::clone(&handle);
        Value::native("signal", Arity::Exact(0), move |_, _: &mut [Value]| {
            let guard = handle.lock().unwrap();
            let handle = guard
                .as_ref()
                .expect("the session gives the frontend its handle before any input");
            assert!(handle.cancel(), "cancel should report the input running");
            Ok(Value::Null)
        })
    };
    let scripted = ScriptedFrontend::new(["def kept = 1", RUNAWAY, "[kept, is_null(lost)]"]);
    let transcript = scripted.transcript();
    let mut frontend = InterruptibleFrontend { scripted, handle };
    let mut repl = Repl::new().with_binding("signal", signal).unwrap();
    within_deadline(move || repl.run(&mut frontend).expect("the session runs"));

    let outcomes = transcript.outcomes();
    let [first, cancelled, last] = outcomes.as_slice() else {
        panic!("every segment should be rendered, but got {outcomes:?}");
    };
    assert_eq!(first.as_ref().ok(), Some(&Value::Null), "`def kept = 1`");
    assert_cancelled(cancelled);
    assert!(
        matches!(last, Err(ReplError::Compile(_))),
        "the cancelled input must not have bound `lost`, but gave {last:?}"
    );
}
