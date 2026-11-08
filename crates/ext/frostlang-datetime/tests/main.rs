//! The integration tests, built as one binary: each file here is a module.
//!
//! Time zones come from the system's IANA Time Zone Database, and every case
//! uses dates whose rules are long settled.

// Shared harness.
mod common;

mod calendar;
mod components;
mod dst;
mod durations;
mod extension;
mod patterns;
mod surface;
mod text;
