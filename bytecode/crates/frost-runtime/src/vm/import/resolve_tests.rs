//! White-box tests for `Importer::import` (registry resolution).
//!
//! The importer here is built with *no* filesystem path, so the miss cases test
//! the permanent contract ("registry miss with no filesystem configured is an
//! error"), not the current placeholder. When filesystem resolution lands, these
//! stay valid: a top-level hit is registry-exclusive regardless, and a miss with
//! nowhere to fall back to is still an error.

use std::sync::Arc;

use crate::Value;

use super::*;

/// A distinct sentinel leaf.
fn leaf(n: i64) -> Value {
    Value::from(n)
}

fn stdlib_module(name: &str, content: Value) -> StdlibModule {
    StdlibModule(Module {
        name: name.to_string(),
        content,
    })
}

/// An importer with a populated dummy registry and no filesystem path.
/// Registry shape:
///   ext.sqlite      = { open: 1, close: 2 }
///   ext.leaf        = 9              (a non-Map leaf)
///   std.math        = { pi: 4 }
///   myapp           = { run: 3 }
fn importer() -> Arc<Importer> {
    ImporterBuilder::new()
        .with_extension(
            Extension::new(
                "sqlite",
                Value::map([("open", leaf(1)), ("close", leaf(2))]),
            )
            .unwrap(),
        )
        .unwrap()
        .with_extension(Extension::new("leaf", leaf(9)).unwrap())
        .unwrap()
        .with_component(HostComponent::new("myapp", Value::map([("run", leaf(3))])).unwrap())
        .unwrap()
        .with_stdlib(Stdlib {
            modules: vec![stdlib_module("math", Value::map([("pi", leaf(4))]))],
        })
        .build()
}

// -- Registry hits (stable regardless of the filesystem) --

#[test]
fn imports_the_whole_ext_namespace() {
    let v = importer().import("ext").unwrap();
    assert!(v.as_map().unwrap().get_str("sqlite").is_some());
}

#[test]
fn imports_an_extension() {
    let v = importer().import("ext.sqlite").unwrap();
    assert_eq!(v.as_map().unwrap().get_str("open"), Some(&leaf(1)));
}

#[test]
fn imports_a_fine_grained_extension_leaf() {
    let v = importer().import("ext.sqlite.open").unwrap();
    assert_eq!(v, leaf(1));
}

#[test]
fn imports_a_stdlib_module_and_leaf() {
    assert_eq!(
        importer()
            .import("std.math")
            .unwrap()
            .as_map()
            .unwrap()
            .get_str("pi"),
        Some(&leaf(4))
    );
    assert_eq!(importer().import("std.math.pi").unwrap(), leaf(4));
}

#[test]
fn imports_a_host_component_and_leaf() {
    assert!(
        importer()
            .import("myapp")
            .unwrap()
            .as_map()
            .unwrap()
            .get_str("run")
            .is_some()
    );
    assert_eq!(importer().import("myapp.run").unwrap(), leaf(3));
}

// -- Registry-exclusive errors: a top-level hit never falls to the filesystem --

#[test]
fn deep_miss_under_a_hit_is_an_error() {
    // The first segment resolves, but a later one does not. This is a final
    // error, never a filesystem lookup, so it holds once the filesystem exists.
    let imp = importer();
    assert!(imp.import("ext.nonexistent").is_err());
    assert!(imp.import("ext.sqlite.nonexistent").is_err());
    assert!(imp.import("std.nonexistent").is_err());
}

#[test]
fn descending_into_a_non_module_is_a_distinct_error() {
    // `ext.leaf` is a non-Map value; a further segment cannot descend.
    let err = importer().import("ext.leaf.deeper").unwrap_err();
    assert!(err.message().contains("not a module"), "{}", err.message());
}

// -- Misses with no filesystem configured (the permanent no-fallback contract) --

#[test]
fn unresolved_top_level_name_errors_without_a_filesystem() {
    // The first segment matches nothing in the registry. With no filesystem
    // path configured, there is nowhere to fall back to, so this is an error
    // both now and once filesystem resolution exists.
    let imp = importer();
    assert!(imp.import("nonexistent").is_err());
    assert!(imp.import("nonexistent.module").is_err());
}

#[test]
fn empty_spec_is_an_error() {
    assert!(importer().import("").is_err());
}
