//! Harness for the tests: scripts calling natives that wait on a [`Runtime`],
//! run with a deadline, since a wait that never ends would hang the test.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{
    AbortReason, CancelToken, HostComponent, ImporterBuilder, RunErrorKind, Value, Vm,
};

/// How long a wait that should end may take before the test fails.
/// Generous: it only bounds a hang, and a passing wait takes milliseconds.
pub(crate) const DEADLINE: Duration = Duration::from_secs(30);

/// How a run ended: its value, or how it failed.
#[derive(Debug, PartialEq)]
pub(crate) enum Ended {
    Finished(Value),
    Raised(String),
    Aborted(AbortReason),
}

/// A Vm running `source`, with the host component `app` holding `natives`,
/// and cancellable by `token` if given.
pub(crate) fn vm(source: &str, natives: Vec<(&str, Value)>, token: Option<&CancelToken>) -> Vm {
    let app = natives
        .into_iter()
        .fold(HostComponent::builder("app"), |app, (key, value)| {
            app.value(key, value)
        })
        .build();
    let importer = ImporterBuilder::new()
        .with_component(app)
        .expect("a lone component is accepted")
        .build();
    let closure = compile_program("main.frst", source, CompilerOptions::new())
        .unwrap_or_else(|errors| panic!("the script compiles:\n{}", errors.render()))
        .code
        .into_closure()
        .expect("the script captures nothing");
    let vm = Vm::factory().with_importer(importer).build(closure);
    match token {
        Some(token) => vm.with_cancel_token(token.clone()),
        None => vm,
    }
}

/// Runs `vm` on the calling thread, and says how the run ended.
pub(crate) fn run(vm: Vm) -> Ended {
    match vm.run() {
        Ok(result) => Ended::Finished(result.tail().clone()),
        Err(failure) => match failure.kind() {
            RunErrorKind::Aborted(reason) => Ended::Aborted(reason),
            RunErrorKind::Raised => Ended::Raised(failure.into_error().message().into_owned()),
        },
    }
}

/// Runs `body` on a thread of its own, failing the test if it does not finish
/// within [`DEADLINE`].
pub(crate) fn within_deadline<T: Send + 'static>(body: impl FnOnce() -> T + Send + 'static) -> T {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || sender.send(body()));
    match receiver.recv_timeout(DEADLINE) {
        Ok(outcome) => outcome,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("the work did not finish within {DEADLINE:?}: a wait never ended")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("the work panicked"),
    }
}

/// Waits for `receiver`'s next message, failing the test if none comes within
/// [`DEADLINE`]. `what` says what the message means, for the failure.
#[track_caller]
pub(crate) fn expect_message<T>(receiver: &mpsc::Receiver<T>, what: &str) -> T {
    receiver
        .recv_timeout(DEADLINE)
        .unwrap_or_else(|err| panic!("expected word that {what}, but got: {err}"))
}

/// Sends `()` on its channel when dropped: dropped with a future, it reports
/// that the future was dropped.
pub(crate) struct DropSignal(pub(crate) mpsc::Sender<()>);

impl Drop for DropSignal {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
