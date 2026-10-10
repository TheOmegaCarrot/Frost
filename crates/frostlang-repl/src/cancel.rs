//! [`CancelHandle`]: the means to stop a [`Repl`](crate::Repl)'s input while it runs.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use frostlang::CancelToken;

/// Cancels whichever input a [`Repl`](crate::Repl) is evaluating, from any thread.
/// Obtain one from [`Repl::cancel_handle`](crate::Repl::cancel_handle) before evaluating.
///
/// A cancelled input fails with [`ReplError::Cancelled`](crate::ReplError::Cancelled),
/// leaving the REPL's bindings unchanged.
/// Cancelling while no input is being evaluated does nothing:
/// the next input runs as usual.
///
/// Clones cancel the same REPL's inputs.
#[derive(Debug, Clone, Default)]
pub struct CancelHandle(Arc<Mutex<Option<CancelToken>>>);

impl CancelHandle {
    /// Cancel the input being evaluated, if any.
    /// Returns whether one was.
    pub fn cancel(&self) -> bool {
        match &*self.lock() {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
    }

    /// A fresh token for an input about to be evaluated, which [`cancel`](Self::cancel)
    /// cancels until [`end`](Self::end).
    pub(crate) fn begin(&self) -> CancelToken {
        let token = CancelToken::new();
        *self.lock() = Some(token.clone());
        token
    }

    /// The input's evaluation is over: there is nothing to cancel.
    pub(crate) fn end(&self) {
        *self.lock() = None;
    }

    fn lock(&self) -> MutexGuard<'_, Option<CancelToken>> {
        // Every critical section is a single store or read, so a panic cannot
        // leave the slot half-written.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
