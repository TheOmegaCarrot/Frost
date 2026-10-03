//! Building a [`Stdlib`] from the [`stdlib`] module constructors, through the
//! public API alone. What each module does is tested from Frost source, in the
//! `std_*` tests.

mod source;

use std::sync::Arc;

use frost_runtime::stdlib::RandomConfig;
use frost_runtime::{Importer, ImporterBuilder, Stdlib, StdlibModule, Value, stdlib};
use source::Script;

/// An importer providing only `stdlib`.
fn importer(stdlib: Stdlib) -> Arc<Importer> {
    ImporterBuilder::new().with_stdlib(stdlib).build()
}

/// Whether `import('std.<module>')` resolves, under an importer of `stdlib`.
fn resolves(stdlib: Stdlib, module: &str) -> bool {
    Script::new(&format!("is_map(import('std.{module}'))"))
        .importer(importer(stdlib))
        .outcome()
        .is_ok()
}

#[test]
fn a_module_is_named_for_its_import_path() {
    assert_eq!(stdlib::encoding().name(), "encoding");
}

#[test]
fn a_module_holds_what_a_script_imports() {
    let module = stdlib::encoding();
    let imported = Script::new(
        r"
        [
            sorted(keys(content)) == sorted(keys(import('std.encoding'))),
            is_map(content.hex),
        ]
        ",
    )
    .capture("content", module.content().clone())
    .importer(importer(
        Stdlib::new()
            .with_module(stdlib::encoding())
            .expect("a lone module is accepted"),
    ))
    .run();
    assert_eq!(
        imported,
        Value::array([Value::Bool(true), Value::Bool(true)]),
        "the content is what `import('std.encoding')` gives, hex submodule included"
    );
}

#[test]
fn a_new_stdlib_holds_no_modules() {
    assert!(Stdlib::new().modules().is_empty());
    assert!(Stdlib::default().modules().is_empty());
    assert!(!resolves(Stdlib::new(), "math"), "nothing is installed");
}

#[test]
fn modules_are_added_in_order_and_installed_under_std() {
    let stdlib = Stdlib::new()
        .with_module(stdlib::math())
        .expect("math is accepted")
        .with_module(stdlib::encoding())
        .expect("encoding is accepted");
    let names: Vec<&str> = stdlib.modules().iter().map(StdlibModule::name).collect();
    assert_eq!(names, ["math", "encoding"]);
    assert!(resolves(stdlib, "math"));
}

#[test]
fn installing_a_stdlib_replaces_any_installed_before() {
    let math = Stdlib::new().with_module(stdlib::math()).expect("math");
    let encoding = Stdlib::new()
        .with_module(stdlib::encoding())
        .expect("encoding");
    let importer = ImporterBuilder::new()
        .with_stdlib(math)
        .with_stdlib(encoding)
        .build();
    let raised = Script::new("import('std.math')")
        .importer(Arc::clone(&importer))
        .raises();
    assert_eq!(raised, "Could not resolve import 'std.math'");
    let encoding = Script::new("is_map(import('std.encoding'))")
        .importer(importer)
        .run();
    assert_eq!(encoding, Value::Bool(true));
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
    // The library is still usable: it takes another module.
    let stdlib = stdlib
        .with_module(stdlib::math())
        .expect("a rejection leaves the library able to take new modules");
    assert!(resolves(stdlib, "math"));
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
fn a_preset_still_takes_modules_but_not_ones_it_holds() {
    let contained = Stdlib::contained(RandomConfig::default())
        .with_module(stdlib::fs())
        .expect("`contained` does not hold `fs`");
    assert_eq!(
        names(&contained),
        ["encoding", "fs", "math", "random", "regex", "string"]
    );

    let presets = [
        Stdlib::contained(RandomConfig::default()),
        Stdlib::complete(RandomConfig::default()),
    ];
    for preset in presets {
        let held = names(&preset).len();
        let err = preset
            .with_module(stdlib::encoding())
            .expect_err("a preset already holds `encoding`");
        assert_eq!(
            err.to_string(),
            "standard library module `encoding` is already present"
        );
        let (preset, rejected) = err.into_parts();
        assert_eq!(rejected.name(), "encoding");
        assert_eq!(names(&preset).len(), held, "the preset comes back intact");
        assert!(names(&preset).contains(&"encoding"));
    }
}
