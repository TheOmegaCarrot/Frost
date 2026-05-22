macro_rules! define_keywords {
    ($($kw:literal),+ $(,)?) => {
        /// All Frost reserved keywords, as str slices.
        pub const KEYWORDS: &[&str] = &[$($kw),+];
        /// All Frost reserved keywords, as byte strings.
        const RESERVED_KEYWORDS: &[&[u8]] = &[$($kw.as_bytes()),+];
    };
}

define_keywords![
    "and", "as", "def", "defn", "do", "elif", "else", "export", "false", "filter", "fn",
    "foreach", "if", "init", "is", "map", "match", "not", "null", "or", "reduce",
    "true", "with",
];

/// Determines if a byte string is a Frost keyword.
pub fn is_reserved_keyword(s: &[u8]) -> bool {
    RESERVED_KEYWORDS.contains(&s)
}

/// Determines if a byte string follows Frost identifier rules.
/// Identifier rules are ASCII-only, so this works on raw bytes.
/// This function is unaware of keywords.
pub fn is_identifier_like(s: &[u8]) -> bool {
    let Some((&first, rest)) = s.split_first() else {
        return false;
    };

    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }

    rest.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_')
}

/// Determines if a byte string follows Frost identifier rules, rejecting Frost keywords.
pub fn is_identifier_like_and_not_keyword(s: &[u8]) -> bool {
    is_identifier_like(s) && !is_reserved_keyword(s)
}
