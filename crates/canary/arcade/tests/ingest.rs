//! The ingester: what it loads, what a change reloads, and what each module
//! remembers. Every test but the watching ones refreshes by hand, so none
//! depends on the timing of filesystem events.

use std::fs;
use std::num::NonZeroU64;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{ImporterBuilder, Stdlib, Value, Vm, VmRuntimeConfiguration, stdlib::StdlibConfig};
use frostlang_canary_arcade::ingest::{Ingester, LoadError, Module, Registry};

/// A fresh, empty directory for one test's module files, named for the test.
struct Tree {
    root: PathBuf,
}

impl Tree {
    fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("frostlang-canary-arcade-tests")
            .join(name);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("the scratch directory is creatable");
        Self { root }
    }

    /// Write `contents` to the file at `relative`, creating its directories.
    fn file(&self, relative: &str, contents: impl AsRef<[u8]>) -> &Self {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("a file has a parent"))
            .expect("the file's directory is creatable");
        fs::write(&path, contents).expect("the file is writable");
        self
    }

    fn remove(&self, relative: &str) -> &Self {
        fs::remove_file(self.root.join(relative)).expect("the file is removable");
        self
    }

    /// An ingester of this tree that rescans only when refreshed.
    fn ingester(&self) -> Ingester {
        Ingester::builder(&self.root)
            .with_watching(false)
            .start()
            .expect("the tree is a directory")
    }
}

/// The module `name`, which must be in `registry`.
fn module<'a>(registry: &'a Registry, name: &str) -> &'a Module {
    registry
        .get(name)
        .unwrap_or_else(|| panic!("the registry has no module {name}"))
}

/// The export `export` of `name`'s latest load, which must have succeeded.
fn export(registry: &Registry, name: &str, export: &str) -> Value {
    let exports = module(registry, name)
        .outcome()
        .unwrap_or_else(|error| panic!("{name} failed to load: {error}"));
    exported(exports, name, export)
}

/// The export `export` from `exports`, a module's exports Map.
fn exported(exports: &Value, name: &str, export: &str) -> Value {
    let Value::Map(exports) = exports else {
        panic!("{name}'s exports are a Map, got {exports:?}");
    };
    exports
        .get_str(export)
        .unwrap_or_else(|| panic!("{name} exports no {export}"))
        .clone()
}

/// The message of `name`'s failed latest load.
fn failure(registry: &Registry, name: &str) -> String {
    match module(registry, name).outcome() {
        Ok(exports) => panic!("{name} loaded, exporting {exports:?}, but was expected to fail"),
        Err(error) => error.to_string(),
    }
}

fn names(registry: &Registry) -> Vec<&str> {
    registry.modules().map(Module::name).collect()
}

#[test]
fn starting_loads_every_module_under_the_root() {
    let tree = Tree::new("starting_loads_every_module_under_the_root");
    tree.file("top.frst", "export def value = 1")
        .file("nested/deeper/inner.frst", "export def value = 2")
        .file("notes.txt", "not a module")
        .file("has-dash.frst", "export def value = 3")
        .file("bad-dir/inside.frst", "export def value = 4")
        .file("3rd.frst", "export def value = 5");
    let ingester = tree.ingester();
    let registry = ingester.registry();
    assert_eq!(
        names(&registry),
        ["nested.deeper.inner", "top"],
        "only files whose path `import` can spell are modules"
    );
    assert_eq!(export(&registry, "top", "value"), Value::from(1));
    assert_eq!(
        export(&registry, "nested.deeper.inner", "value"),
        Value::from(2)
    );
    assert_eq!(registry.generation(), 1, "the first build is generation 1");
}

#[test]
fn an_empty_root_still_publishes_its_first_build() {
    let tree = Tree::new("an_empty_root_still_publishes_its_first_build");
    let registry = tree.ingester().registry();
    assert_eq!(names(&registry), Vec::<&str>::new());
    assert_eq!(
        registry.generation(),
        0,
        "with no module to load, nothing changed from the empty registry"
    );
}

#[test]
fn a_root_that_is_not_a_directory_does_not_start() {
    let tree = Tree::new("a_root_that_is_not_a_directory_does_not_start");
    tree.file("file.frst", "1");
    let error = Ingester::builder(tree.root.join("file.frst"))
        .with_watching(false)
        .start()
        .expect_err("a file is not a root");
    assert!(
        error.to_string().contains("as the root"),
        "the error names the root's role: {error}"
    );
    let error = Ingester::builder(tree.root.join("missing"))
        .with_watching(false)
        .start()
        .expect_err("a missing directory is not a root");
    assert!(
        error.to_string().contains("missing"),
        "the error names the root: {error}"
    );
}

#[test]
fn a_module_that_does_not_compile_records_its_diagnostics() {
    let tree = Tree::new("a_module_that_does_not_compile_records_its_diagnostics");
    tree.file("broken.frst", "export def = ");
    let registry = tree.ingester().registry();
    let broken = module(&registry, "broken");
    assert!(
        matches!(broken.outcome(), Err(LoadError::Compile(_))),
        "a parse error is a compile failure: {:?}",
        broken.outcome()
    );
    assert!(broken.last_good().is_none(), "no version has loaded yet");
    assert!(
        failure(&registry, "broken").starts_with("does not compile"),
        "the message says what failed"
    );
}

#[test]
fn a_module_whose_top_level_fails_records_the_error() {
    let tree = Tree::new("a_module_whose_top_level_fails_records_the_error");
    tree.file("fails.frst", "export def value = 1 + 'one'");
    let registry = tree.ingester().registry();
    assert!(
        matches!(module(&registry, "fails").outcome(), Err(LoadError::Run(_))),
        "a runtime error is a run failure"
    );
}

#[test]
fn a_file_that_is_not_text_is_unreadable() {
    let tree = Tree::new("a_file_that_is_not_text_is_unreadable");
    tree.file("binary.frst", [0xff, 0xfe, 0x00]);
    let registry = tree.ingester().registry();
    assert!(
        matches!(
            module(&registry, "binary").outcome(),
            Err(LoadError::Unreadable(_))
        ),
        "invalid UTF-8 cannot be read as source: {:?}",
        module(&registry, "binary").outcome()
    );
}

#[test]
fn a_module_records_its_lints() {
    let tree = Tree::new("a_module_records_its_lints");
    tree.file(
        "linted.frst",
        r"
defn f() -> {
    def unused = 1
    2
}
export def value = f()
",
    )
    .file("clean.frst", "export def value = 1");
    let registry = tree.ingester().registry();
    assert_eq!(
        module(&registry, "linted").lints().len(),
        1,
        "one unused binding"
    );
    assert!(module(&registry, "clean").lints().is_empty());
}

#[test]
fn refreshing_reloads_a_changed_module_and_keeps_the_rest() {
    let tree = Tree::new("refreshing_reloads_a_changed_module_and_keeps_the_rest");
    tree.file("changes.frst", "export def value = 1")
        .file("stays.frst", "export def value = 1");
    let ingester = tree.ingester();
    let before = ingester.registry();
    tree.file("changes.frst", "export def value = 2");
    let after = ingester.refresh();
    assert_eq!(after.generation(), before.generation() + 1);
    assert_eq!(export(&after, "changes", "value"), Value::from(2));
    assert_eq!(
        module(&after, "changes").generation(),
        after.generation(),
        "the changed module was reloaded in the new generation"
    );
    assert_eq!(
        module(&after, "stays").generation(),
        module(&before, "stays").generation(),
        "an unchanged module is carried over, not reloaded"
    );
    assert_eq!(
        export(&before, "changes", "value"),
        Value::from(1),
        "an earlier registry never changes"
    );
}

#[test]
fn refreshing_with_nothing_changed_publishes_nothing_new() {
    let tree = Tree::new("refreshing_with_nothing_changed_publishes_nothing_new");
    tree.file("module.frst", "export def value = 1");
    let ingester = tree.ingester();
    let before = ingester.registry();
    // Rewriting the same text is no change either.
    tree.file("module.frst", "export def value = 1");
    let after = ingester.refresh();
    assert!(
        Arc::ptr_eq(&before, &after),
        "the same registry is still the latest"
    );
}

#[test]
fn a_failed_reload_keeps_the_last_good_exports() {
    let tree = Tree::new("a_failed_reload_keeps_the_last_good_exports");
    tree.file("module.frst", "export def value = 1");
    let ingester = tree.ingester();
    tree.file("module.frst", "export def value = ");
    let broken = ingester.refresh();
    let module_now = module(&broken, "module");
    assert!(
        module_now.outcome().is_err(),
        "the new version does not compile"
    );
    let last_good = module_now.last_good().expect("the first version loaded");
    assert_eq!(exported(last_good, "module", "value"), Value::from(1));

    tree.file("module.frst", "export def value = 3");
    let fixed = ingester.refresh();
    assert_eq!(export(&fixed, "module", "value"), Value::from(3));
    assert_eq!(
        exported(
            module(&fixed, "module").last_good().unwrap(),
            "module",
            "value"
        ),
        Value::from(3),
        "a successful load is the new last good version"
    );
}

#[test]
fn a_failed_compile_keeps_the_lints_of_the_last_version_that_compiled() {
    let tree = Tree::new("a_failed_compile_keeps_the_lints_of_the_last_version_that_compiled");
    tree.file(
        "module.frst",
        r"
defn f() -> {
    def unused = 1
    2
}
export def value = f()
",
    );
    let ingester = tree.ingester();
    tree.file("module.frst", "export def value = ");
    let registry = ingester.refresh();
    assert_eq!(module(&registry, "module").lints().len(), 1);
}

#[test]
fn modules_import_one_another_and_record_it() {
    let tree = Tree::new("modules_import_one_another_and_record_it");
    tree.file("lib/util.frst", "export defn double(x) -> x * 2")
        .file(
            "main.frst",
            r"
def util = import('lib.util')
export def value = util.double(21)
",
        );
    let registry = tree.ingester().registry();
    assert_eq!(export(&registry, "main", "value"), Value::from(42));
    assert_eq!(
        module(&registry, "main").imports().collect::<Vec<_>>(),
        ["lib.util"]
    );
    assert_eq!(module(&registry, "lib.util").imports().count(), 0);
}

#[test]
fn changing_a_module_reloads_everything_that_imports_it() {
    let tree = Tree::new("changing_a_module_reloads_everything_that_imports_it");
    tree.file("base.frst", "export def value = 1")
        .file(
            "middle.frst",
            "export def value = import('base').value * 10",
        )
        .file("top.frst", "export def value = import('middle').value + 1")
        .file("unrelated.frst", "export def value = 0");
    let ingester = tree.ingester();
    let before = ingester.registry();
    assert_eq!(export(&before, "top", "value"), Value::from(11));

    tree.file("base.frst", "export def value = 2");
    let after = ingester.refresh();
    assert_eq!(
        export(&after, "top", "value"),
        Value::from(21),
        "the change reaches through every importer"
    );
    for name in ["base", "middle", "top"] {
        assert_eq!(
            module(&after, name).generation(),
            after.generation(),
            "{name} reloaded"
        );
    }
    assert_eq!(
        module(&after, "unrelated").generation(),
        module(&before, "unrelated").generation(),
        "a module importing nothing that changed is not reloaded"
    );
}

#[test]
fn each_module_runs_once_per_build_however_often_it_is_imported() {
    let tree = Tree::new("each_module_runs_once_per_build_however_often_it_is_imported");
    // A seeded engine is shared state: if `shared` ran twice, the two importers
    // would see different engines, each starting the sequence afresh.
    tree.file(
        "shared.frst",
        "export def rng = import('std.random').seed(7)",
    )
    .file(
        "a.frst",
        "export def draw = import('shared').rng.int(0, 1000000)",
    )
    .file(
        "b.frst",
        "export def draw = import('shared').rng.int(0, 1000000)",
    );
    let ingester = Ingester::builder(&tree.root)
        .with_watching(false)
        .with_importer(
            ImporterBuilder::new().with_stdlib(Stdlib::contained(StdlibConfig::default())),
        )
        .start()
        .unwrap();
    let registry = ingester.registry();
    assert_ne!(
        export(&registry, "a", "draw"),
        export(&registry, "b", "draw"),
        "both importers drew from one engine, so from one run of the module"
    );
}

#[test]
fn a_module_importing_a_failed_module_fails_too() {
    let tree = Tree::new("a_module_importing_a_failed_module_fails_too");
    tree.file("base.frst", "export def value = 1")
        .file("top.frst", "export def value = import('base').value");
    let ingester = tree.ingester();
    tree.file("base.frst", "export def value = ");
    let registry = ingester.refresh();
    let message = failure(&registry, "top");
    assert!(
        message.contains("Cannot import 'base'"),
        "the importer's failure names the module it could not import: {message}"
    );
    assert_eq!(
        exported(
            module(&registry, "top").last_good().unwrap(),
            "top",
            "value"
        ),
        Value::from(1),
        "the importer keeps its last good version too"
    );
}

#[test]
fn a_module_that_appears_satisfies_an_import_that_failed_for_its_absence() {
    let tree = Tree::new("a_module_that_appears_satisfies_an_import_that_failed_for_its_absence");
    tree.file("top.frst", "export def value = import('later').value");
    let ingester = tree.ingester();
    let before = ingester.registry();
    assert!(
        module(&before, "top").outcome().is_err(),
        "'later' does not exist yet"
    );
    assert_eq!(
        module(&before, "top").imports().collect::<Vec<_>>(),
        ["later"],
        "the missing import is recorded"
    );

    tree.file("later.frst", "export def value = 5");
    let after = ingester.refresh();
    assert_eq!(export(&after, "top", "value"), Value::from(5));
}

#[test]
fn removing_a_module_drops_it_and_fails_its_importers() {
    let tree = Tree::new("removing_a_module_drops_it_and_fails_its_importers");
    tree.file("base.frst", "export def value = 1")
        .file("top.frst", "export def value = import('base').value");
    let ingester = tree.ingester();
    tree.remove("base.frst");
    let registry = ingester.refresh();
    assert_eq!(names(&registry), ["top"]);
    assert!(module(&registry, "top").outcome().is_err());
}

#[test]
fn an_import_cycle_fails_every_module_in_it() {
    let tree = Tree::new("an_import_cycle_fails_every_module_in_it");
    tree.file("a.frst", "export def value = import('b').value")
        .file("b.frst", "export def value = import('a').value");
    let registry = tree.ingester().registry();
    for name in ["a", "b"] {
        let message = failure(&registry, name);
        assert!(
            message.contains("Import cycle"),
            "{name} fails for the cycle: {message}"
        );
    }
    assert!(
        failure(&registry, "a").contains("a imports b imports a"),
        "the cycle is named in import order: {}",
        failure(&registry, "a")
    );
}

#[test]
fn modules_import_what_the_host_grants() {
    let tree = Tree::new("modules_import_what_the_host_grants");
    tree.file(
        "module.frst",
        "export def value = import('std.math').abs(-3)",
    );
    let without = tree.ingester();
    assert!(
        module(&without.registry(), "module").outcome().is_err(),
        "by default, nothing but the other modules is importable"
    );
    drop(without);
    let with = Ingester::builder(&tree.root)
        .with_watching(false)
        .with_importer(
            ImporterBuilder::new().with_stdlib(Stdlib::contained(StdlibConfig::default())),
        )
        .start()
        .unwrap();
    assert_eq!(export(&with.registry(), "module", "value"), Value::from(3));
}

#[test]
fn modules_run_under_the_configured_limits() {
    let tree = Tree::new("modules_run_under_the_configured_limits");
    tree.file(
        "runaway.frst",
        r"
defn forever(n) -> forever(n + 1)
export def value = forever(0)
",
    );
    let ingester = Ingester::builder(&tree.root)
        .with_watching(false)
        .with_configuration(VmRuntimeConfiguration::default().with_fuel(NonZeroU64::new(1000)))
        .start()
        .unwrap();
    assert!(
        module(&ingester.registry(), "runaway").outcome().is_err(),
        "the fuel limit stops the runaway top level"
    );
}

#[test]
fn modules_compile_with_the_configured_options() {
    let tree = Tree::new("modules_compile_with_the_configured_options");
    tree.file("module.frst", "def value = 1");
    let ingester = Ingester::builder(&tree.root)
        .with_watching(false)
        .with_compiler_options(CompilerOptions::new().with_implicit_export(true))
        .start()
        .unwrap();
    assert_eq!(
        export(&ingester.registry(), "module", "value"),
        Value::from(1)
    );
}

/// The tail value of `source`, run on a host Vm with `ingester`'s importer, or
/// its error's message.
fn run_on_host(ingester: &Ingester, source: &str) -> Result<Value, String> {
    let closure = compile_program("host.frst", source, CompilerOptions::new())
        .expect("the host script compiles")
        .code
        .into_closure()
        .unwrap();
    Vm::factory()
        .with_importer(ingester.importer())
        .build(closure)
        .run()
        .map(|result| result.tail().clone())
        .map_err(|error| error.into_error().to_string())
}

#[test]
fn the_importer_serves_host_vms_the_latest_registry() {
    let tree = Tree::new("the_importer_serves_host_vms_the_latest_registry");
    tree.file("module.frst", "export def value = 1");
    let ingester = tree.ingester();
    let run = |source: &str| run_on_host(&ingester, source);
    assert_eq!(run("import('module').value"), Ok(Value::from(1)));

    tree.file("module.frst", "export def value = 2");
    ingester.refresh();
    assert_eq!(
        run("import('module').value"),
        Ok(Value::from(2)),
        "a later import sees the latest registry"
    );

    tree.file("module.frst", "export def value = ");
    ingester.refresh();
    let error = run("import('module').value").expect_err("the latest load failed");
    assert!(error.contains("Cannot import 'module'"), "{error}");

    let error = run("import('missing')").expect_err("no such module");
    assert!(
        !error.contains("Cannot import 'missing'"),
        "a module that does not exist is left to the importer to report: {error}"
    );
}

#[test]
fn host_vms_and_modules_share_one_set_of_capabilities() {
    let tree = Tree::new("host_vms_and_modules_share_one_set_of_capabilities");
    tree.file("module.frst", "export def rng = import('std.random').rng");
    let ingester = Ingester::builder(&tree.root)
        .with_watching(false)
        .with_importer(
            ImporterBuilder::new().with_stdlib(Stdlib::contained(StdlibConfig::default())),
        )
        .start()
        .unwrap();
    assert_eq!(
        run_on_host(
            &ingester,
            "import('std.random').rng == import('module').rng"
        ),
        Ok(Value::from(true)),
        "the host and the module import the same `std.random`, so the same engine"
    );
}

#[test]
fn watching_reloads_a_changed_file_without_a_refresh() {
    let tree = Tree::new("watching_reloads_a_changed_file_without_a_refresh");
    tree.file("module.frst", "export def value = 1");
    let ingester = Ingester::builder(&tree.root).start().unwrap();
    let before = ingester.registry();
    tree.file("module.frst", "export def value = 2");
    let after = ingester.wait_past(before.generation(), Duration::from_secs(10));
    assert_eq!(export(&after, "module", "value"), Value::from(2));
}

#[test]
fn watching_finds_a_module_in_a_new_directory() {
    let tree = Tree::new("watching_finds_a_module_in_a_new_directory");
    let ingester = Ingester::builder(&tree.root).start().unwrap();
    let before = ingester.registry();
    tree.file("fresh/module.frst", "export def value = 1");
    let after = ingester.wait_past(before.generation(), Duration::from_secs(10));
    assert_eq!(export(&after, "fresh.module", "value"), Value::from(1));
}
