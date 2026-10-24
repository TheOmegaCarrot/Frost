//! The runner for Frost's core-language corpus.
//!
//! The corpus, its expectation format, and how to run it are described in
//! `corpus/README.md`; the rules for maintaining it, in `corpus/MAINTENANCE.md`.
//! The `corpus` test target runs it, and the `corpus` bench target times its
//! benchmarks; this library is what those targets, and the runner's own tests,
//! build on.

mod bench;
mod discover;
mod expectation;
mod mode;
mod observe;
mod verdict;

pub use bench::{
    BENCHMARKS, Benchmark, MAX_ID_PART, OPTIONS_VARIABLE, OptionSet, option_sets, prepare,
};
pub use discover::{Entry, discover};
pub use expectation::{Bench, Expectation, Expected, ExpectedRun, Kind, KnownIssue};
pub use mode::Mode;
pub use verdict::{Report, Tally, Verdict, check, check_source};
