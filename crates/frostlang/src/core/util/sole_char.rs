//! The one code point of a String holding exactly one.

/// The code point `text` consists of, or `None` if it holds none or several.
pub(crate) fn sole_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}
