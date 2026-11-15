//! The integration tests, built as one binary: each file here is a module.

// Shared harness.
mod common;

mod cancellation;
mod compression;
mod extension;
mod failures;
mod fields;
mod files;
mod proxy;
mod requests;
mod tls;
mod urls;
