//! White-box tests for the import registry builder.
//!
//! The `import` module is not re-exported from the crate root, so this
//! registration behavior is only reachable from inside the crate. Once
//! `Importer::import` exists, resolution gets its own black-box coverage.

use std::path::{Path, PathBuf};

use crate::{FrostMap, Value};

use super::*;

/// A distinct sentinel leaf, so registered content can be told apart.
fn content(n: i64) -> Value {
    Value::from(n)
}

/// The `ext` namespace submap of a builder's registry.
fn ext_submap(builder: &ImporterBuilder) -> &FrostMap {
    match builder.registry.get("ext") {
        Some(Value::Map(m)) => m,
        other => panic!("expected `ext` to be a Map, got {other:?}"),
    }
}

/// The `std` namespace submap of a builder's registry.
fn std_submap(builder: &ImporterBuilder) -> &FrostMap {
    match builder.registry.get("std") {
        Some(Value::Map(m)) => m,
        other => panic!("expected `std` to be a Map, got {other:?}"),
    }
}

/// A stdlib module (its own builder is crate-internal and not yet designed).
fn stdlib_module(name: &str, content: Value) -> StdlibModule {
    StdlibModule(Module {
        name: name.to_string(),
        content,
    })
}

// -- Builder defaults / filesystem settings --

#[test]
fn new_builder_is_empty() {
    let b = ImporterBuilder::new();
    assert!(b.registry.is_empty());
    assert!(b.file_search_path.is_none());
    assert!(b.cwd.is_none());
}

#[test]
fn filesystem_settings_are_recorded() {
    let importer = ImporterBuilder::new()
        .with_working_directory(PathBuf::from("/scripts"))
        .with_file_search_path(vec![PathBuf::from("/lib")].into())
        .build();
    assert_eq!(importer.cwd.as_deref(), Some(Path::new("/scripts")));
    assert!(importer.file_search_path.is_some());
}

// -- Extensions (under `ext`) --

#[test]
fn extension_registers_under_ext() {
    let b = ImporterBuilder::new()
        .with_extension(Extension::new("sqlite", content(1)).unwrap())
        .unwrap();
    assert_eq!(ext_submap(&b).get_str("sqlite"), Some(&content(1)));
}

#[test]
fn multiple_extensions_coexist() {
    let b = ImporterBuilder::new()
        .with_extension(Extension::new("a", content(1)).unwrap())
        .unwrap()
        .with_extension(Extension::new("b", content(2)).unwrap())
        .unwrap();
    let ext = ext_submap(&b);
    assert_eq!(ext.get_str("a"), Some(&content(1)));
    assert_eq!(ext.get_str("b"), Some(&content(2)));
    assert_eq!(ext.len(), 2);
}

#[test]
fn extension_name_collision_is_rejected_intact() {
    let b = ImporterBuilder::new()
        .with_extension(Extension::new("dup", content(1)).unwrap())
        .unwrap();
    let (b, returned) = b
        .with_extension(Extension::new("dup", content(2)).unwrap())
        .unwrap_err()
        .into_parts();
    // The rejected extension comes back for recovery...
    assert_eq!(returned.name(), "dup");
    // ...and the registry is untouched: the first registration stands.
    assert_eq!(ext_submap(&b).get_str("dup"), Some(&content(1)));
}

#[test]
fn renaming_resolves_an_extension_collision() {
    let b = ImporterBuilder::new()
        .with_extension(Extension::new("dup", content(1)).unwrap())
        .unwrap();
    let (b, returned) = b
        .with_extension(Extension::new("dup", content(2)).unwrap())
        .unwrap_err()
        .into_parts();
    let b = b.with_extension(returned.rename("dup2").unwrap()).unwrap();
    let ext = ext_submap(&b);
    assert_eq!(ext.get_str("dup"), Some(&content(1)));
    assert_eq!(ext.get_str("dup2"), Some(&content(2)));
}

// -- Host components (top-level) --

#[test]
fn component_registers_at_top_level() {
    let b = ImporterBuilder::new()
        .with_component(HostComponent::new("myapp", content(1)).unwrap())
        .unwrap();
    assert_eq!(b.registry.get("myapp"), Some(&content(1)));
}

#[test]
fn component_cannot_claim_reserved_names() {
    // Rejected by name (as ReservedName, not a collision), even though
    // `std`/`ext` are not populated here.
    for reserved in ["std", "ext"] {
        let err = ImporterBuilder::new()
            .with_component(HostComponent::new(reserved, content(1)).unwrap())
            .unwrap_err();
        let HostComponentError::ReservedName(b, returned) = err else {
            panic!("expected ReservedName, got {err:?}");
        };
        assert_eq!(returned.name(), reserved);
        assert!(b.registry.is_empty());
    }
}

#[test]
fn component_name_collision_is_rejected_intact() {
    let b = ImporterBuilder::new()
        .with_component(HostComponent::new("app", content(1)).unwrap())
        .unwrap();
    let err = b
        .with_component(HostComponent::new("app", content(2)).unwrap())
        .unwrap_err();
    let HostComponentError::NameCollision(b, returned) = err else {
        panic!("expected NameCollision, got {err:?}");
    };
    assert_eq!(returned.name(), "app");
    assert_eq!(b.registry.get("app"), Some(&content(1)));
}

#[test]
fn component_rejection_recovers_via_into_parts() {
    // Either rejection hands back a usable builder and component: rename and retry.
    let err = ImporterBuilder::new()
        .with_component(HostComponent::new("std", content(1)).unwrap())
        .unwrap_err();
    let (b, returned) = err.into_parts();
    let b = b
        .with_component(returned.rename("app").unwrap())
        .expect("renamed component registers");
    assert_eq!(b.registry.get("app"), Some(&content(1)));
}

// -- Stdlib (under `std`) --

#[test]
fn stdlib_installs_modules_under_std() {
    let stdlib = Stdlib {
        modules: vec![
            stdlib_module("encoding", content(1)),
            stdlib_module("math", content(2)),
        ],
    };
    let b = ImporterBuilder::new().with_stdlib(stdlib);
    let std = std_submap(&b);
    assert_eq!(std.get_str("encoding"), Some(&content(1)));
    assert_eq!(std.get_str("math"), Some(&content(2)));
}

#[test]
fn with_stdlib_replaces_wholesale() {
    let b = ImporterBuilder::new()
        .with_stdlib(Stdlib {
            modules: vec![stdlib_module("a", content(1))],
        })
        .with_stdlib(Stdlib {
            modules: vec![stdlib_module("b", content(2))],
        });
    let std = std_submap(&b);
    assert_eq!(std.get_str("a"), None);
    assert_eq!(std.get_str("b"), Some(&content(2)));
}

// -- Namespace partitioning --

#[test]
fn ext_component_and_std_namespaces_are_independent() {
    let b = ImporterBuilder::new()
        .with_extension(Extension::new("db", content(1)).unwrap())
        .unwrap()
        .with_component(HostComponent::new("app", content(2)).unwrap())
        .unwrap()
        .with_stdlib(Stdlib {
            modules: vec![stdlib_module("io", content(3))],
        });
    assert_eq!(ext_submap(&b).get_str("db"), Some(&content(1)));
    assert_eq!(b.registry.get("app"), Some(&content(2)));
    assert_eq!(std_submap(&b).get_str("io"), Some(&content(3)));

    // An extension `app` (under `ext`) and a top-level component `app` don't
    // collide: they live in different namespaces.
    let b = b
        .with_extension(Extension::new("app", content(4)).unwrap())
        .unwrap();
    assert_eq!(ext_submap(&b).get_str("app"), Some(&content(4)));
    assert_eq!(b.registry.get("app"), Some(&content(2)));
}

// -- Name validation --

#[test]
fn invalid_names_are_rejected_carrying_the_name() {
    for bad in ["a.b", "has space", "1abc", "", "match", "def"] {
        let err = Extension::new(bad, content(1)).unwrap_err();
        assert_eq!(err.invalid_name(), bad);
        assert!(HostComponent::new(bad, content(1)).is_err());
    }
}

#[test]
fn valid_identifier_names_are_accepted() {
    for good in ["sqlite", "http_client", "_private", "foo123"] {
        assert!(Extension::new(good, content(1)).is_ok(), "{good}");
        assert!(HostComponent::new(good, content(1)).is_ok(), "{good}");
    }
}

#[test]
fn rename_validates_the_new_name() {
    let ext = Extension::new("ok", content(1)).unwrap();
    let err = ext.rename("not valid").unwrap_err();
    assert_eq!(err.invalid_name(), "not valid");
}
