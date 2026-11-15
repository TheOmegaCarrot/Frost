//! The integration tests, built as one binary: each file here is a module.

mod extensions;
// Where its module doc says.
#[cfg(all(unix, not(target_os = "android"), not(target_vendor = "apple")))]
mod http_trust;
mod repl;
