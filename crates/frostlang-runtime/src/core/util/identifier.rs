/// Every word Frost reserves.
/// A reserved word can never serve as an identifier, so a name matching one of
/// these is rejected wherever Frost source must be able to refer to it.
pub const KEYWORDS: &[&str] = &[
    "and", "as", "def", "defn", "do", "elif", "else", "export", "false", "filter", "fn", "foreach",
    "if", "init", "is", "map", "match", "not", "null", "or", "reduce", "true", "with",
];

/// Whether `s` is one of Frost's reserved [`KEYWORDS`].
pub fn is_reserved_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

/// Whether `s` has the shape of a Frost identifier: an ASCII letter or `_`,
/// followed by any number of ASCII letters, digits, or `_`.
/// The empty string does not.
///
/// Shape only. A reserved word such as `if` is identifier-shaped and answers
/// `true` here; [`is_identifier_like_and_not_keyword`] is the check for a name
/// that must actually be usable.
pub fn is_identifier_like(s: &str) -> bool {
    // Identifier rules are ASCII-only, so bytes are enough: any multi-byte
    // character fails the ASCII tests below whichever way it is inspected.
    let mut bytes = s.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };

    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }

    bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Whether `s` may be used as a Frost identifier: identifier-shaped, and not
/// reserved.
///
/// The check to apply to a name that Frost source will refer to, such as an
/// [`Extension`](crate::Extension) or [`HostComponent`](crate::HostComponent)
/// name.
pub fn is_identifier_like_and_not_keyword(s: &str) -> bool {
    is_identifier_like(s) && !is_reserved_keyword(s)
}
