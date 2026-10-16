//! The runner for Frost's core-language corpus.
//!
//! The corpus, its expectation format, and how to run it are described in
//! `corpus/README.md`; the rules for maintaining it, in `corpus/MAINTENANCE.md`.
//! The `corpus` test target runs it; this library is what that target, and
//! the runner's own tests, build on.

mod discover;
mod expectation;
mod mode;
mod observe;
mod verdict;

pub use discover::{Entry, discover};
pub use expectation::{Expectation, Expected, ExpectedRun, Kind, KnownIssue};
pub use mode::Mode;
pub use verdict::{Report, Tally, Verdict, check, check_source};
