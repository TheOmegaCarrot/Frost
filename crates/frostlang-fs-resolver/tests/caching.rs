//! With caching, a module runs once and every import shares its exports; without,
//! every import runs it afresh. Each module here prints when it runs, so the
//! printed lines count its runs.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use common::{Tree, count, importer, run};
use frostlang_runtime::{Arity, HostComponent, ImporterBuilder, Value};

/// A module that prints `ran <name>` when it runs, and exports a function, whose
/// identity tells one run's exports from another's.
fn counted(name: &str) -> String {
    format!(
        r"
        print('ran {name}')
        export defn f() -> '{name}'
        "
    )
}

#[test]
fn a_cached_module_runs_once_however_often_it_is_imported() {
    let tree = Tree::new("caching/once");
    tree.file("m.frst", counted("m"));
    let importer = importer(tree.resolver(&["."]));
    let run = run(
        r"
        def first = import('m')
        def second = import('m')
        first.f == second.f
        ",
        &importer,
    );
    assert_eq!(count(&run.printed, "ran m"), 1, "{:?}", run.printed);
    assert_eq!(
        run.value(),
        Value::Bool(true),
        "both imports share one run's exports"
    );
}

#[test]
fn the_cache_outlives_a_run() {
    // One resolver serves both scripts, so the second finds the module cached.
    let tree = Tree::new("caching/across_runs");
    tree.file("m.frst", counted("m"));
    let importer = importer(tree.resolver(&["."]));
    let first = run("import('m')", &importer);
    let second = run("import('m')", &importer);
    assert_eq!(count(&first.printed, "ran m"), 1);
    assert_eq!(
        count(&second.printed, "ran m"),
        0,
        "cached by the first run"
    );
}

#[test]
fn a_diamond_runs_its_shared_module_once() {
    let tree = Tree::new("caching/diamond");
    tree.file("left.frst", "export def base = import('base').f")
        .file("right.frst", "export def base = import('base').f")
        .file("base.frst", counted("base"));
    let importer = importer(tree.resolver(&["."]));
    let run = run("import('left').base == import('right').base", &importer);
    assert_eq!(count(&run.printed, "ran base"), 1, "{:?}", run.printed);
    assert_eq!(run.value(), Value::Bool(true));
}

#[test]
fn one_file_by_two_specifications_is_one_module() {
    // The roots overlap: `pkg.m` from the first and `m` from the second are the
    // same file.
    let tree = Tree::new("caching/two_specifications");
    tree.file("pkg/m.frst", counted("m"));
    let importer = importer(tree.resolver(&[".", "pkg"]));
    let run = run("import('pkg.m').f == import('m').f", &importer);
    assert_eq!(count(&run.printed, "ran m"), 1, "{:?}", run.printed);
    assert_eq!(run.value(), Value::Bool(true));
}

#[test]
fn without_caching_every_import_runs_the_module() {
    let tree = Tree::new("caching/off");
    tree.file("m.frst", counted("m"));
    let importer = importer(tree.resolver(&["."]).with_caching(false));
    let run = run("import('m').f == import('m').f", &importer);
    assert_eq!(count(&run.printed, "ran m"), 2, "{:?}", run.printed);
    assert_eq!(
        run.value(),
        Value::Bool(false),
        "each run's exports are its own"
    );
}

#[test]
fn a_module_that_fails_is_cached_once_it_succeeds() {
    // `flaky` fails until the host's `switch` is on.
    let tree = Tree::new("caching/failure");
    tree.file(
        "flaky.frst",
        r"
        print('ran flaky')
        export def ok = 1
        if import('switch').on(): null else: error('not yet')
        ",
    );
    let on = Arc::new(AtomicBool::new(false));
    let switch = {
        let on = Arc::clone(&on);
        Value::native("on", Arity::Exact(0), move |_, _| {
            Ok(Value::Bool(on.load(Ordering::SeqCst)))
        })
    };
    let importer = ImporterBuilder::new()
        .with_component(
            HostComponent::new("switch", Value::map([("on", switch)])).expect("a valid name"),
        )
        .expect("an unclaimed name")
        .append_resolver(Arc::new(tree.resolver(&["."])))
        .build();

    let failed = run("try_call(import, ['flaky']).ok", &importer);
    assert_eq!(failed.value(), Value::Bool(false));
    on.store(true, Ordering::SeqCst);
    let succeeded = run("import('flaky').ok", &importer);
    assert_eq!(
        count(&succeeded.printed, "ran flaky"),
        1,
        "the failure was not cached"
    );
    assert_eq!(succeeded.value(), Value::Int(1));
    let cached = run("import('flaky').ok", &importer);
    assert_eq!(
        count(&cached.printed, "ran flaky"),
        0,
        "the success was cached"
    );
}

#[test]
fn a_failed_cached_import_runs_again_when_retried() {
    let tree = Tree::new("caching/retry");
    tree.file(
        "bad.frst",
        r"
        print('ran bad')
        error('boom')
        ",
    );
    let importer = importer(tree.resolver(&["."]));
    let run = run(
        r"
        def first = try_call(import, ['bad'])
        def second = try_call(import, ['bad'])
        [first.error, second.error]
        ",
        &importer,
    );
    assert_eq!(count(&run.printed, "ran bad"), 2, "{:?}", run.printed);
    assert_eq!(
        run.value(),
        Value::array(["boom", "boom"]),
        "the retry fails the same way, not as a cycle"
    );
}
