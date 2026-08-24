use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use crate::{
    FrostError, MapKey, Value, core::util::identifier::is_identifier_like_and_not_keyword,
};

use super::{Vm, VmFactory};

// White-box tests for the `import` module.
#[cfg(test)]
mod builder_tests;
#[cfg(test)]
mod resolve_tests;

/// Opaque identity of a loaded module, assigned by whichever resolver loaded it.
///
/// The runtime carries a `ModuleId` but never interprets one:
/// its format belongs to the resolver that produced it.
/// A resolver handed an id it does not recognize should treat it as foreign
/// rather than guess at its meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleId(Arc<str>);

impl ModuleId {
    /// Wraps a resolver-assigned identity.
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    /// The identity as the assigning resolver wrote it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a resolver is given for one import: who is importing, and the means to
/// build the [`Vm`] an imported module runs in.
///
/// Borrowed from the importing Vm for the duration of the call:
/// a resolver may use it while resolving, but cannot retain it.
#[derive(Debug)]
pub struct ImportCtx<'vm> {
    factory: VmFactory,
    importing: Option<ModuleId>,
    _vm: PhantomData<&'vm Vm>,
}

impl ImportCtx<'_> {
    pub(super) fn new(factory: VmFactory, importing: Option<ModuleId>) -> Self {
        Self {
            factory,
            importing,
            _vm: PhantomData,
        }
    }

    /// A factory for the Vm an imported module runs in:
    /// the importing Vm's configuration and importer, one import level deeper.
    ///
    /// Resource counters start fresh rather than continuing the importing Vm's;
    /// the limits are a runaway guard, not a budget shared across a module tree.
    pub fn child_factory(&self) -> VmFactory {
        self.factory.clone()
    }

    /// The identity of the module performing this import,
    /// or `None` when the importing script has none
    /// (a REPL, `-e`, or a host-run script left unidentified).
    pub fn importing_module(&self) -> Option<&ModuleId> {
        self.importing.as_ref()
    }
}

/// Resolves module specifications that the import registry does not claim.
/// Registering one is a capability grant; see [`Importer`].
///
/// Implementors own **caching** and **cycle detection**, where applicable.
/// Both belong here because only the resolver knows module identity:
/// one spec may name different modules for different importers,
/// and different specs may name the same module.
/// The runtime caches nothing,
/// so a resolver that does not cache re-runs a module on every import,
/// and one that does not track what it is already loading recurses until
/// [`max_import_depth`](super::VmRuntimeConfiguration::max_import_depth) stops it.
pub trait ImportResolver: std::fmt::Debug + Send + Sync {
    /// Attempt to resolve `module_spec`, which is what the importing script passed to `import`.
    ///
    /// `Ok(Some)` produces the value the importing script receives.
    /// `Ok(None)` means this resolver does not claim the spec, and the next resolver is queried.
    /// `Err` means the spec was claimed but failed to load;
    /// it propagates to the importing script and no further resolvers are queried.
    fn resolve(&self, ctx: &ImportCtx, module_spec: &str) -> Result<Option<Value>, FrostError>;
}

/// The resolver behind Frost's `import`: maps an import specification to a [`Value`].
/// Build one with [`ImporterBuilder`]; the [`Default`] is empty (every import fails).
///
/// # Imports are the capability boundary
///
/// An `Importer` is the whole of what a script can reach beyond the language's own globals.
/// A [`Stdlib`] module, an [`Extension`], a [`HostComponent`], and whatever an [`ImportResolver`]
/// serves differ only in how the content arrives:
/// each ends in native Rust functions, which may do anything.
///
/// The default is no capabilities: a default [`ImporterBuilder`] yields an `Importer` that
/// resolves nothing.
/// What is registered is therefore the answer to "what may a script do".
///
/// Resource limits are a separate concern and not a substitute;
/// see [`VmRuntimeConfiguration`](super::VmRuntimeConfiguration).
#[derive(Debug, Default)]
pub struct Importer {
    // The tree of importable modules, including the stdlib, any extensions, and host-provided functionality.
    registry: Arc<BTreeMap<String, Value>>,
    // The resolver sequence
    resolvers: Arc<[Arc<dyn ImportResolver>]>,
}

/// Builds an [`Importer`] from an import registry and dynamic import resolvers.
/// Begin with [`new`](Self::new); finish with [`build`](Self::build).
///
/// Everything registered here is a capability grant; see [`Importer`].
#[derive(Debug)]
pub struct ImporterBuilder {
    // Partial registry
    registry: BTreeMap<String, Value>,
    // Growing list of resolvers
    resolvers: Vec<Arc<dyn ImportResolver>>,
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
        if is_identifier_like_and_not_keyword(&name) {
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
        if is_identifier_like_and_not_keyword(&name) {
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
    /// Returns this component under a new `name`, validated as in [`new`](Self::new).
    /// The way to resolve a rejection reported by [`ImporterBuilder::with_component`].
    pub fn rename(mut self, name: impl Into<String>) -> Result<Self, InvalidComponentName> {
        let name = name.into();
        if is_identifier_like_and_not_keyword(&name) {
            self.0.name = name;
            Ok(self)
        } else {
            Err(InvalidComponentName(name))
        }
    }

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
        if is_identifier_like_and_not_keyword(&name) {
            Ok(Self(Module { name, content }))
        } else {
            Err(InvalidComponentName(name))
        }
    }
}

/// Rejection from [`ImporterBuilder::with_component`]: the component's name is
/// not claimable. Carries the untouched builder and the rejected component back
/// so the caller can [`rename`](HostComponent::rename) and retry.
#[derive(Debug)]
pub enum HostComponentError {
    /// The name is reserved for the runtime (`std`, `ext`) and can never be claimed.
    ReservedName(ImporterBuilder, HostComponent),
    /// The name was already claimed by an earlier registration.
    NameCollision(ImporterBuilder, HostComponent),
}

impl HostComponentError {
    /// Recovers the builder and the rejected component, whatever the reason.
    pub fn into_parts(self) -> (ImporterBuilder, HostComponent) {
        match self {
            Self::ReservedName(builder, component) | Self::NameCollision(builder, component) => {
                (builder, component)
            }
        }
    }
}

impl std::fmt::Display for HostComponentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReservedName(_, c) => {
                write!(f, "component name `{}` is reserved", c.name())
            }
            Self::NameCollision(_, c) => {
                write!(f, "component name `{}` is already registered", c.name())
            }
        }
    }
}

impl std::error::Error for HostComponentError {}

impl Default for ImporterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Rejection from [`ImporterBuilder::with_extension`]: the name is already
/// claimed under `ext`. Carries the untouched builder and the rejected
/// extension back so the caller can [`rename`](Extension::rename) and retry.
#[derive(Debug)]
pub struct ExtensionError(ImporterBuilder, Extension);

impl ExtensionError {
    /// Recovers the builder and the rejected extension.
    pub fn into_parts(self) -> (ImporterBuilder, Extension) {
        (self.0, self.1)
    }
}

impl std::error::Error for ExtensionError {}

impl std::fmt::Display for ExtensionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "extension name `{}` is already registered",
            self.1.name()
        )
    }
}

impl ImporterBuilder {
    /// A builder with an empty registry and no dynamic resolvers.
    pub fn new() -> ImporterBuilder {
        Self {
            registry: BTreeMap::new(),
            resolvers: Vec::new(),
        }
    }

    /// Registers `extension` under `ext` (imported as `ext.{name}`).
    /// A name already claimed under `ext` is rejected with an [`ExtensionError`]
    /// handing the builder and the extension back unchanged.
    /// Imports under `ext` are a part of the import registry.
    pub fn with_extension(mut self, extension: Extension) -> Result<Self, ExtensionError> {
        // Extensions live under the `ext` namespace: a Map of extension-name -> content.
        let key = MapKey::from(extension.name());

        // A name already claimed under `ext` is a collision. Leave the registry
        // untouched and hand the extension back for the caller to `rename` and retry.
        if let Some(Value::Map(ext)) = self.registry.get("ext")
            && ext.contains_key(&key)
        {
            return Err(ExtensionError(self, extension));
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
    /// `std` and `ext` are reserved; a reserved or already-claimed name is
    /// rejected with a [`HostComponentError`] handing the builder and the
    /// component back unchanged.
    /// These components are a part of the import registry.
    pub fn with_component(mut self, component: HostComponent) -> Result<Self, HostComponentError> {
        // A host component claims a top-level registry name. `std` and `ext` are
        // reserved (the stdlib and extensions); every other name is the host's to
        // claim, unless already taken. Reservation is by *name*, not by presence,
        // so it holds regardless of whether `std`/`ext` are populated yet.
        let name = component.name();
        if name == "std" || name == "ext" {
            return Err(HostComponentError::ReservedName(self, component));
        }
        if self.registry.contains_key(name) {
            return Err(HostComponentError::NameCollision(self, component));
        }

        self.registry.insert(component.0.name, component.0.content);
        Ok(self)
    }

    /// Installs the standard library into `std`, replacing any already present.
    /// The stdlib is a part of the import registry.
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

    /// Appends a dynamic import resolver.
    /// Resolvers are queried in registration order, and only for specifications
    /// the import registry does not claim.
    pub fn append_resolver(mut self, resolver: Arc<dyn ImportResolver>) -> Self {
        self.resolvers.push(resolver);
        self
    }

    /// Finalizes into a shared [`Importer`].
    pub fn build(self) -> Arc<Importer> {
        Arc::new(Importer {
            registry: Arc::new(self.registry),
            resolvers: Arc::from(self.resolvers),
        })
    }
}

impl Importer {
    pub(crate) fn import(&self, target: &str, ctx: &ImportCtx) -> Result<Value, FrostError> {
        // `target` is a `.`-separated path. The first segment selects a top-level
        // registry entry; the rest descend through nested Maps. A top-level hit is
        // registry-exclusive and never reaches a resolver, so `std`/`ext` and any
        // host component own their whole subtree and cannot be shadowed.
        if target.is_empty() {
            return Err(FrostError::from_static(
                "import requires a non-empty module name",
            ));
        }

        let mut segments = target.split('.');
        let first = segments
            .next()
            .expect("split always yields a first segment");

        let Some(root) = self.registry.get(first) else {
            return self.resolve_dynamic(target, ctx);
        };

        // Bind through each remaining segment: descend into a Map, else fail. A miss
        // or a non-Map value along the way is a final error, never a resolver fallback.
        segments
            .try_fold((first, root), |(name, current), segment| match current {
                Value::Map(map) => {
                    let next = map.get_str(segment).ok_or_else(|| {
                        FrostError::from_string(format!("Could not resolve import '{target}'"))
                    })?;
                    Ok((segment, next))
                }
                _ => Err(FrostError::from_string(format!(
                    "Cannot import '{target}': '{name}' is not a module"
                ))),
            })
            .map(|(_, value)| value.clone())
    }

    /// Offer an unclaimed specification to each resolver in registration order.
    /// The first to claim it wins; a resolver's error ends the search.
    fn resolve_dynamic(&self, target: &str, ctx: &ImportCtx) -> Result<Value, FrostError> {
        for resolver in self.resolvers.iter() {
            if let Some(value) = resolver.resolve(ctx, target)? {
                return Ok(value);
            }
        }
        Err(FrostError::from_string(format!(
            "Could not resolve import '{target}'"
        )))
    }
}
