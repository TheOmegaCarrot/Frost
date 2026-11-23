//! The integration tests, built as one binary: each file here is a module.

// Shared harness.
mod common;

mod clap_rules;
mod commands;
mod extension;
mod generated_specs;
mod hidden_and_epilog;
mod outcomes;
mod parse;
mod parsing;
mod spec;
mod strange;
mod surface;
