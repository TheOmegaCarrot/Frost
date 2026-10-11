//! White-box tests for `#[frost_fn]` used within this crate, as the stdlib
//! would: its generated paths start `::frostlang`, which resolve here only
//! through the crate's `extern crate self as frostlang`. Its behavior is
//! covered black-box in `tests/native_frost_fn.rs`.

use crate::{Value, frost_fn};

#[frost_fn(name = "double")]
fn double(n: i64) -> i64 {
    n * 2
}

#[test]
fn frost_fn_works_within_this_crate() {
    let Value::NativeFunction(function) = double_frost() else {
        panic!("the maker returns a native function");
    };
    assert_eq!(function.name(), "double");
}
