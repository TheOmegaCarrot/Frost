const RESERVED_KEYWORDS: &[&[u8]] = &[
    b"and", b"as", b"def", b"defn", b"do", b"elif", b"else", b"export", b"false", b"filter", b"fn",
    b"foreach", b"if", b"init", b"is", b"map", b"match", b"not", b"null", b"or", b"reduce",
    b"true", b"with",
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
