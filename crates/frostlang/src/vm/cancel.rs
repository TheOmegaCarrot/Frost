//! [`CancelToken`]: a host's means to stop a running [`Vm`](super::Vm) from outside.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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
/// A Vm notices cancellation when its script makes a function call, not while a
/// native function runs; a native that blocks or loops delays it until the native returns.
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
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A token that is not cancelled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancel this token and every clone of it.
    pub fn cancel(&self) {
        // Relaxed suffices: the flag publishes no other data, and a Vm that reads
        // it a moment late only aborts a moment later.
        self.0.store(true, Ordering::Relaxed);
    }

    /// Whether this token, or any clone of it, has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}
