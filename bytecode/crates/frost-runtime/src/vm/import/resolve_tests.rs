//! White-box tests for `Importer::import`: registry resolution,
//! and the resolver chain consulted for whatever the registry does not claim.
//!
//! Most of these build an importer with no resolvers, so the miss cases test the
//! permanent contract (a registry miss with nowhere to fall back to is an error)
//! rather than any particular resolver's behavior.

use std::sync::{Arc, Mutex};

use crate::{FrostError, Value};

use super::*;

/// A distinct sentinel leaf.
fn leaf(n: i64) -> Value {
    Value::from(n)
}

/// A top-level import context: no importing module, default child factory.
/// Registry resolution ignores it entirely; the chain tests pass it through.
fn ctx() -> ImportCtx<'static> {
    ImportCtx::new(VmFactory::default(), None)
}

/// A resolver that claims exactly one spec, recording every spec it is offered,
/// so a test can prove what the chain did and did not consult.
#[derive(Debug)]
struct Claims {
    spec: &'static str,
    value: Value,
    seen: Mutex<Vec<String>>,
}

impl Claims {
    fn new(spec: &'static str, value: Value) -> Arc<Self> {
        Arc::new(Self {
            spec,
            value,
            seen: Mutex::new(Vec::new()),
        })
    }

    fn seen(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }
}

impl ImportResolver for Claims {
    fn resolve(&self, _ctx: &ImportCtx, module_spec: &str) -> Result<Option<Value>, FrostError> {
        self.seen.lock().unwrap().push(module_spec.to_string());
        if module_spec == self.spec {
            Ok(Some(self.value.clone()))
        } else {
            Ok(None)
        }
    }
}

/// A resolver that claims every spec by failing: proves an error ends the chain.
#[derive(Debug)]
struct Fails;

impl ImportResolver for Fails {
    fn resolve(&self, _ctx: &ImportCtx, module_spec: &str) -> Result<Option<Value>, FrostError> {
        Err(FrostError::from_string(format!("kerplooie: {module_spec}")))
    }
}

fn stdlib_module(name: &str, content: Value) -> StdlibModule {
    StdlibModule(Module {
        name: name.to_string(),
        content,
    })
}

/// An importer with a populated dummy registry and no resolvers.
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

// -- Registry hits --

#[test]
fn imports_the_whole_ext_namespace() {
    let v = importer().import("ext", &ctx()).unwrap();
    assert!(v.as_map().unwrap().get_str("sqlite").is_some());
}

#[test]
fn imports_an_extension() {
    let v = importer().import("ext.sqlite", &ctx()).unwrap();
    assert_eq!(v.as_map().unwrap().get_str("open"), Some(&leaf(1)));
}

#[test]
fn imports_a_fine_grained_extension_leaf() {
    let v = importer().import("ext.sqlite.open", &ctx()).unwrap();
    assert_eq!(v, leaf(1));
}

#[test]
fn imports_a_stdlib_module_and_leaf() {
    assert_eq!(
        importer()
            .import("std.math", &ctx())
            .unwrap()
            .as_map()
            .unwrap()
            .get_str("pi"),
        Some(&leaf(4))
    );
    assert_eq!(importer().import("std.math.pi", &ctx()).unwrap(), leaf(4));
}

#[test]
fn imports_a_host_component_and_leaf() {
    assert!(
        importer()
            .import("myapp", &ctx())
            .unwrap()
            .as_map()
            .unwrap()
            .get_str("run")
            .is_some()
    );
    assert_eq!(importer().import("myapp.run", &ctx()).unwrap(), leaf(3));
}

// -- Registry-exclusive errors: a top-level hit never reaches a resolver --

#[test]
fn deep_miss_under_a_hit_is_an_error() {
    // The first segment resolves, but a later one does not.
    // This is a final error, never a fallback to the resolver chain.
    let imp = importer();
    assert!(imp.import("ext.nonexistent", &ctx()).is_err());
    assert!(imp.import("ext.sqlite.nonexistent", &ctx()).is_err());
    assert!(imp.import("std.nonexistent", &ctx()).is_err());
}

#[test]
fn descending_into_a_non_module_is_a_distinct_error() {
    // `ext.leaf` is a non-Map value; a further segment cannot descend.
    let err = importer().import("ext.leaf.deeper", &ctx()).unwrap_err();
    assert!(err.message().contains("not a module"), "{}", err.message());
}

// -- Misses with nothing to fall back to --

#[test]
fn unresolved_top_level_name_errors_without_a_filesystem() {
    // The first segment matches nothing in the registry, and no resolver is
    // registered, so there is nowhere to fall back to.
    let imp = importer();
    assert!(imp.import("nonexistent", &ctx()).is_err());
    assert!(imp.import("nonexistent.module", &ctx()).is_err());
}

#[test]
fn empty_spec_is_an_error() {
    assert!(importer().import("", &ctx()).is_err());
}

// -- The resolver chain: consulted only for what the registry does not claim --

#[test]
fn a_resolver_serves_what_the_registry_misses() {
    let imp = ImporterBuilder::new()
        .append_resolver(Claims::new("dynamic", leaf(7)))
        .build();
    assert_eq!(imp.import("dynamic", &ctx()).unwrap(), leaf(7));
}

#[test]
fn the_registry_wins_and_resolvers_never_see_the_spec() {
    // A top-level registry hit is exclusive: `std`, `ext`, and host components
    // cannot be shadowed by a resolver, which is what makes registration a
    // capability grant rather than a suggestion.
    let claims = Claims::new("myapp", leaf(99));
    let imp = ImporterBuilder::new()
        .with_component(HostComponent::new("myapp", leaf(3)).unwrap())
        .unwrap()
        .append_resolver(claims.clone())
        .build();
    assert_eq!(imp.import("myapp", &ctx()).unwrap(), leaf(3));
    assert!(claims.seen().is_empty(), "resolver was consulted");
}

#[test]
fn resolvers_are_tried_in_registration_order() {
    let first = Claims::new("shared", leaf(1));
    let second = Claims::new("shared", leaf(2));
    let imp = ImporterBuilder::new()
        .append_resolver(first.clone())
        .append_resolver(second.clone())
        .build();
    assert_eq!(imp.import("shared", &ctx()).unwrap(), leaf(1));
    // The first claimed it, so the second was never asked.
    assert!(second.seen().is_empty());
}

#[test]
fn a_declining_resolver_falls_through_to_the_next() {
    let declines = Claims::new("other", leaf(1));
    let claims = Claims::new("wanted", leaf(2));
    let imp = ImporterBuilder::new()
        .append_resolver(declines.clone())
        .append_resolver(claims)
        .build();
    assert_eq!(imp.import("wanted", &ctx()).unwrap(), leaf(2));
    assert_eq!(declines.seen(), vec!["wanted".to_string()]);
}

#[test]
fn a_resolver_error_ends_the_chain() {
    // `Err` means claimed-but-failed, so no later resolver gets a turn even
    // though this one would have served the spec.
    let later = Claims::new("boom", leaf(1));
    let imp = ImporterBuilder::new()
        .append_resolver(Arc::new(Fails))
        .append_resolver(later.clone())
        .build();
    let err = imp.import("boom", &ctx()).unwrap_err();
    assert!(err.message().contains("kerplooie"), "{}", err.message());
    assert!(later.seen().is_empty(), "chain continued past an error");
}

#[test]
fn a_spec_no_resolver_claims_is_an_error() {
    let imp = ImporterBuilder::new()
        .append_resolver(Claims::new("other", leaf(1)))
        .build();
    let err = imp.import("unclaimed", &ctx()).unwrap_err();
    assert!(
        err.message().contains("Could not resolve"),
        "{}",
        err.message()
    );
}

#[test]
fn resolvers_receive_the_whole_dotted_spec() {
    // The chain gets the specification verbatim; splitting it is registry
    // behavior, not something a resolver should have to reproduce.
    let claims = Claims::new("a.b.c", leaf(5));
    let imp = ImporterBuilder::new()
        .append_resolver(claims.clone())
        .build();
    assert_eq!(imp.import("a.b.c", &ctx()).unwrap(), leaf(5));
    assert_eq!(claims.seen(), vec!["a.b.c".to_string()]);
}
