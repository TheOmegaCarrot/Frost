//! Async Rust behind blocking native functions, for extensions built on
//! async clients.
//!
//! Frost functions are not async: a native function runs to completion on
//! the thread running its Vm. An extension built on an async client, such as
//! an HTTP or database client, takes a [`Runtime`] from its host and, inside
//! each native function, waits for its futures with [`Runtime::run`] or
//! [`Runtime::run_all`]. The futures run on the runtime's own threads, never
//! on the thread running the Vm, and a cancelled run stops waiting at once.
//!
//! A host creates one runtime and passes clones of it to every extension
//! that needs one:
//!
//! ```
//! use std::num::NonZeroUsize;
//!
//! let runtime = frostlang_tokio::Runtime::builder()
//!     .worker_threads(NonZeroUsize::new(2).unwrap())
//!     .build();
//! let for_some_extension = runtime.clone();
//! ```
//!
//! An extension's native function then runs a future to completion:
//!
//! ```
//! use frostlang::{FrostResult, Value, frost_closure};
//! use frostlang_tokio::Runtime;
//!
//! fn delayed_answer(runtime: Runtime) -> Value {
//!     frost_closure!("delayed_answer", move |#[frost(ctx)] ctx| -> FrostResult {
//!         let answer = runtime.run(ctx, async {
//!             tokio::time::sleep(std::time::Duration::from_millis(10)).await;
//!             42
//!         })?;
//!         Ok(Value::Int(answer))
//!     })
//! }
//! ```

use std::any::Any;
use std::fmt;
use std::future::Future;
use std::io;
use std::num::NonZeroUsize;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};

use frostlang::{FrostError, NativeCtx};
use futures_util::{FutureExt, StreamExt, stream};

/// A pool of threads on which extensions run async work for native functions.
///
/// Create one with [`new`](Self::new) or [`builder`](Self::builder), and give
/// clones of it to every extension that needs one: clones share one pool. Its
/// threads start when a native function first runs a future, so a host may
/// install extensions that scripts never use at no cost.
///
/// The pool shuts down when the last clone is dropped, or when any clone is
/// [shut down](Self::shutdown). Neither waits for running futures, and neither
/// panics, whatever thread or async context it happens on.
///
/// # Calling contexts
///
/// [`run`](Self::run) and [`run_all`](Self::run_all) block the thread running
/// the Vm until their futures finish. They neither panic nor deadlock wherever
/// that thread is: a plain thread, a Tokio `spawn_blocking` task, or even a
/// worker thread of the host's own Tokio runtime, which stays tied up while it
/// waits. A host running Frost from async code should run each Vm with
/// `spawn_blocking` or on threads of its own.
#[derive(Clone)]
pub struct Runtime(Arc<RuntimeState>);

/// What every clone of a [`Runtime`] shares.
struct RuntimeState {
    /// The pool's size, or `None` for Tokio's default: one thread per CPU core.
    worker_threads: Option<NonZeroUsize>,
    tokio: Mutex<TokioState>,
}

/// The Tokio runtime behind a [`Runtime`], through its life.
enum TokioState {
    /// No future has been run yet.
    NotStarted,
    Running(tokio::runtime::Runtime),
    /// Shut down, for good.
    ShutDown,
}

impl Runtime {
    /// A runtime with Tokio's default number of threads: one per CPU core.
    pub fn new() -> Self {
        Self::builder().build()
    }

    /// A builder for a runtime with settings of the host's choosing.
    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder::default()
    }

    /// Runs `future` to completion on this runtime, for the native function
    /// that `ctx` runs, and returns its output.
    ///
    /// Blocks until the future finishes.
    ///
    /// If the Vm's run is already aborting, fails at once, as
    /// [`NativeCtx::check_abort`] does, without starting the future. Once the
    /// wait has begun, only a cancellation ends it early: it fails with the
    /// run's cancellation error, the future is dropped at its next `.await`,
    /// without this waiting for it, and the run fails as cancelled, whatever
    /// the native function then returns.
    ///
    /// Otherwise fails, with an error naming the native function, if:
    ///
    /// - The runtime has shut down, or shuts down during the wait.
    /// - The runtime's threads cannot be started.
    /// - The future panics.
    ///
    /// Any other failure is the future's own to report in its output, as a
    /// `Result` or otherwise.
    pub fn run<F>(&self, ctx: &NativeCtx<'_>, future: F) -> Result<F::Output, FrostError>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        ctx.check_abort()?;
        let handle = self.handle(ctx.name())?;
        let (sender, receiver) = mpsc::channel();
        let outcome_sender = OutcomeSender(Some(sender.clone()));
        let task = handle.spawn(async move {
            let outcome = match AssertUnwindSafe(future).catch_unwind().await {
                Ok(output) => TaskOutcome::Finished(output),
                Err(payload) => TaskOutcome::Panicked(panic_message(&*payload)),
            };
            outcome_sender.send(outcome);
        });
        let abort = task.abort_handle();
        let _on_cancel = ctx.cancel_token().map(|token| {
            token.on_cancel(move || {
                // Before aborting, so that this is what the wait receives,
                // rather than the dropped task's outcome.
                let _ = sender.send(TaskOutcome::Cancelled);
                abort.abort();
            })
        });
        match receiver.recv() {
            Ok(TaskOutcome::Finished(output)) => Ok(output),
            Ok(TaskOutcome::Cancelled) => Err(ctx
                .check_abort()
                .expect_err("a run whose token is cancelled is aborting")),
            Ok(TaskOutcome::Panicked(message)) => Err(FrostError::from_string(format!(
                "Function {} failed: its async task panicked: {message}",
                ctx.name()
            ))),
            // The task's sender always sends before it goes, so the channel
            // never closes without a word; that is treated as a shutdown too,
            // defensively.
            Ok(TaskOutcome::Dropped) | Err(mpsc::RecvError) => Err(shut_down_error(ctx.name())),
        }
    }

    /// Runs every one of `futures` to completion on this runtime, for the
    /// native function that `ctx` runs, and returns their outputs in the
    /// order given.
    ///
    /// At most `concurrency` of the futures are in progress at once; the next
    /// starts as soon as any finishes. Each future is taken from `futures`
    /// only as a place frees up, on the runtime's threads, and none is taken
    /// once the batch has failed. They are run as one, so they fail as
    /// one as [`run`](Self::run) does: a cancellation drops every future still
    /// in progress, and a panic in any fails the whole batch.
    pub fn run_all<I>(
        &self,
        ctx: &NativeCtx<'_>,
        futures: I,
        concurrency: NonZeroUsize,
    ) -> Result<Vec<<I::Item as Future>::Output>, FrostError>
    where
        I: IntoIterator,
        I::IntoIter: Send + 'static,
        I::Item: Future + Send + 'static,
        <I::Item as Future>::Output: Send + 'static,
    {
        let futures = futures.into_iter().enumerate();
        let batch = async move {
            // Unordered, so that a slow future holds only its own place, not
            // the start of those after it; the order is restored after.
            let mut outputs: Vec<_> = stream::iter(futures)
                .map(|(index, future)| future.map(move |output| (index, output)))
                .buffer_unordered(concurrency.get())
                .collect()
                .await;
            outputs.sort_unstable_by_key(|&(index, _)| index);
            outputs
                .into_iter()
                .map(|(_, output)| output)
                .collect::<Vec<_>>()
        };
        self.run(ctx, batch)
    }

    /// Shuts this runtime down, for every clone, for good.
    ///
    /// Every future in progress is dropped at its next `.await`, without this
    /// waiting for it, and every native function waiting for one fails.
    /// Running a future afterwards fails at once.
    pub fn shutdown(&self) {
        let previous = std::mem::replace(&mut *self.0.lock(), TokioState::ShutDown);
        // Outside the lock: dropping tasks runs their destructors.
        previous.shut_down();
    }

    /// A handle to the Tokio runtime, started if this is its first use, for
    /// the native function `function`.
    fn handle(&self, function: &str) -> Result<tokio::runtime::Handle, FrostError> {
        let mut tokio = self.0.lock();
        match &*tokio {
            TokioState::Running(runtime) => Ok(runtime.handle().clone()),
            TokioState::ShutDown => Err(shut_down_error(function)),
            TokioState::NotStarted => {
                let runtime = self
                    .0
                    .build_tokio()
                    .map_err(|err| start_error(function, &err))?;
                let handle = runtime.handle().clone();
                *tokio = TokioState::Running(runtime);
                Ok(handle)
            }
        }
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match &*self.0.lock() {
            TokioState::NotStarted => "not started",
            TokioState::Running(_) => "running",
            TokioState::ShutDown => "shut down",
        };
        f.debug_struct("Runtime")
            .field("worker_threads", &self.0.worker_threads)
            .field("state", &state)
            .finish()
    }
}

impl RuntimeState {
    fn lock(&self) -> MutexGuard<'_, TokioState> {
        // Every critical section replaces the state whole, so a panic cannot
        // leave it half-changed.
        self.tokio.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A new Tokio runtime with this runtime's settings.
    fn build_tokio(&self) -> io::Result<tokio::runtime::Runtime> {
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        builder.enable_all().thread_name("frostlang-tokio");
        if let Some(count) = self.worker_threads {
            builder.worker_threads(count.get());
        }
        builder.build()
    }
}

impl Drop for RuntimeState {
    fn drop(&mut self) {
        let tokio = self.tokio.get_mut().unwrap_or_else(PoisonError::into_inner);
        std::mem::replace(tokio, TokioState::ShutDown).shut_down();
    }
}

impl TokioState {
    /// Shuts down the Tokio runtime, if running, without waiting for it.
    fn shut_down(self) {
        // Dropping the Tokio runtime itself would wait for its threads, which
        // panics in an async context.
        if let Self::Running(runtime) = self {
            runtime.shutdown_background();
        }
    }
}

/// A builder for a [`Runtime`].
#[derive(Debug, Default)]
#[must_use = "a builder does nothing until built"]
pub struct RuntimeBuilder {
    worker_threads: Option<NonZeroUsize>,
}

impl RuntimeBuilder {
    /// Run futures on `count` threads, rather than one per CPU core.
    ///
    /// Futures waiting on the network or a timer take no thread while they
    /// wait, so a few threads serve many requests at once.
    pub fn worker_threads(mut self, count: NonZeroUsize) -> Self {
        self.worker_threads = Some(count);
        self
    }

    /// The runtime. Its threads start when it first runs a future.
    pub fn build(self) -> Runtime {
        Runtime(Arc::new(RuntimeState {
            worker_threads: self.worker_threads,
            tokio: Mutex::new(TokioState::NotStarted),
        }))
    }
}

/// How a task run for a waiting native function ended, as the wait learns
/// it.
enum TaskOutcome<T> {
    Finished(T),
    /// The run was cancelled; the task is being aborted.
    Cancelled,
    /// The future panicked, with this message.
    Panicked(String),
    /// The task was dropped before finishing, as the runtime shut down.
    Dropped,
}

/// A task's means to send its outcome to the waiting native function, which
/// sends [`Dropped`](TaskOutcome::Dropped) if dropped before sending anything
/// else.
///
/// The task must say it was dropped itself: the wait cannot learn it from the
/// channel closing, since the cancel callback holds a sender of its own.
struct OutcomeSender<T>(Option<mpsc::Sender<TaskOutcome<T>>>);

impl<T> OutcomeSender<T> {
    fn send(mut self, outcome: TaskOutcome<T>) {
        if let Some(sender) = self.0.take() {
            // The native may have stopped waiting already, if cancelled.
            let _ = sender.send(outcome);
        }
    }
}

impl<T> Drop for OutcomeSender<T> {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(TaskOutcome::Dropped);
        }
    }
}

/// The error of a native function `function` whose runtime could not start.
fn start_error(function: &str, err: &io::Error) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} could not start its async runtime: {err}"
    ))
}

/// The error of a native function `function` whose runtime has shut down.
fn shut_down_error(function: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} could not finish: its async runtime has shut down"
    ))
}

/// The message a panic was raised with, when it is text.
fn panic_message(payload: &(dyn Any + Send)) -> String {
    match (
        payload.downcast_ref::<&str>(),
        payload.downcast_ref::<String>(),
    ) {
        (Some(message), _) => (*message).to_string(),
        (None, Some(message)) => message.clone(),
        (None, None) => String::from("a panic without a message"),
    }
}
