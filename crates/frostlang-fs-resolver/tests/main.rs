//! The integration tests, built as one binary: each file here is a module.

// Shared harness.
mod common;

mod caching;
mod concurrency;
mod cycles;
mod errors;
mod permissions;
mod resolution;
mod symlinks;
