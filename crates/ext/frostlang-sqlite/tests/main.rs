//! The integration tests, built as one binary: each file here is a module.

// Shared harness.
mod common;

mod authorizer;
mod bindings;
mod cancellation;
mod close;
mod exec;
mod extension;
mod factory;
mod filter;
mod open;
mod query;
mod reduce;
mod script;
mod sharing;
mod transaction;
mod transaction_statements;
mod values;
