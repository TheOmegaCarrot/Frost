//! One connection bundle shared by scripts running at once on several threads,
//! as a host may choose to share it.

use std::thread;

use frostlang::{HostComponent, Value};

use crate::common::{frost, host_importer, run_source, value};

const THREADS: usize = 8;
const CALLS_PER_THREAD: i64 = 200;

#[test]
fn a_bundle_shared_across_threads_counts_each_calls_own_changes() {
    let db = value(
        r"
        def db = sqlite.open_memory()
        db.script('CREATE TABLE t (x)')
        db
        ",
    );
    let importer = host_importer(HostComponent::builder("shared").value("db", db).build());
    // Each call changes 2 rows, so a count reading another call's changes,
    // made while it ran, would be more than 2.
    let source = format!(
        r"
        def db = import('shared').db
        map range({CALLS_PER_THREAD}) with fn i -> db.script('INSERT INTO t VALUES (1); INSERT INTO t VALUES (2)')
        "
    );
    let counts = thread::scope(|scope| {
        let runs: Vec<_> = (0..THREADS)
            .map(|_| scope.spawn(|| run_source(&source, importer.clone())))
            .collect();
        runs.into_iter()
            .map(|run| {
                run.join()
                    .expect("no thread panicked")
                    .unwrap_or_else(|error| panic!("each script runs, but one raised: {error}"))
            })
            .collect::<Vec<_>>()
    });
    let twos = Value::Array(vec![Value::Int(2); CALLS_PER_THREAD as usize].into());
    for count in counts {
        assert_eq!(count, twos, "every call counted only its own 2 rows");
    }
    let total = run_source("import('shared').db.script('DELETE FROM t')", importer)
        .expect("the database is still usable");
    assert_eq!(
        total,
        Value::Int(2 * CALLS_PER_THREAD * THREADS as i64),
        "every call's rows are there"
    );
}

/// The results of running `source` on each of `THREADS` threads at once, with
/// `db` shared as `import('shared').db`.
fn run_on_threads(db: Value, source: &str) -> Vec<Value> {
    let importer = host_importer(HostComponent::builder("shared").value("db", db).build());
    thread::scope(|scope| {
        let runs: Vec<_> = (0..THREADS)
            .map(|_| scope.spawn(|| run_source(source, importer.clone())))
            .collect();
        runs.into_iter()
            .map(|run| {
                run.join()
                    .expect("no thread panicked")
                    .unwrap_or_else(|error| panic!("each script runs, but one raised: {error}"))
            })
            .collect()
    })
}

#[test]
fn a_bundle_shared_across_threads_counts_each_execs_own_changes() {
    let db = value(
        r"
        def db = sqlite.open_memory()
        db.exec('CREATE TABLE t (x)')
        db
        ",
    );
    // Each call inserts 1 row, so a count reading another call's would be
    // more.
    let source = format!(
        r"
        def db = import('shared').db
        map range({CALLS_PER_THREAD}) with fn i -> db.exec('INSERT INTO t VALUES (?)', [i])
        "
    );
    let ones = Value::Array(vec![Value::Int(1); CALLS_PER_THREAD as usize].into());
    for counts in run_on_threads(db, &source) {
        assert_eq!(counts, ones, "every call counted only its own row");
    }
}

#[test]
fn a_fold_shared_across_threads_holds_the_connection_for_its_callbacks() {
    let db = value(
        r"
        def db = sqlite.open_memory()
        db.exec('CREATE TABLE t (x)')
        db.exec('INSERT INTO t VALUES (1), (2), (3)')
        db.exec('CREATE TABLE log (x)')
        db
        ",
    );
    // Each callback counts the log, from the count the fold's first one
    // found, then logs a row: no other thread's call runs on the connection
    // between a fold's own.
    let source = format!(
        r"
        def db = import('shared').db
        map range({CALLS_PER_THREAD}) with fn i -> {{
            def folded = db.fold('SELECT x FROM t', {{start: null, seen: []}}, fn acc, row -> {{
                def [count] = db.query('SELECT count(*) AS c FROM log')
                def start = acc.start or count.c
                db.exec('INSERT INTO log VALUES (?)', [row.x])
                {{start: start, seen: acc.seen + [count.c - start]}}
            }})
            folded.seen
        }}
        "
    );
    let runs = run_on_threads(db, &source);
    for run in runs {
        let Value::Array(folds) = run else {
            panic!("each script returns an Array, not {run:?}");
        };
        for fold in &folds {
            assert_eq!(
                fold,
                &frost("[0, 1, 2]"),
                "only the fold's own rows were logged meanwhile"
            );
        }
    }
}
