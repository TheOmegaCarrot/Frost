//! Splitting, joining, case conversion, and searching strings.
//!
//! The content-agnostic functions (`split`, `split_once`, `join`, `replace`,
//! `contains`, `starts_with`, `ends_with`) accept String and Bytes in any mix,
//! working on a String's UTF-8 bytes where Bytes are involved. Their result is a String only
//! when every content argument is a String, and Bytes otherwise.
//! `contains` also searches an Array, for an element equal to its second argument.
//! The text functions (`lines`, `trim*`, `to_upper`, `to_lower`) accept only a String.

use crate::native::Flat;
use crate::{
    FrostArray, FrostError, FrostString, FrostType, NativeCtx, Value, frost_closure, frost_fn,
};

/// Flat arguments resolved to one content type: text when every one is a String,
/// else bytes, with each String contributing its UTF-8.
enum Content<'a, const N: usize> {
    Text([&'a str; N]),
    Binary([&'a [u8]; N]),
}

/// Resolve `values` to their [`Content`].
fn content<const N: usize>(values: [&Flat; N]) -> Content<'_, N> {
    let texts = values.map(|value| match value {
        Flat::String(text) => Some(text.as_str()),
        Flat::Bytes(_) => None,
    });
    if texts.iter().all(Option::is_some) {
        Content::Text(texts.map(|text| text.expect("every value is a String")))
    } else {
        Content::Binary(values.map(Flat::as_bytes))
    }
}

/// The start of each non-overlapping occurrence of `needle` in `haystack`, left to
/// right. `needle` must not be empty.
fn match_starts<'a>(haystack: &'a [u8], needle: &'a [u8]) -> impl Iterator<Item = usize> + 'a {
    let mut from = 0;
    std::iter::from_fn(move || {
        let at = from
            + haystack[from..]
                .windows(needle.len())
                .position(|window| window == needle)?;
        from = at + needle.len();
        Some(at)
    })
}

/// Whether `needle` occurs in `haystack`.
fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || match_starts(haystack, needle).next().is_some()
}

/// `haystack` split on each occurrence of `delimiter`, or into single bytes when
/// `delimiter` is empty.
fn split_bytes<'a>(haystack: &'a [u8], delimiter: &[u8]) -> Vec<&'a [u8]> {
    if delimiter.is_empty() {
        return haystack.chunks(1).collect();
    }
    let mut pieces = Vec::new();
    let mut start = 0;
    for at in match_starts(haystack, delimiter) {
        pieces.push(&haystack[start..at]);
        start = at + delimiter.len();
    }
    pieces.push(&haystack[start..]);
    pieces
}

/// `text` split on each occurrence of `delimiter`, or into single characters when
/// `delimiter` is empty.
fn split_text<'a>(text: &'a str, delimiter: &str) -> Vec<&'a str> {
    if delimiter.is_empty() {
        return text
            .char_indices()
            .map(|(at, c)| &text[at..at + c.len_utf8()])
            .collect();
    }
    text.split(delimiter).collect()
}

#[frost_fn]
pub(super) fn split(#[frost(unnamed)] target: Flat, delimiter: Flat) -> Value {
    match content([&target, &delimiter]) {
        Content::Text([text, delimiter]) => split_text(text, delimiter)
            .into_iter()
            .map(Value::from)
            .collect(),
        Content::Binary([bytes, delimiter]) => split_bytes(bytes, delimiter)
            .into_iter()
            .map(Value::from)
            .collect(),
    }
}

/// `haystack` split at the first occurrence of `delimiter`, or after its first
/// byte when `delimiter` is empty: the first two pieces of [`split_bytes`],
/// with the rest kept whole. `None` when that would give fewer than two pieces.
fn split_bytes_once<'a>(haystack: &'a [u8], delimiter: &[u8]) -> Option<(&'a [u8], &'a [u8])> {
    if delimiter.is_empty() {
        return (haystack.len() > 1).then(|| haystack.split_at(1));
    }
    let at = match_starts(haystack, delimiter).next()?;
    Some((&haystack[..at], &haystack[at + delimiter.len()..]))
}

/// `text` split at the first occurrence of `delimiter`, or after its first
/// character when `delimiter` is empty: the first two pieces of [`split_text`],
/// with the rest kept whole. `None` when that would give fewer than two pieces.
fn split_text_once<'a>(text: &'a str, delimiter: &str) -> Option<(&'a str, &'a str)> {
    if delimiter.is_empty() {
        let first = text.chars().next()?;
        return (first.len_utf8() < text.len()).then(|| text.split_at(first.len_utf8()));
    }
    text.split_once(delimiter)
}

#[frost_fn]
pub(super) fn split_once(#[frost(unnamed)] target: Flat, delimiter: Flat) -> Value {
    let pieces =
        match content([&target, &delimiter]) {
            Content::Text([text, delimiter]) => split_text_once(text, delimiter)
                .map(|(before, after)| Value::array([before, after])),
            Content::Binary([bytes, delimiter]) => split_bytes_once(bytes, delimiter)
                .map(|(before, after)| Value::array([before, after])),
        };
    pieces.unwrap_or(Value::Null)
}

#[frost_fn]
pub(super) fn lines(#[frost(unnamed)] text: FrostString) -> Value {
    text.lines().map(Value::from).collect()
}

#[frost_fn]
pub(super) fn join(
    #[frost(unnamed)] elements: FrostArray,
    separator: Flat,
) -> Result<Value, FrostError> {
    if let Some(bad) = elements
        .iter()
        .find(|element| !element.fits(FrostType::FLAT))
    {
        return Err(FrostError::from_string(format!(
            "Function join requires Array of String or Bytes as argument 1, got Array containing {}",
            bad.type_name()
        )));
    }

    let texts: Option<Vec<&str>> = elements.iter().map(Value::as_str).collect();
    Ok(match (texts, &separator) {
        (Some(texts), Flat::String(separator)) => texts.join(separator).into(),
        _ => {
            let pieces: Vec<&[u8]> = elements
                .iter()
                .map(|element| element.as_byte_slice().expect("the element is Flat"))
                .collect();
            pieces.join(separator.as_bytes()).into()
        }
    })
}

#[frost_fn]
pub(super) fn replace(#[frost(unnamed)] target: Flat, find: Flat, replacement: Flat) -> Value {
    // An empty `find` matches nothing, so the target comes back unchanged
    // (though as Bytes, if the other arguments make the result Bytes).
    match content([&target, &find, &replacement]) {
        Content::Text([text, "", _]) => Value::from(text),
        Content::Text([text, find, replacement]) => text.replace(find, replacement).into(),
        Content::Binary([bytes, [], _]) => Value::from(bytes),
        Content::Binary([bytes, find, replacement]) => {
            split_bytes(bytes, find).join(replacement).into()
        }
    }
}

/// A one-String global returning `op` applied to its argument.
fn text_op(name: &'static str, op: fn(&str) -> Value) -> Value {
    frost_closure!(name, move |#[frost(unnamed)] text: FrostString| op(&text))
}

/// A one-String global trimming its argument with `trim`, which must return a
/// subslice of its input. An argument with nothing to trim is returned as is.
fn trim_op(name: &'static str, trim: fn(&str) -> &str) -> Value {
    frost_closure!(name, move |#[frost(unnamed)] text: FrostString| {
        let trimmed = trim(&text);
        if trimmed.len() == text.len() {
            Value::String(text)
        } else {
            Value::from(trimmed)
        }
    })
}

pub(super) fn trim_global() -> Value {
    trim_op("trim", str::trim)
}

pub(super) fn trim_left_global() -> Value {
    trim_op("trim_left", str::trim_start)
}

pub(super) fn trim_right_global() -> Value {
    trim_op("trim_right", str::trim_end)
}

pub(super) fn to_upper_global() -> Value {
    text_op("to_upper", |text| text.to_uppercase().into())
}

pub(super) fn to_lower_global() -> Value {
    text_op("to_lower", |text| text.to_lowercase().into())
}

/// Whether an Array has an element equal to `needle`, or String or Bytes hold
/// `needle`, also String or Bytes, as a substring.
#[frost_fn]
pub(super) fn contains(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    #[frost(unnamed, types = Flat | Array)] haystack: Value,
    needle: Value,
) -> Result<bool, FrostError> {
    if let Value::Array(array) = &haystack {
        return Ok(array.iter().any(|element| *element == needle));
    }
    let haystack = Flat::try_from(haystack).expect("type-checked as String or Bytes");
    let needle = match needle {
        Value::String(text) => Flat::String(text),
        Value::Bytes(bytes) => Flat::Bytes(bytes),
        other => {
            let searched = match haystack {
                Flat::String(_) => "a String",
                Flat::Bytes(_) => "Bytes",
            };
            return Err(FrostError::from_string(format!(
                "Function {} requires String or Bytes as argument 2 (needle) for {searched}, \
                 got {}",
                ctx.name(),
                other.type_name()
            )));
        }
    };
    Ok(match content([&haystack, &needle]) {
        Content::Text([text, needle]) => text.contains(needle),
        Content::Binary([bytes, needle]) => contains_bytes(bytes, needle),
    })
}

#[frost_fn]
pub(super) fn starts_with(#[frost(unnamed)] haystack: Flat, prefix: Flat) -> bool {
    haystack.as_bytes().starts_with(prefix.as_bytes())
}

#[frost_fn]
pub(super) fn ends_with(#[frost(unnamed)] haystack: Flat, suffix: Flat) -> bool {
    haystack.as_bytes().ends_with(suffix.as_bytes())
}
