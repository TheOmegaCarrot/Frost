//! Frost modules from script files: an [`ImportResolver`] that serves
//! `import('a.b.c')` from the file `a/b/c.frst` in one of a list of directories.
//!
//! ```no_run
//! use std::sync::Arc;
//!
//! use frostlang_fs_resolver::FsResolver;
//! use frostlang_runtime::ImporterBuilder;
//!
//! let resolver = FsResolver::new(["/usr/share/my-app/frost", "scripts"])?;
//! let importer = ImporterBuilder::new()
//!     .append_resolver(Arc::new(resolver))
//!     .build();
//! # Ok::<(), frostlang_fs_resolver::RootError>(())
//! ```
//!
//! # Resolution
//!
//! A specification names a file when it is a `.`-separated path of names, each
//! a letter or underscore followed by letters, digits, and underscores:
//! `a.b.c` names `a/b/c.frst`. The roots are searched in order, and the first
//! with that file serves the import.
//! A specification of any other shape, or one no root has a file for, is left
//! to the next resolver.
//! Once a file is found, every failure to import it is an error for the
//! importing script, including one from the filesystem.
//!
//! An import receives a Map of the module's `export`ed bindings.
//!
//! An [`Importer`](frostlang_runtime::Importer)'s registry is consulted before
//! any resolver, so a file whose first name is one the registry claims, such as
//! `std/math.frst`, is never imported.
//!
//! # Caching
//!
//! A module is its file, by canonical path. With caching on, as it is by
//! default, a module runs once: every later import of its file, by whatever
//! specification, receives the same exports. With caching off, every import
//! runs the module afresh.
//!
//! On a filesystem that ignores case, specifications that differ only in case,
//! such as `Util` and `util`, find the same file. Whether they are one module
//! depends on the platform: some report a canonical path in the file's own case,
//! making them one; others keep the case of the specification, making them two,
//! each running separately with its own exports.
//!
//! # Symbolic links
//!
//! By default, symbolic links below the roots are not followed: a link to a file
//! or to a directory, anywhere on the way from a root to a module's file, makes
//! importing that module an error. A root itself may be a link.
//!
//! Following them, with [`FsResolver::with_symlinks_followed`], lets imports pass
//! through links to directories as well as files, wherever they lead, including
//! outside the roots. The module is then the file the links lead to, so two
//! links to one file are one module.
//!
//! # Cycles and concurrency
//!
//! A module that imports itself, however indirectly, is an import cycle: the
//! import that closes it is an error naming the chain.
//!
//! One `FsResolver` may serve many [`Vm`](frostlang_runtime::Vm)s, on many
//! threads, and an import never waits for another thread. When threads import
//! a module that is not yet cached at the same time, each runs it, and the first
//! to finish supplies the exports they all receive. A module's top level may
//! therefore run more than once, so keep it to defining what it exports.
//! A cycle through another thread, such as a module whose top level waits on a
//! thread that imports it, is not detected.
//!
//! A module runs under the configuration of the Vm that first imports it, and
//! with caching, every Vm then shares the result. Use one `FsResolver` for each
//! set of Vms that share a configuration.

mod loading;
mod lookup;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt::{self, Display};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::{fs, io};

use frostlang_compile::{CompilerOptions, Diagnostics, OptimizationOptions, compile_program};
use frostlang_runtime::{
    FrostError, ImportCtx, ImportResolver, MapKey, ModuleId, RunError, Value, ValueMap,
};

use loading::{Loading, ResolverId};

/// Serves imports from Frost script files under a list of root directories.
/// See the [crate documentation](crate) for how.
///
/// Configure it, then register it with
/// [`ImporterBuilder::append_resolver`](frostlang_runtime::ImporterBuilder::append_resolver).
/// Registering it grants scripts every module under its roots.
#[derive(Debug)]
pub struct FsResolver {
    // Canonical, so a later change of working directory cannot move them.
    roots: Vec<PathBuf>,
    follow_symlinks: bool,
    caching: bool,
    diagnostics: DiagnosticStyle,
    optimization: OptimizationOptions,
    // Each module's exports, by canonical path. Locked only to read or insert,
    // never while a module runs, so no import waits on another.
    cache: Mutex<HashMap<PathBuf, Value>>,
    id: ResolverId,
}

/// How an import error renders a module's compile errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagnosticStyle {
    /// Monochrome ASCII; see [`Diagnostics::render_plain`].
    #[default]
    Plain,
    /// Unicode box-drawing without color; see [`Diagnostics::render_unicode`].
    Unicode,
}

/// A root that [`FsResolver::new`] cannot use: it is missing, inaccessible, or
/// not a directory.
#[derive(Debug)]
pub struct RootError {
    root: PathBuf,
    error: io::Error,
}

impl RootError {
    /// The root as it was given.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Display for RootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cannot use '{}' as a module root: {}",
            self.root.display(),
            self.error
        )
    }
}

impl std::error::Error for RootError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

impl FsResolver {
    /// A resolver searching `roots`, in order, with caching on, symbolic links
    /// not followed, [`DiagnosticStyle::Plain`], and every optimization.
    ///
    /// Each root is resolved to its canonical path now, so a relative root is
    /// relative to the current working directory at this call.
    pub fn new(roots: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<Self, RootError> {
        let roots = roots
            .into_iter()
            .map(|root| canonical_root(root.as_ref()))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            roots,
            follow_symlinks: false,
            caching: true,
            diagnostics: DiagnosticStyle::default(),
            optimization: OptimizationOptions::ALL,
            cache: Mutex::default(),
            id: ResolverId::unique(),
        })
    }

    /// Whether to follow symbolic links below the roots; see the
    /// [crate documentation](crate#symbolic-links).
    pub fn with_symlinks_followed(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    /// Whether to cache modules; see the [crate documentation](crate#caching).
    pub fn with_caching(mut self, caching: bool) -> Self {
        self.caching = caching;
        self
    }

    /// How an import error renders a module's compile errors.
    pub fn with_diagnostics(mut self, style: DiagnosticStyle) -> Self {
        self.diagnostics = style;
        self
    }

    /// The optimizations modules compile with.
    pub fn with_optimization(mut self, optimization: OptimizationOptions) -> Self {
        self.optimization = optimization;
        self
    }

    /// The roots, canonical, in search order.
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Compile and run the module at `path`, imported as `spec`, for its exports.
    fn load(&self, ctx: &ImportCtx, spec: &str, path: &Path) -> Result<Value, FrostError> {
        let source = fs::read_to_string(path).map_err(|error| {
            cannot_import(
                spec,
                format_args!("cannot read '{}': {error}", path.display()),
            )
        })?;
        let filename = path.to_string_lossy();
        let options = CompilerOptions {
            optimization_options: self.optimization,
            implicit_export: false,
        };
        let program = compile_program(&filename, &source, options).map_err(|errors| {
            cannot_import(
                spec,
                format_args!(
                    "'{}' does not compile:\n{}",
                    path.display(),
                    self.render(&errors)
                ),
            )
        })?;
        let closure = program
            .code
            .into_closure()
            .expect("a module compiles in no enclosing scope, so it captures nothing");
        let result = ctx
            .child_factory()
            .build(closure)?
            .with_module_id(ModuleId::new(filename.as_ref()))
            .run()
            .map_err(RunError::into_error)?;
        let exports: ValueMap = result
            .exports()
            .map(|(name, value)| (MapKey::from(name), value.clone()))
            .collect();
        Ok(Value::Map(exports.into()))
    }

    fn render(&self, errors: &Diagnostics) -> String {
        match self.diagnostics {
            DiagnosticStyle::Plain => errors.render_plain(),
            DiagnosticStyle::Unicode => errors.render_unicode(),
        }
    }

    fn cache(&self) -> MutexGuard<'_, HashMap<PathBuf, Value>> {
        // Nothing that can panic runs under the lock, so a poisoned map is intact.
        self.cache.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The exports cached for `path`, caching `exports` there if there are none.
    fn cached_or_insert(&self, path: PathBuf, exports: Value) -> Value {
        let cached = match self.cache().entry(path) {
            Entry::Occupied(entry) => Some(entry.get().clone()),
            Entry::Vacant(entry) => {
                entry.insert(exports.clone());
                None
            }
        };
        // Any `exports` not kept drops here, outside the lock: dropping a Value
        // may run arbitrary code.
        cached.unwrap_or(exports)
    }
}

impl ImportResolver for FsResolver {
    fn resolve(&self, ctx: &ImportCtx, module_spec: &str) -> Result<Option<Value>, FrostError> {
        let Some(path) = self.find(module_spec)? else {
            return Ok(None);
        };
        if self.caching
            && let Some(exports) = self.cache().get(&path).cloned()
        {
            return Ok(Some(exports));
        }
        let _loading = Loading::enter(self.id, module_spec, &path)?;
        let exports = self.load(ctx, module_spec, &path)?;
        Ok(Some(if self.caching {
            self.cached_or_insert(path, exports)
        } else {
            exports
        }))
    }
}

/// `root` as a canonical path, if it is a directory.
fn canonical_root(root: &Path) -> Result<PathBuf, RootError> {
    let error = |error| RootError {
        root: root.to_path_buf(),
        error,
    };
    let canonical = fs::canonicalize(root).map_err(error)?;
    if canonical.is_dir() {
        Ok(canonical)
    } else {
        Err(error(io::ErrorKind::NotADirectory.into()))
    }
}

/// The error for an import of `spec` that failed for `reason`.
fn cannot_import(spec: &str, reason: impl Display) -> FrostError {
    FrostError::from_string(format!("Cannot import '{spec}': {reason}"))
}
