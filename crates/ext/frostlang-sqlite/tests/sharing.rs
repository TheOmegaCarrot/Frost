//! One connection bundle shared by scripts running at once on several threads,
//! as a host may choose to share it.

use std::thread;

use frostlang::{HostComponent, Value};

use crate::common::{host_importer, run_source, value};

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
