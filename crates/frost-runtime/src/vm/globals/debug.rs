//! Assertions and value inspection.

use crate::Value;

pub(super) fn assert_global() -> Value {
    super::stub("assert")
}

pub(super) fn debug_dump_global() -> Value {
    super::stub("debug_dump")
}
