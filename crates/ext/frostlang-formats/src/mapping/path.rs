//! Where a value sits within the value being encoded, for errors.

use std::fmt;

use frostlang::{MapKey, is_identifier_like_and_not_keyword};

use crate::mapping::key_source;

/// The keys and indices leading from the value being encoded to one within
/// it. Each level borrows its parent, so building a path allocates nothing;
/// only an error renders it.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Path<'a> {
    /// The value being encoded.
    Root,
    /// The value under a key of the Map at the parent path.
    Key(&'a Path<'a>, &'a MapKey),
    /// The element at an index of the Array at the parent path.
    Index(&'a Path<'a>, usize),
}

/// The path as Frost would index it: `name.inner`, `items[2]`, `["my-key"]`.
/// The root renders as nothing.
impl fmt::Display for Path<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Path::Root => Ok(()),
            Path::Index(parent, index) => write!(f, "{parent}[{index}]"),
            Path::Key(parent, MapKey::String(name)) if is_identifier_like_and_not_keyword(name) => {
                let dot = if matches!(parent, Path::Root) {
                    ""
                } else {
                    "."
                };
                write!(f, "{parent}{dot}{name}")
            }
            Path::Key(parent, key) => write!(f, "{parent}[{}]", key_source(key)),
        }
    }
}
