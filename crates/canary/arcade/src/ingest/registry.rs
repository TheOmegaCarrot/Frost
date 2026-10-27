//! What the ingester publishes: an immutable snapshot of every module it found.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use frostlang::compile::Diagnostics;
use frostlang::{FrostError, Value};

/// Every module under the ingester's root, as of one rebuild.
///
/// A registry never changes; the ingester publishes a new one, with a higher
/// [`generation`](Self::generation), whenever a rebuild changes anything.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    pub(super) generation: u64,
    pub(super) modules: BTreeMap<String, Module>,
}

impl Registry {
    /// Counts rebuilds that changed something; the empty registry before the
    /// first build is generation 0.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The module named `name`, such as `a.b` for the file `a/b.frst`.
    pub fn get(&self, name: &str) -> Option<&Module> {
        self.modules.get(name)
    }

    /// Every module, by name.
    pub fn modules(&self) -> impl Iterator<Item = &Module> {
        self.modules.values()
    }
}

/// One module file, as last loaded.
#[derive(Debug, Clone)]
pub struct Module {
    pub(super) name: String,
    pub(super) path: PathBuf,
    // The file's text, or why it could not be read.
    pub(super) source: Result<Arc<str>, String>,
    pub(super) generation: u64,
    pub(super) outcome: Result<Value, LoadError>,
    pub(super) last_good: Option<Value>,
    pub(super) lints: Diagnostics,
    pub(super) imports: BTreeSet<String>,
}

impl Module {
    /// The module's name, as `import` takes it.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The module's file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The [registry generation](Registry::generation) that last loaded this
    /// module. It changes when the module's file changes, and when any module
    /// it imports is reloaded.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The module's exports, as a Map, or why the latest load failed.
    pub fn outcome(&self) -> Result<&Value, &LoadError> {
        self.outcome.as_ref()
    }

    /// The exports of the latest load that succeeded, if any has. After a
    /// failed reload, this is the version before it.
    pub fn last_good(&self) -> Option<&Value> {
        self.last_good.as_ref()
    }

    /// What the compiler noticed about the latest version that compiled.
    pub fn lints(&self) -> &Diagnostics {
        &self.lints
    }

    /// The modules under the root that this one imported while it loaded,
    /// including any it asked for that do not exist.
    pub fn imports(&self) -> impl Iterator<Item = &str> {
        self.imports.iter().map(String::as_str)
    }
}

/// Why a module failed to load.
#[derive(Debug, Clone)]
pub enum LoadError {
    /// The file could not be read as UTF-8 text.
    Unreadable(String),
    /// The source did not compile.
    Compile(Diagnostics),
    /// The module's top level failed, including by importing a module that
    /// failed.
    Run(FrostError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(reason) => write!(f, "cannot read the file: {reason}"),
            Self::Compile(diagnostics) => {
                write!(f, "does not compile:\n{}", diagnostics.render_narrated())
            }
            Self::Run(error) => write!(f, "{}", error.with_backtrace()),
        }
    }
}

impl std::error::Error for LoadError {}
