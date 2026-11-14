//! [`CancelToken`]: a host's means to stop a running [`Vm`](super::Vm) from outside.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

/// A request, from any thread, to stop running Frost code.
///
/// Attach a token to a [`Vm`](super::Vm) with
/// [`with_cancel_token`](super::Vm::with_cancel_token).
/// Once the token is [cancelled](Self::cancel), that Vm's run fails with
/// [`AbortReason::Cancelled`](super::AbortReason::Cancelled),
/// as does the run of every module it imports.
///
/// Clones share one state: cancelling any clone cancels them all,
/// so one token attached to many Vms cancels them as a group.
/// Cancellation is permanent; for a run that should be cancellable on its own,
/// give it a fresh token.
///
/// A Vm notices cancellation when its script makes a function call,
/// when a native function returns, whatever it returns,
/// or when a native checks [`NativeCtx::abort_reason`](super::NativeCtx::abort_reason).
/// A native that blocks or loops without checking delays it until the native returns;
/// see [`NativeCtx::cancel_token`](super::NativeCtx::cancel_token) for work a native
/// cannot check from, and [`on_cancel`](Self::on_cancel) for work it waits on.
///
/// ```
/// use frostlang::CancelToken;
///
/// let token = CancelToken::new();
/// let remote = token.clone();
/// std::thread::spawn(move || remote.cancel()).join().unwrap();
/// assert!(token.is_cancelled());
/// ```
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<TokenState>);

/// The state every clone of a token shares.
#[derive(Debug, Default)]
struct TokenState {
    /// Set by the first cancellation. Read without the lock, so checking stays cheap.
    cancelled: AtomicBool,
    /// Callbacks waiting for cancellation. The flag is set while this lock is held,
    /// so a registration and a cancellation always agree on which came first.
    callbacks: Mutex<Callbacks>,
    /// Notified when the last callback a cancellation is running finishes.
    finished: Condvar,
}

impl TokenState {
    fn callbacks(&self) -> MutexGuard<'_, Callbacks> {
        // No callback runs or drops under the lock, so a poisoned lock still holds
        // consistent state.
        self.callbacks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// The callbacks registered with [`CancelToken::on_cancel`] that have not yet finished.
#[derive(Default)]
struct Callbacks {
    /// The id the next registration gets; ids are never reused.
    next_id: u64,
    waiting: BTreeMap<u64, Box<dyn FnOnce() + Send>>,
    /// How many callbacks cancellations have taken and not yet finished running.
    running: usize,
}

impl fmt::Debug for Callbacks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Callbacks")
            .field("waiting", &self.waiting.len())
            .field("running", &self.running)
            .finish_non_exhaustive()
    }
}

thread_local! {
    /// How many callbacks, of any token, cancellations on this thread are running.
    static CALLBACK_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// A callback a cancellation has taken, counted as running until this drops,
/// even when the callback panics.
struct RunningCallback<'a> {
    token: &'a TokenState,
    /// Whether this thread's depth counts the callback: not once the thread's
    /// locals are torn down, as when a cancellation runs in a thread-local destructor.
    counted_here: bool,
}

impl<'a> RunningCallback<'a> {
    /// Counts a callback of `token` as running,
    /// both in `token`'s count and in this thread's depth.
    /// `callbacks` is `token`'s, already locked by the caller.
    fn start(token: &'a TokenState, callbacks: &mut Callbacks) -> Self {
        callbacks.running += 1;
        let counted_here = CALLBACK_DEPTH
            .try_with(|depth| depth.set(depth.get() + 1))
            .is_ok();
        Self {
            token,
            counted_here,
        }
    }

    /// Whether this thread is running a callback, of any token.
    fn on_this_thread() -> bool {
        CALLBACK_DEPTH
            .try_with(|depth| depth.get() > 0)
            .unwrap_or(false)
    }
}

impl Drop for RunningCallback<'_> {
    fn drop(&mut self) {
        if self.counted_here {
            let _ = CALLBACK_DEPTH.try_with(|depth| depth.set(depth.get() - 1));
        }
        let mut callbacks = self.token.callbacks();
        callbacks.running -= 1;
        if callbacks.running == 0 {
            self.token.finished.notify_all();
        }
    }
}

impl CancelToken {
    /// A token that is not cancelled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancel this token and every clone of it.
    ///
    /// Runs every callback registered with [`on_cancel`](Self::on_cancel),
    /// and returns once each has finished,
    /// including those a cancellation on another thread is running.
    /// Callbacks run on the threads that cancel, in no particular order:
    /// when several threads cancel at once, they share the callbacks out.
    /// A cancellation from inside a callback that a cancellation is running,
    /// of this token or any other,
    /// returns without waiting for callbacks other threads are running,
    /// so that tokens whose callbacks cancel one another cannot deadlock.
    ///
    /// # Panics
    ///
    /// If a callback panics. The token stays cancelled,
    /// and callbacks not yet run stay registered until a cancellation runs them.
    pub fn cancel(&self) {
        let state = &*self.0;
        let mut callbacks = state.callbacks();
        // Relaxed suffices: the lock orders the flag against registrations, and
        // a Vm that reads it a moment late only aborts a moment later.
        state.cancelled.store(true, Ordering::Relaxed);
        // One at a time, each taken under the lock and run outside it, so that a
        // callback can unregister one that has not run yet. None is added once the
        // flag is set: `on_cancel` runs it at once instead.
        while let Some((_, callback)) = callbacks.waiting.pop_first() {
            let running = RunningCallback::start(state, &mut callbacks);
            drop(callbacks);
            callback();
            // Before locking again: finishing locks the callbacks itself.
            drop(running);
            callbacks = state.callbacks();
        }
        if !RunningCallback::on_this_thread() {
            let _callbacks = state
                .finished
                .wait_while(callbacks, |callbacks| callbacks.running > 0)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Whether this token, or any clone of it, has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Relaxed)
    }

    /// Runs `callback` once this token is cancelled:
    /// on a cancelling thread, during [`cancel`](Self::cancel).
    /// If the token is already cancelled, runs it at once, on this thread, before returning.
    /// Either way, no cancellation is missed.
    ///
    /// Use it to wake or stop work a native waits on,
    /// such as a wait for another thread or a call into a library,
    /// when the run is cancelled.
    /// Dropping the returned [`CancelCallback`] unregisters the callback.
    ///
    /// A callback runs at most once, and should return quickly:
    /// [`cancel`](Self::cancel) waits for every callback before it returns.
    /// A callback may register or unregister callbacks and cancel tokens, this one included,
    /// but must not wait for a cancellation on another thread,
    /// which may be waiting for the callback in turn.
    ///
    /// ```
    /// use std::sync::mpsc;
    /// use frostlang::CancelToken;
    ///
    /// let token = CancelToken::new();
    /// let (wake, woken) = mpsc::channel();
    /// let _callback = token.on_cancel(move || wake.send("cancelled").unwrap());
    ///
    /// let remote = token.clone();
    /// std::thread::spawn(move || remote.cancel());
    /// assert_eq!(woken.recv(), Ok("cancelled"));
    /// ```
    pub fn on_cancel(&self, callback: impl FnOnce() + Send + 'static) -> CancelCallback {
        let mut callbacks = self.0.callbacks();
        let id = callbacks.next_id;
        callbacks.next_id += 1;
        if self.is_cancelled() {
            drop(callbacks);
            callback();
        } else {
            callbacks.waiting.insert(id, Box::new(callback));
        }
        CancelCallback {
            token: Arc::clone(&self.0),
            id,
        }
    }
}

/// A callback registered with [`CancelToken::on_cancel`].
///
/// Dropping it unregisters the callback, which then never runs.
/// If the callback is already running on another thread, dropping does not wait for it
/// to finish.
#[derive(Debug)]
#[must_use = "dropping a CancelCallback unregisters its callback"]
pub struct CancelCallback {
    token: Arc<TokenState>,
    id: u64,
}

impl Drop for CancelCallback {
    fn drop(&mut self) {
        // The guard is a temporary of this statement, so the callback, whose
        // captures may run arbitrary code as they drop, drops after the lock is released.
        let unregistered = self.token.callbacks().waiting.remove(&self.id);
        drop(unregistered);
    }
}
