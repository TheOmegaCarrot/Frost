//! Harness for the tests of deeply nested values: building them, and working on them
//! on a thread with a small stack, which handling one native frame per level would
//! overflow many times over.
//!
//! A stack overflow aborts the whole test binary, so a failure in such a test shows as a
//! crash rather than a failed assertion.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use frostlang::{MapKey, Value, ValueMap};

/// Far less stack than one frame per level of [`DEPTH`] would take.
pub(crate) const SMALL_STACK: usize = 512 * 1024;

/// How deeply the deep cases nest.
pub(crate) const DEPTH: usize = 100_000;

/// How long [`on_a_small_stack_within_deadline`] lets work run before failing it as hung.
pub(crate) const DEADLINE: Duration = Duration::from_secs(30);

/// Runs `f` to completion on a thread with [`SMALL_STACK`].
pub(crate) fn on_a_small_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    thread::scope(|scope| {
        thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn_scoped(scope, f)
            .expect("the thread spawns")
            .join()
            .expect("the thread completes")
    })
}

/// Runs `f` on a thread with [`SMALL_STACK`], failing the test if it does not end within
/// [`DEADLINE`], as work that visits every place in a value sharing its contents would not.
pub(crate) fn on_a_small_stack_within_deadline<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (sender, receiver) = mpsc::channel();
    // Not scoped: a scope would wait for a hung thread instead of failing the test.
    thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(move || {
            // Past the deadline, no one is left to receive it.
            let _ = sender.send(f());
        })
        .expect("the thread spawns");
    match receiver.recv_timeout(DEADLINE) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => panic!("the work did not end within {DEADLINE:?}"),
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("the work panicked"),
    }
}

/// `inner` nested `depth` levels deep, each level built by `wrap` around the one below.
pub(crate) fn nested(depth: usize, inner: Value, wrap: impl Fn(Value) -> Value) -> Value {
    (0..depth).fold(inner, |below, _| wrap(below))
}

pub(crate) fn in_an_array(below: Value) -> Value {
    Value::from(vec![below])
}

pub(crate) fn in_a_map(below: Value) -> Value {
    Value::from(ValueMap::from([(MapKey::from("below"), below)]))
}
