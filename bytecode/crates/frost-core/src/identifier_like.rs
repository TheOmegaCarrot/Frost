const RESERVED_KEYWORDS: &[&str] = &[
    "and", "as", "def", "defn", "do", "elif", "else", "export", "false", "filter", "fn", "foreach",
    "if", "init", "is", "map", "match", "not", "null", "or", "reduce", "true", "with",
];

/// Determines if a string is a Frost keyword.
pub fn is_reserved_keyword(s: &str) -> bool {
    RESERVED_KEYWORDS.contains(&s)
}

/// Determines if a string follows Frost identifier rules.
/// This function is unaware of keywords.
pub fn is_identifier_like(s: &str) -> bool {
    let mut chars = s.chars();

    let Some(first) = chars.next() else {
        return false;
    };

    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }

    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Determines if a string follows Frost identifier rules, rejecting Frost keywords.
pub fn is_identifier_like_and_not_keyword(s: &str) -> bool {
    is_identifier_like(s) && !is_reserved_keyword(s)
}
