//! Building a [`Stdlib`] from the [`stdlib`] module constructors, through the
//! public API alone. What each module does is tested from Frost source, in the
//! `std_*` tests.

use frost_runtime::stdlib::RandomConfig;
use frost_runtime::{Stdlib, StdlibModule, Value, stdlib};

#[test]
fn a_module_is_named_for_its_import_path() {
    assert_eq!(stdlib::encoding().name(), "encoding");
}

#[test]
fn a_module_holds_what_a_script_imports() {
    let module = stdlib::encoding();
    let Value::Map(content) = module.content() else {
        panic!("std.encoding is a Map, got {:?}", module.content());
    };
    assert!(
        content
            .get_str("hex")
            .is_some_and(|hex| matches!(hex, Value::Map(_))),
        "std.encoding holds the hex submodule"
    );
}

#[test]
fn a_module_is_added_once() {
    let stdlib = Stdlib::new()
        .with_module(stdlib::encoding())
        .expect("the first `encoding` is accepted");
    let err = stdlib
        .with_module(stdlib::encoding())
        .expect_err("a second `encoding` is rejected");
    assert_eq!(
        err.to_string(),
        "standard library module `encoding` is already present"
    );
    let (stdlib, rejected) = err.into_parts();
    assert_eq!(rejected.name(), "encoding");
    let names: Vec<&str> = stdlib.modules().iter().map(StdlibModule::name).collect();
    assert_eq!(names, ["encoding"]);
}

// --- Presets ---

/// The names of `stdlib`'s modules, sorted.
fn names(stdlib: &Stdlib) -> Vec<&str> {
    let mut names: Vec<&str> = stdlib.modules().iter().map(StdlibModule::name).collect();
    names.sort_unstable();
    names
}

#[test]
fn the_contained_preset_holds_the_contained_modules() {
    assert_eq!(
        names(&Stdlib::contained(RandomConfig::default())),
        ["encoding", "math", "random", "regex", "string"]
    );
}

#[test]
fn the_complete_preset_holds_every_module() {
    assert_eq!(
        names(&Stdlib::complete(RandomConfig::default())),
        ["encoding", "fs", "math", "os", "random", "regex", "string"]
    );
}

#[test]
fn the_complete_preset_holds_every_contained_module() {
    let complete = Stdlib::complete(RandomConfig::default());
    let complete = names(&complete);
    let contained = Stdlib::contained(RandomConfig::default());
    for name in names(&contained) {
        assert!(complete.contains(&name), "complete holds `{name}`");
    }
}

#[test]
fn a_preset_still_takes_modules_but_not_ones_it_holds() {
    let presets = [
        Stdlib::contained(RandomConfig::default()),
        Stdlib::complete(RandomConfig::default()),
    ];
    for preset in presets {
        let err = preset
            .with_module(stdlib::encoding())
            .expect_err("a preset already holds `encoding`");
        assert_eq!(
            err.to_string(),
            "standard library module `encoding` is already present"
        );
    }
}
