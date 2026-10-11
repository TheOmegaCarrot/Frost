//! The integration tests, built as one binary: each file here is a module.
//!
//! They are compile tests, run with trybuild: each case in a `pass` directory
//! must compile and run, and each in a `fail` directory must fail to compile
//! with exactly the errors in its `.stderr` file. The macros' behavior is
//! covered from Frost source in `frostlang`'s tests.
//!
//! A case without a `.stderr` file fails, writing what it got to `wip/`. Run
//! with `TRYBUILD=overwrite` to write each `.stderr` file in place, then review
//! the diff; do so too when a new Rust release rewords a rustc error.

mod frost_bundle;
mod frost_closure;
mod frost_fn;
