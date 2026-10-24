//! Times the corpus's benchmarks: each script marked `bench`, compiled and run
//! under each selected option set. See `corpus/README.md`.

use std::hint::black_box;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion};
use frostlang_corpus::{OPTIONS_VARIABLE, option_sets, prepare};

fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let prepared = option_sets(std::env::var(OPTIONS_VARIABLE).ok().as_deref())
        .and_then(|sets| Ok((prepare(&root, &sets)?, sets)));
    let (benchmarks, sets) = match prepared {
        Ok(prepared) => prepared,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };

    // Shorter than criterion's defaults; its own flags, such as
    // `--measurement-time`, override these.
    let mut criterion = Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .configure_from_args();
    for benchmark in &benchmarks {
        // Each ID part stays within criterion's directory-name limit, which
        // `prepare` checks: `<directory>/<file>/<compile|run>/<options>`.
        let mut group = criterion.benchmark_group(benchmark.group());
        for set in &sets {
            let id =
                |phase: &str| BenchmarkId::new(benchmark.file(), format!("{phase}/{}", set.label));
            group.bench_function(id("compile"), |bencher| {
                bencher.iter(|| black_box(benchmark.compile(set.options)));
            });
            let closure = benchmark
                .closure(set.options)
                .expect("`prepare` compiled every benchmark");
            group.bench_function(id("run"), |bencher| {
                bencher.iter(|| black_box(benchmark.run(&closure)));
            });
        }
        group.finish();
    }
    criterion.final_summary();
    ExitCode::SUCCESS
}
