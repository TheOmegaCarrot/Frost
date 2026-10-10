//! One resolver serving several threads: an import never waits for another
//! thread, and a load that panics leaves the resolver usable.
//!
//! Each concurrent case runs under a watchdog, so a hang fails the test rather
//! than stalling the suite.

use crate::common;

use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, mpsc};
use std::thread;
use std::time::Duration;

use common::{Run, Tree, count, run};
use frostlang::{Arity, HostComponent, Importer, ImporterBuilder, Value};

/// Far longer than any case takes when it does not hang.
const WATCHDOG: Duration = Duration::from_secs(30);

/// An importer serving `tree`'s root, with a host component `host` holding
/// `natives`.
fn importer_with(tree: &Tree, natives: Value) -> Arc<Importer> {
    ImporterBuilder::new()
        .with_component(HostComponent::new("host", natives).expect("a valid name"))
        .expect("an unclaimed name")
        .append_resolver(Arc::new(tree.resolver(&["."])))
        .build()
}

/// Run `source` on each of `threads` threads at once, returning every run, or
/// panicking if they are not all done within the watchdog's time.
fn run_concurrently(source: &'static str, importer: &Arc<Importer>, threads: usize) -> Vec<Run> {
    let (done, finished) = mpsc::channel();
    for _ in 0..threads {
        let (importer, done) = (Arc::clone(importer), done.clone());
        thread::spawn(move || done.send(run(source, &importer)).unwrap());
    }
    (0..threads)
        .map(|_| {
            finished
                .recv_timeout(WATCHDOG)
                .expect("every thread finishes: an import must never wait on another")
        })
        .collect()
}

#[test]
fn threads_loading_one_module_at_once_do_not_wait_for_each_other() {
    // The module's top level waits until both threads are running it. If either
    // import waited for the other's load, neither would finish.
    let tree = Tree::new("concurrency/no_waiting");
    tree.file(
        "m.frst",
        r"
        import('host').meet()
        print('ran m')
        export defn f() -> 'm'
        ",
    );
    let barrier = Arc::new(Barrier::new(2));
    let meet = Value::native("meet", Arity::Exact(0), move |_, _| {
        barrier.wait();
        Ok(Value::Null)
    });
    let importer = importer_with(&tree, Value::map([("meet", meet)]));

    let runs = run_concurrently("import('m')", &importer, 2);
    for run in &runs {
        assert_eq!(
            count(&run.printed, "ran m"),
            1,
            "each thread ran the module"
        );
    }
    let exports: Vec<Value> = runs.into_iter().map(Run::value).collect();
    assert_eq!(
        exports[0], exports[1],
        "both receive the exports of whichever load finished first"
    );
    let later = run("import('m')", &importer);
    assert_eq!(count(&later.printed, "ran m"), 0, "and that load is cached");
    assert_eq!(later.value(), exports[0]);
}

#[test]
fn many_threads_importing_a_module_all_receive_the_same_exports() {
    let tree = Tree::new("concurrency/many");
    tree.file("m.frst", "export defn f() -> 'm'")
        .file("chain.frst", "export def f = import('m').f");
    let importer = importer_with(&tree, Value::map::<&str, 0>([]));
    let runs = run_concurrently("[import('m').f, import('chain').f]", &importer, 8);
    let first = run("import('m').f", &importer).value();
    for run in runs {
        assert_eq!(run.value(), Value::array([first.clone(), first.clone()]));
    }
}

#[test]
fn a_load_that_panics_leaves_the_resolver_usable() {
    // `m`'s top level panics the first time it runs. The panic unwinds out of the
    // import; importing `m` again on the same thread must load it afresh, not
    // find the abandoned load still in progress and call it a cycle.
    let tree = Tree::new("concurrency/panic");
    tree.file(
        "m.frst",
        r"
        import('host').explode()
        export def ok = true
        ",
    );
    let exploded = AtomicBool::new(false);
    let explode = Value::native("explode", Arity::Exact(0), move |_, _| {
        if !exploded.swap(true, Ordering::SeqCst) {
            panic!("a native panicked while `m` was loading");
        }
        Ok(Value::Null)
    });
    let importer = importer_with(&tree, Value::map([("explode", explode)]));

    let (done, finished) = mpsc::channel();
    let thread_importer = Arc::clone(&importer);
    thread::spawn(move || {
        let panicked =
            panic::catch_unwind(AssertUnwindSafe(|| run("import('m')", &thread_importer))).is_err();
        let retried = run("import('m').ok", &thread_importer).outcome;
        done.send((panicked, retried)).unwrap();
    });
    let (panicked, retried) = finished
        .recv_timeout(WATCHDOG)
        .expect("the thread finishes");
    assert!(panicked, "the first import panics");
    assert_eq!(retried.expect("the retry imports `m`"), Value::Bool(true));
    assert_eq!(
        run("import('m').ok", &importer).value(),
        Value::Bool(true),
        "another thread imports it too"
    );
}
