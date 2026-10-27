//! An eager script ingester: a thread that keeps every Frost module under a
//! directory loaded, so that a host's hot path never compiles.
//!
//! An [`Ingester`] watches its root recursively. When files change, it
//! rescans, reloads each module whose file changed along with every module
//! that imports one, and publishes a new [`Registry`]. The host reads the
//! latest registry whenever it likes, at the cost of a lock and a reference
//! count, and finds the modules there already run.
//!
//! A module under the root is named as `import` names it: `a/b/c.frst` is
//! `a.b.c`. A module's value is a Map of its exports, as with the filesystem
//! resolver.
//!
//! The ingester's [`Importer`] serves modules and host Vms alike: share it, by
//! [`Ingester::importer`]. Modules loading on the ingester's thread import the
//! rebuild in progress, which records who imports what. Every other import,
//! such as one a host Vm makes, takes the module from the latest registry
//! without compiling.

mod build;
mod registry;
mod scan;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::{io, thread};

use frostlang::compile::CompilerOptions;
use frostlang::{
    CancelToken, ImportResolver, Importer, ImporterBuilder, Vm, VmRuntimeConfiguration,
};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

pub use registry::{LoadError, Module, Registry};

use build::{Builder, IngestResolver};

/// Configures and starts an [`Ingester`].
#[derive(Debug)]
pub struct IngesterBuilder {
    root: PathBuf,
    importer: ImporterBuilder,
    configuration: VmRuntimeConfiguration,
    options: CompilerOptions,
    watching: bool,
    debounce: Duration,
}

/// Why an [`Ingester`] could not start.
#[derive(Debug, thiserror::Error)]
pub enum StartError {
    /// The root is missing, inaccessible, or not a directory.
    #[error("cannot use '{}' as the root: {error}", .root.display())]
    Root {
        /// The root as it was given.
        root: PathBuf,
        /// What went wrong.
        #[source]
        error: io::Error,
    },
    /// The root cannot be watched.
    #[error("cannot watch '{}': {error}", .root.display())]
    Watch {
        /// The root, canonical.
        root: PathBuf,
        /// What went wrong.
        #[source]
        error: notify::Error,
    },
}

impl IngesterBuilder {
    /// What may be imported besides the modules under the root; empty by
    /// default. The ingester adds its own resolver to it and builds the
    /// [`Importer`] that [`Ingester::importer`] shares.
    pub fn with_importer(mut self, importer: ImporterBuilder) -> Self {
        self.importer = importer;
        self
    }

    /// The limits each module's top level runs under.
    pub fn with_configuration(mut self, configuration: VmRuntimeConfiguration) -> Self {
        self.configuration = configuration;
        self
    }

    /// How modules compile; [`CompilerOptions::new`] by default.
    pub fn with_compiler_options(mut self, options: CompilerOptions) -> Self {
        self.options = options;
        self
    }

    /// Whether to rescan when files change, as by default. Without it,
    /// the ingester rescans only on [`Ingester::refresh`].
    pub fn with_watching(mut self, watching: bool) -> Self {
        self.watching = watching;
        self
    }

    /// How long changes must pause before a rescan: an editor saving a file
    /// often makes several changes in a burst. 30 milliseconds by default.
    pub fn with_debounce(mut self, debounce: Duration) -> Self {
        self.debounce = debounce;
        self
    }

    /// Start the ingester, returning once its first registry is published.
    ///
    /// A module whose top level never finishes keeps this from returning, as
    /// it keeps every later rescan from publishing; a fuel limit in the
    /// [configuration](Self::with_configuration) prevents it.
    pub fn start(self) -> Result<Ingester, StartError> {
        let root = std::fs::canonicalize(&self.root)
            .and_then(|root| {
                if root.is_dir() {
                    Ok(root)
                } else {
                    Err(io::ErrorKind::NotADirectory.into())
                }
            })
            .map_err(|error| StartError::Root {
                root: self.root.clone(),
                error,
            })?;
        let (commands, received) = mpsc::channel();
        let watcher = if self.watching {
            Some(watch(&root, commands.clone())?)
        } else {
            None
        };
        let shared = Arc::new(Shared::default());
        let resolver = Arc::new(IngestResolver::new(self.options, Arc::clone(&shared)));
        let importer = self
            .importer
            .append_resolver(Arc::clone(&resolver) as Arc<dyn ImportResolver>)
            .build();
        let cancel = CancelToken::new();
        let builder = Builder {
            root: root.clone(),
            factory: Vm::factory()
                .configuration(self.configuration)
                .with_importer(Arc::clone(&importer)),
            cancel: cancel.clone(),
            resolver,
        };
        let thread = thread::Builder::new()
            .name("frost-ingester".to_owned())
            .spawn({
                let shared = Arc::clone(&shared);
                let debounce = self.debounce;
                move || {
                    builder.resolver.claim_thread();
                    work(&builder, &shared, &received, debounce);
                }
            })
            .expect("the platform can spawn a thread");
        let ingester = Ingester {
            root,
            shared,
            importer,
            commands,
            cancel,
            thread: Some(thread),
            _watcher: watcher,
        };
        ingester.refresh();
        Ok(ingester)
    }
}

/// A recursive watch of `root` that asks for a rescan on every change.
fn watch(root: &Path, commands: Sender<Command>) -> Result<RecommendedWatcher, StartError> {
    let error = |error| StartError::Watch {
        root: root.to_path_buf(),
        error,
    };
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        // Reading a file is an access, and every rescan reads them all.
        // An error may mean events were missed, so it rescans too.
        if !event.is_ok_and(|event| matches!(event.kind, EventKind::Access(_))) {
            // The ingester has stopped if this fails, and wants no more.
            let _ = commands.send(Command::Rescan);
        }
    })
    .map_err(error)?;
    watcher
        .watch(root, RecursiveMode::Recursive)
        .map_err(error)?;
    Ok(watcher)
}

/// Keeps the Frost modules under a directory loaded, from a thread of its own.
/// See the [module documentation](self).
///
/// Dropping it stops the thread, cancelling any module running there.
#[derive(Debug)]
pub struct Ingester {
    root: PathBuf,
    shared: Arc<Shared>,
    importer: Arc<Importer>,
    commands: Sender<Command>,
    cancel: CancelToken,
    thread: Option<JoinHandle<()>>,
    _watcher: Option<RecommendedWatcher>,
}

impl Ingester {
    /// An ingester of the modules under `root`, with nothing else to import,
    /// no runtime limits, default compiler options, and watching on.
    pub fn builder(root: impl Into<PathBuf>) -> IngesterBuilder {
        IngesterBuilder {
            root: root.into(),
            importer: ImporterBuilder::new(),
            configuration: VmRuntimeConfiguration::default(),
            options: CompilerOptions::new(),
            watching: true,
            debounce: Duration::from_millis(30),
        }
    }

    /// The root, canonical.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The latest registry.
    pub fn registry(&self) -> Arc<Registry> {
        Arc::clone(&self.shared.state().registry)
    }

    /// Rescan now, returning the registry once the rescan is published: it
    /// reflects every file as it was when this was called.
    pub fn refresh(&self) -> Arc<Registry> {
        let ticket = {
            let mut state = self.shared.state();
            state.tickets_issued += 1;
            state.tickets_issued
        };
        // The thread outlives `self`, so it is there to receive this.
        let _ = self.commands.send(Command::Refresh(ticket));
        let state = self
            .shared
            .published
            .wait_while(self.shared.state(), |state| state.tickets_served < ticket)
            .unwrap_or_else(PoisonError::into_inner);
        Arc::clone(&state.registry)
    }

    /// The latest registry once its generation passes `generation`, or the
    /// latest registry anyway after `timeout`.
    pub fn wait_past(&self, generation: u64, timeout: Duration) -> Arc<Registry> {
        let deadline = Instant::now() + timeout;
        let mut state = self.shared.state();
        while state.registry.generation <= generation {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            state = self
                .shared
                .published
                .wait_timeout(state, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        Arc::clone(&state.registry)
    }

    /// The importer the modules use, for the host's own Vms too. Off the
    /// ingester's thread, it serves the modules under the root from the latest
    /// registry, never compiling: a module whose latest load failed is an
    /// import error.
    pub fn importer(&self) -> Arc<Importer> {
        Arc::clone(&self.importer)
    }
}

impl Drop for Ingester {
    fn drop(&mut self) {
        self.cancel.cancel();
        let _ = self.commands.send(Command::Stop);
        if let Some(thread) = self.thread.take() {
            // A panic there has already been reported.
            let _ = thread.join();
        }
    }
}

#[derive(Debug, Default)]
struct Shared {
    state: Mutex<Published>,
    published: Condvar,
}

impl Shared {
    fn state(&self) -> MutexGuard<'_, Published> {
        // Nothing that can panic runs under the lock, so a poisoned state is intact.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Debug, Default)]
struct Published {
    registry: Arc<Registry>,
    /// The last [`Ingester::refresh`] ticket handed out.
    tickets_issued: u64,
    /// The last ticket whose rescan is published.
    tickets_served: u64,
}

enum Command {
    /// Something under the root changed.
    Rescan,
    /// Rescan for the [`Ingester::refresh`] holding this ticket.
    Refresh(u64),
    Stop,
}

/// The ingester's thread: rescan whenever asked, until told to stop.
fn work(builder: &Builder, shared: &Shared, commands: &Receiver<Command>, debounce: Duration) {
    loop {
        let Ok(first) = commands.recv() else {
            return;
        };
        let mut ticket = None;
        if !absorb(first, &mut ticket) {
            return;
        }
        // Wait out a burst of changes, but keep no refresh waiting.
        loop {
            let next = if ticket.is_some() {
                commands.try_recv().map_err(|_| RecvTimeoutError::Timeout)
            } else {
                commands.recv_timeout(debounce)
            };
            match next {
                Ok(command) => {
                    if !absorb(command, &mut ticket) {
                        return;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => break,
            }
        }
        let previous = Arc::clone(&shared.state().registry);
        let rebuilt = builder.rebuild(&previous).map(Arc::new);
        // `previous` drops after the lock is released: dropping a Value may run
        // arbitrary code.
        let replaced = {
            let mut state = shared.state();
            let replaced = rebuilt.map(|registry| std::mem::replace(&mut state.registry, registry));
            if let Some(ticket) = ticket {
                state.tickets_served = state.tickets_served.max(ticket);
            }
            replaced
        };
        shared.published.notify_all();
        drop(replaced);
    }
}

/// Fold `command` into the rescan about to happen, keeping the latest refresh
/// ticket in `ticket`. False when told to stop.
fn absorb(command: Command, ticket: &mut Option<u64>) -> bool {
    match command {
        Command::Rescan => true,
        Command::Refresh(t) => {
            *ticket = (*ticket).max(Some(t));
            true
        }
        Command::Stop => false,
    }
}
