//! One rebuild: rescanning the root and reloading what changed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread::{self, ThreadId};

use frostlang::compile::{CompilerOptions, Diagnostics, compile_program};
use frostlang::{
    CancelToken, Closure, FrostError, ImportCtx, ImportResolver, MapKey, ModuleId, RunError, Value,
    ValueMap, Vm, VmFactory,
};

use crate::ingest::Shared;
use crate::ingest::registry::{LoadError, Module, Registry};
use crate::ingest::scan::{Found, is_module_name, scan};

/// Rebuilds registries, on the ingester's thread.
pub(super) struct Builder {
    pub(super) root: PathBuf,
    pub(super) factory: VmFactory,
    pub(super) cancel: CancelToken,
    pub(super) resolver: Arc<IngestResolver>,
}

impl Builder {
    /// The registry after `previous`, or `None` when no module file changed.
    ///
    /// A module reloads when its file changed, or when a module it imports
    /// reloads; every other module carries over as it was.
    pub(super) fn rebuild(&self, previous: &Arc<Registry>) -> Option<Registry> {
        let scanned = scan(&self.root);
        let changed: BTreeSet<&str> = scanned
            .iter()
            .filter(|(name, found)| {
                previous
                    .get(name)
                    .is_none_or(|module| module.source != found.source)
            })
            .map(|(name, _)| name.as_str())
            .chain(
                previous
                    .modules
                    .keys()
                    .map(String::as_str)
                    .filter(|name| !scanned.contains_key(*name)),
            )
            .collect();
        if changed.is_empty() {
            return None;
        }
        let dirty: BTreeSet<String> = with_importers(previous, changed)
            .into_iter()
            .filter(|name| scanned.contains_key(name))
            .collect();
        let generation = previous.generation + 1;
        *self.resolver.state() = Some(Build {
            scanned,
            previous: Arc::clone(previous),
            dirty: dirty.clone(),
            done: BTreeMap::new(),
            loading: Vec::new(),
            imports: BTreeMap::new(),
        });
        for name in &dirty {
            // Its outcome is recorded in the build; the result here is a copy.
            let _ = self.resolver.load(name, |closure| {
                Ok(self
                    .factory
                    .build(closure)
                    .with_cancel_token(self.cancel.clone()))
            });
        }
        let build = self
            .resolver
            .state()
            .take()
            .expect("the build is in place until it is taken");
        Some(build.into_registry(generation))
    }
}

/// `names`, and every module of `registry` that imports any of them, however
/// indirectly.
fn with_importers(registry: &Registry, names: BTreeSet<&str>) -> BTreeSet<String> {
    let mut found: BTreeSet<String> = names.iter().map(|&name| name.to_owned()).collect();
    let mut pending: Vec<String> = found.iter().cloned().collect();
    while let Some(name) = pending.pop() {
        for importer in registry.modules() {
            if importer.imports.contains(&name) && found.insert(importer.name.clone()) {
                pending.push(importer.name.clone());
            }
        }
    }
    found
}

/// A rebuild in progress.
struct Build {
    scanned: BTreeMap<String, Found>,
    previous: Arc<Registry>,
    /// The modules to reload.
    dirty: BTreeSet<String>,
    /// The modules reloaded so far.
    done: BTreeMap<String, Loaded>,
    /// The modules loading, outermost first: each is importing the next.
    loading: Vec<String>,
    /// What each module reloaded so far imported.
    imports: BTreeMap<String, BTreeSet<String>>,
}

/// A module reloaded by a build.
struct Loaded {
    outcome: Result<Value, LoadError>,
    /// `None` when it did not compile.
    lints: Option<Diagnostics>,
}

impl Build {
    fn into_registry(mut self, generation: u64) -> Registry {
        let modules = self
            .scanned
            .into_iter()
            .map(|(name, found)| {
                let module = match self.done.remove(&name) {
                    Some(loaded) => {
                        let previous = self.previous.get(&name);
                        Module {
                            last_good: match &loaded.outcome {
                                Ok(exports) => Some(exports.clone()),
                                Err(_) => previous.and_then(|module| module.last_good.clone()),
                            },
                            lints: loaded
                                .lints
                                .or_else(|| previous.map(|module| module.lints.clone()))
                                .unwrap_or_default(),
                            imports: self.imports.remove(&name).unwrap_or_default(),
                            name: name.clone(),
                            path: found.path,
                            source: found.source,
                            generation,
                            outcome: loaded.outcome,
                        }
                    }
                    None => self
                        .previous
                        .get(&name)
                        .expect("a module not reloaded is unchanged, so was there before")
                        .clone(),
                };
                (name, module)
            })
            .collect();
        Registry {
            generation,
            modules,
        }
    }
}

/// The ingester's resolver. On the ingester's thread, it serves imports
/// between modules during a build, reloading a module on its first import if
/// it is due; on any other thread, it serves the latest registry.
//
// Only builds run Vms on the ingester's thread, so the thread alone tells
// which an import is for.
// TODO: unsound once Frost can run on other threads within a run, as a
// parallel map would: a build's imports from those threads would be served
// the latest registry instead of the build.
#[derive(Debug)]
pub(super) struct IngestResolver {
    options: CompilerOptions,
    shared: Arc<Shared>,
    builder_thread: OnceLock<ThreadId>,
    // `None` between builds.
    state: Mutex<Option<Build>>,
}

impl std::fmt::Debug for Build {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Build")
            .field("dirty", &self.dirty)
            .field("loading", &self.loading)
            .finish_non_exhaustive()
    }
}

impl IngestResolver {
    pub(super) fn new(options: CompilerOptions, shared: Arc<Shared>) -> Self {
        Self {
            options,
            shared,
            builder_thread: OnceLock::new(),
            state: Mutex::default(),
        }
    }

    /// Make the calling thread the one builds run on, before any build.
    pub(super) fn claim_thread(&self) {
        self.builder_thread
            .set(thread::current().id())
            .expect("one thread builds, for the resolver's whole life");
    }

    fn on_builder_thread(&self) -> bool {
        self.builder_thread.get() == Some(&thread::current().id())
    }

    fn state(&self) -> MutexGuard<'_, Option<Build>> {
        // Nothing that can panic runs under the lock, so a poisoned build is intact.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Reload the module `name` in the current build, unless the build already
    /// has, on a Vm from `vm`.
    fn load(
        &self,
        name: &str,
        vm: impl FnOnce(Arc<Closure>) -> Result<Vm, FrostError>,
    ) -> Result<Value, LoadError> {
        let found = {
            let mut state = self.state();
            let build = state.as_mut().expect("modules load only during a build");
            if let Some(loaded) = build.done.get(name) {
                return loaded.outcome.clone();
            }
            if let Some(start) = build.loading.iter().position(|loading| loading == name) {
                let chain: Vec<&str> = build.loading[start..]
                    .iter()
                    .map(String::as_str)
                    .chain([name])
                    .collect();
                return Err(LoadError::Run(FrostError::from_string(format!(
                    "Import cycle: {}",
                    chain.join(" imports ")
                ))));
            }
            build.loading.push(name.to_owned());
            let found = &build.scanned[name];
            (found.path.clone(), found.source.clone())
        };
        let loaded = match found {
            (_, Err(reason)) => Loaded {
                outcome: Err(LoadError::Unreadable(reason)),
                lints: None,
            },
            (path, Ok(source)) => self.compile_and_run(name, &path.to_string_lossy(), &source, vm),
        };
        let outcome = loaded.outcome.clone();
        let mut state = self.state();
        let build = state.as_mut().expect("the build outlasts its loads");
        build.loading.pop();
        build.done.insert(name.to_owned(), loaded);
        outcome
    }

    fn compile_and_run(
        &self,
        name: &str,
        filename: &str,
        source: &str,
        vm: impl FnOnce(Arc<Closure>) -> Result<Vm, FrostError>,
    ) -> Loaded {
        let output = match compile_program(filename, source, self.options) {
            Ok(output) => output,
            Err(diagnostics) => {
                return Loaded {
                    outcome: Err(LoadError::Compile(diagnostics)),
                    lints: None,
                };
            }
        };
        let closure = output
            .code
            .into_closure()
            .expect("a module compiles in no enclosing scope, so it captures nothing");
        let outcome = vm(closure)
            .and_then(|vm| {
                vm.with_module_id(ModuleId::new(name))
                    .run()
                    .map_err(RunError::into_error)
            })
            .map(|result| {
                let exports: ValueMap = result
                    .into_parts()
                    .exports
                    .into_iter()
                    .map(|(name, value)| (MapKey::from(name), value))
                    .collect();
                Value::Map(exports.into())
            })
            .map_err(LoadError::Run);
        Loaded {
            outcome,
            lints: Some(output.lints),
        }
    }
}

impl ImportResolver for IngestResolver {
    fn resolve(&self, ctx: &ImportCtx, spec: &str) -> Result<Option<Value>, FrostError> {
        if !is_module_name(spec) {
            return Ok(None);
        }
        if !self.on_builder_thread() {
            let registry = Arc::clone(&self.shared.state().registry);
            return registry
                .get(spec)
                .map(|module| {
                    module
                        .outcome()
                        .cloned()
                        .map_err(|error| cannot_import(spec, error))
                })
                .transpose();
        }
        let carried = {
            let mut state = self.state();
            let build = state
                .as_mut()
                .expect("Vms run on the ingester's thread only during builds");
            // Recorded even when `spec` is absent, so that its arrival reloads
            // the importer.
            if let Some(importer) = ctx.importing_module() {
                build
                    .imports
                    .entry(importer.as_str().to_owned())
                    .or_default()
                    .insert(spec.to_owned());
            }
            if !build.scanned.contains_key(spec) {
                return Ok(None);
            }
            if build.dirty.contains(spec) {
                None
            } else {
                Some(
                    build
                        .previous
                        .get(spec)
                        .expect("a module not due to reload is unchanged, so was there before")
                        .outcome
                        .clone(),
                )
            }
        };
        let outcome = match carried {
            Some(outcome) => outcome,
            None => self.load(spec, |closure| ctx.child_factory().build(closure)),
        };
        outcome
            .map(Some)
            .map_err(|error| cannot_import(spec, &error))
    }
}

/// The error for an import of `spec`, whose module failed with `error`.
fn cannot_import(spec: &str, error: &LoadError) -> FrostError {
    FrostError::from_string(format!("Cannot import '{spec}': {error}"))
}
