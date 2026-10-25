//! The integration tests, built as one binary: each file here is a module.
//!
//! `arbitrary-precision/` runs them again with `serde_json`'s
//! `arbitrary_precision` feature, which a plain `cargo test` skips; see its
//! manifest.

// Shared harness.
mod common;

mod extension;
mod json;
mod json_api;
mod msgpack;
mod msgpack_api;
mod toml;
mod toml_api;
mod uint64;
mod uint64_api;
