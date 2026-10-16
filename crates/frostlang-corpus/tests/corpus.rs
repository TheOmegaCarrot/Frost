//! Runs the corpus: one trial per script, then a summary of the verdicts.
//! See `corpus/README.md`.

use std::num::NonZeroUsize;
use std::path::Path;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::thread;

use frostlang_corpus::{Mode, Tally, check, discover};
use libtest_mimic::{Arguments, Trial};
use rayon::ThreadPoolBuilder;

fn main() -> ExitCode {
    let mut arguments = Arguments::from_args();
    let threads = arguments
        .test_threads
        .or_else(threads_from_env)
        .unwrap_or_else(|| thread::available_parallelism().map_or(1, NonZeroUsize::get));
    arguments.test_threads = Some(threads);
    // Every check runs on this pool, its option sets spread across the
    // workers, so the scripts and their option sets share one thread budget;
    // the harness's own threads only wait.
    let pool = Arc::new(
        ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("the thread pool starts"),
    );
    let mode = Mode::from_env();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let entries = discover(&root).expect("the corpus is readable");
    let tally = Arc::new(Mutex::new(Tally::new(mode)));
    let trials = entries
        .into_iter()
        .map(|entry| {
            let tally = Arc::clone(&tally);
            let pool = Arc::clone(&pool);
            Trial::test(entry.name.clone(), move || {
                let verdict = pool.install(|| check(&entry, mode));
                tally.lock().unwrap().record(&entry.name, &verdict);
                match verdict.report.failure() {
                    Some(reason) => Err(reason.into()),
                    None => Ok(()),
                }
            })
        })
        .collect();
    let conclusion = libtest_mimic::run(&arguments, trials);
    if !arguments.list {
        println!("{}\n", tally.lock().unwrap());
    }
    conclusion.exit_code()
}

/// The thread count `RUST_TEST_THREADS` asks for, as libtest reads it;
/// libtest-mimic reads only `--test-threads`.
fn threads_from_env() -> Option<usize> {
    let value = std::env::var("RUST_TEST_THREADS").ok()?;
    let threads = value
        .parse::<NonZeroUsize>()
        .unwrap_or_else(|_| panic!("RUST_TEST_THREADS is `{value}`, should be a positive integer"));
    Some(threads.get())
}
