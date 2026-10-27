//! Places within a value, written as Frost indexes them.

use std::fmt;

use crate::core::util::identifier::is_identifier_like_and_not_keyword;
use crate::core::{MapKey, Value};

/// One step into a value: a Map entry or an Array element.
#[derive(Debug)]
pub(crate) enum PathSegment {
    Key(MapKey),
    Index(usize),
}

/// Writes `segments`, outermost first, as Frost would index along them from the
/// top, such as `name.inner`, `items[2]`, or `["my-key"]`. No segments write
/// nothing.
pub(crate) fn write_path<'a>(
    f: &mut impl fmt::Write,
    segments: impl IntoIterator<Item = &'a PathSegment>,
) -> fmt::Result {
    for (i, segment) in segments.into_iter().enumerate() {
        match segment {
            PathSegment::Key(MapKey::String(name)) if is_identifier_like_and_not_keyword(name) => {
                if i > 0 {
                    f.write_str(".")?;
                }
                f.write_str(name)?;
            }
            PathSegment::Key(key) => {
                write!(f, "[{}]", Value::from(key.clone()).to_debug_string())?;
            }
            PathSegment::Index(index) => write!(f, "[{index}]")?,
        }
    }
    Ok(())
}
