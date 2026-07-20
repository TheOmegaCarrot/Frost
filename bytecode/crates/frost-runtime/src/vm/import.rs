use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use crate::{
    FrostError, MapKey, Value, core::util::identifier::is_identifier_like_and_not_keyword,
};

/// The resolver behind Frost's `import`: maps an import specification to a [`Value`].
/// Build one with [`ImporterBuilder`].
#[derive(Debug)]
pub struct Importer {
    // The tree of importable modules, including the stdlib, any extensions, and host-provided functionality.
    registry: Arc<BTreeMap<String, Value>>,
    // The search path for filesystem imports.
    file_search_path: Option<Arc<[PathBuf]>>,
    // Virtual CWD of the Importer.
    cwd: Option<PathBuf>,
    // TODO: filesystem import cache
}

/// Builds an [`Importer`] from built-in modules and file-import settings.
/// Begin with [`new`](Self::new); finish with [`build`](Self::build).
#[derive(Debug)]
pub struct ImporterBuilder {
    // Partial registry
    registry: BTreeMap<String, Value>,
    file_search_path: Option<Arc<[PathBuf]>>,
    cwd: Option<PathBuf>,
}

// Basic type the ImporterBuilder uses to build up the registry
#[derive(Debug)]
struct Module {
    name: String,
    content: Value,
}

/// One standard-library module, such as the one imported as `std.math`.
/// A [`Stdlib`] is a collection of them.
// Intentionally not constructible outside this crate.
#[derive(Debug)]
pub struct StdlibModule(Module);

// TODO: configuration API for building a Stdlib
// Blocked by: a stdlib (developed in this crate to specially-reserve the `std` import prefix)

/// The complete standard library, installed with [`ImporterBuilder::with_stdlib`].
#[derive(Debug)]
pub struct Stdlib {
    modules: Vec<StdlibModule>,
}

/// Third-party library module, registerable within the import registry.
/// Always has its content placed under the `ext` path for importing.
/// For example, an `Extension` named `sqlite` will have its content importable as
/// `import('ext.sqlite')`.
#[derive(Debug)]
pub struct Extension(Module);

/// A rejected name for an [`Extension`] or [`HostComponent`]: not identifier-like, or a Frost keyword.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidComponentName(String);

impl InvalidComponentName {
    /// The rejected name.
    pub fn invalid_name(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for InvalidComponentName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid component name {:?}: names must be identifier-like and must not be a Frost keyword",
            self.0
        )
    }
}

impl std::error::Error for InvalidComponentName {}

impl Extension {
    /// Returns this extension under a new `name`, validated as in [`new`](Self::new).
    /// The way to resolve a collision reported by [`ImporterBuilder::with_extension`].
    pub fn rename(mut self, name: impl Into<String>) -> Result<Self, InvalidComponentName> {
        let name = name.into();
        if is_identifier_like_and_not_keyword(name.as_bytes()) {
            self.0.name = name;
            Ok(self)
        } else {
            Err(InvalidComponentName(name))
        }
    }

    /// The name this extension registers under (imported as `ext.{name}`).
    pub fn name(&self) -> &str {
        &self.0.name
    }

    /// Construct a new Extension.
    /// `name` must be a string that adheres to Frost's identifier rules. This includes forbidding
    /// Frost keywords being used as `Extension` names.
    /// `content` may be any Frost Value, but is generally a nested Map structure.
    pub fn new(name: impl Into<String>, content: Value) -> Result<Self, InvalidComponentName> {
        let name = name.into();
        if is_identifier_like_and_not_keyword(name.as_bytes()) {
            Ok(Self(Module { name, content }))
        } else {
            Err(InvalidComponentName(name))
        }
    }
}

/// A host-defined import registry component.
/// Intended for use by a host application to provide their own application-specific functionality to scripts.
/// A host component is added to the top-level of the import registry.
#[derive(Debug)]
pub struct HostComponent(Module);

impl HostComponent {
    /// The top-level name this component registers under.
    pub fn name(&self) -> &str {
        &self.0.name
    }

    /// Construct a new HostComponent.
    /// `name` must be a string that adheres to Frost's identifier rules. This includes forbidding
    /// Frost keywords being used as `HostComponent` names.
    /// `content` may be any Frost Value, but is generally a nested Map structure.
    pub fn new(name: impl Into<String>, content: Value) -> Result<Self, InvalidComponentName> {
        let name = name.into();
        if is_identifier_like_and_not_keyword(name.as_bytes()) {
            Ok(Self(Module { name, content }))
        } else {
            Err(InvalidComponentName(name))
        }
    }
}

impl ImporterBuilder {
    /// A builder with an empty registry and file-based imports disabled.
    pub fn new() -> ImporterBuilder {
        Self {
            registry: BTreeMap::new(),
            file_search_path: None,
            cwd: None,
        }
    }

    /// Registers `extension` under `ext` (imported as `ext.{name}`).
    /// A name already claimed under `ext` returns `Err((builder, extension))` unchanged,
    /// for the caller to [`rename`](Extension::rename) and retry.
    pub fn with_extension(mut self, extension: Extension) -> Result<Self, (Self, Extension)> {
        // Extensions live under the `ext` namespace: a Map of extension-name -> content.
        let key = MapKey::from(extension.name());

        // A name already claimed under `ext` is a collision. Leave the registry
        // untouched and hand the extension back for the caller to `rename` and retry.
        if let Some(Value::Map(ext)) = self.registry.get("ext")
            && ext.contains_key(&key)
        {
            return Err((self, extension));
        }

        // Take the `ext` submap (or start one), insert, and put it back. The builder
        // holds the only reference to it, so `into_map` steals rather than copies.
        let mut ext = match self.registry.remove("ext") {
            Some(Value::Map(ext)) => ext.into_map(),
            None => BTreeMap::new(),
            Some(_) => unreachable!("the `ext` registry entry is always a Map"),
        };
        ext.insert(key, extension.0.content);
        self.registry
            .insert("ext".to_string(), Value::Map(ext.into()));
        Ok(self)
    }

    /// Registers `component` at a top-level name (imported as `{name}`).
    /// `std` and `ext` are reserved; a reserved or already-claimed name returns
    /// `Err((builder, component))` unchanged.
    pub fn with_component(
        mut self,
        component: HostComponent,
    ) -> Result<Self, (Self, HostComponent)> {
        // A host component claims a top-level registry name. `std` and `ext` are
        // reserved (the stdlib and extensions); every other name is the host's to
        // claim, unless already taken. Rejection is by *name*, not by presence, so
        // the reservation holds regardless of whether `std`/`ext` are populated yet.
        let name = component.name();
        let unavailable = name == "std" || name == "ext" || self.registry.contains_key(name);
        if unavailable {
            // Leave the registry untouched and hand the component back to the caller.
            return Err((self, component));
        }

        self.registry.insert(component.0.name, component.0.content);
        Ok(self)
    }

    /// Installs the standard library into `std`, replacing any already present.
    pub fn with_stdlib(mut self, stdlib: Stdlib) -> Self {
        // The `std` namespace is a Map of module-name -> content, replaced wholesale:
        // the stdlib is crate-controlled and has a single source.
        let modules = stdlib
            .modules
            .into_iter()
            .map(|StdlibModule(module)| (MapKey::from(module.name), module.content))
            .collect::<BTreeMap<MapKey, Value>>();
        self.registry
            .insert("std".to_string(), Value::Map(modules.into()));
        self
    }

    /// Sets the directories searched for file-based imports.
    /// Unset by default, which disables file-based imports.
    pub fn with_file_search_path(mut self, path: Arc<[PathBuf]>) -> Self {
        self.file_search_path = Some(path);
        self
    }

    /// Sets the directory file-based imports resolve against first, before the search path.
    pub fn with_working_directory(mut self, path: PathBuf) -> Self {
        self.cwd = Some(path);
        self
    }

    /// Finalizes into a shared [`Importer`].
    pub fn build(self) -> Arc<Importer> {
        Arc::new(Importer {
            registry: Arc::new(self.registry),
            file_search_path: self.file_search_path,
            cwd: self.cwd,
        })
    }
}

impl Importer {
    fn import(&self, target: &str) -> Result<Value, FrostError> {
        // `target` is a `.`-separated sequence of names.
        // Interpret as chained access into the registry,
        // or, if that misses, try walking the CWD, then file search path,
        // replacing `.` with path separators
        //
        // If registry hit, just return the Value it lands on, regardless of type (usually Map).
        // If fails over to filesystem, compile the module, run it, and put exports into a Map,
        // cached in the Importer. Filesystem imports get a separate Importer, with a
        // reconfigured search path, but a shared registry and filesystem cache.
        // This all has to be parallel-safe, handle diamond-imports, and weird parallel cases.
        //
        // Caveat: if the registry contains the top-level component, or if the top-level component
        // is `std` or `ext`, then it can *only* resolve inside the registry.

        todo!();
    }
}
