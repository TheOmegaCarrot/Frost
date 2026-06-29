//! Printing to stdout and message formatting.

use crate::Value;

pub(super) fn print_global() -> Value {
    super::stub("print")
}

pub(super) fn mformat_global() -> Value {
    super::stub("mformat")
}

pub(super) fn mprint_global() -> Value {
    super::stub("mprint")
}
