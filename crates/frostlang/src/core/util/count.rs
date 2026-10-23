//! Counts in messages.

use std::fmt::Display;

/// `count` beside `noun`, singular for a count of one and plural otherwise:
/// `1 argument`, `2 arguments`. `noun` must form its plural by adding `s`.
pub(crate) fn counted<N: Display + PartialEq + From<u8>>(count: N, noun: &str) -> String {
    if count == N::from(1) {
        format!("{count} {noun}")
    } else {
        format!("{count} {noun}s")
    }
}
