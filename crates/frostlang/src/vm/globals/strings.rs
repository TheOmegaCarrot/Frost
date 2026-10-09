//! Splitting, joining, case conversion, and searching strings.
//!
//! The content-agnostic functions (`split`, `split_once`, `join`, `replace`,
//! `contains`, `starts_with`, `ends_with`, `strip_prefix`, `strip_suffix`)
//! accept String and Bytes in any mix, working on a
//! String's UTF-8 bytes where Bytes are involved. Their result is a String only
//! when every content argument is a String, and Bytes otherwise.
//! The text functions (`lines`, `trim*`, `to_upper`, `to_lower`) accept only a String.

use crate::{FrostError, FrostType, Param, Params, Value};

const ONE_STRING: Params = Params::new(&[Param::of(FrostType::STRING)]);

/// Flat arguments resolved to one content type: text when every one is a String,
/// else bytes, with each String contributing its UTF-8.
enum Content<'a, const N: usize> {
    Text([&'a str; N]),
    Binary([&'a [u8]; N]),
}

/// Resolve `values`, which must all be Flat, to their [`Content`].
fn content<const N: usize>(values: [&Value; N]) -> Content<'_, N> {
    if values.iter().all(|value| matches!(value, Value::String(_))) {
        Content::Text(values.map(|value| value.as_str().expect("every value is a String")))
    } else {
        Content::Binary(values.map(|value| value.as_byte_slice().expect("the value is Flat")))
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

pub(super) fn split_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::FLAT),
        Param::of(FrostType::FLAT).named("delimiter"),
    ]);
    Value::checked_native("split", PARAMS, |_, args| {
        Ok(match content([&args[0], &args[1]]) {
            Content::Text([text, delimiter]) => split_text(text, delimiter)
                .into_iter()
                .map(Value::from)
                .collect(),
            Content::Binary([bytes, delimiter]) => split_bytes(bytes, delimiter)
                .into_iter()
                .map(Value::from)
                .collect(),
        })
    })
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

pub(super) fn split_once_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::FLAT),
        Param::of(FrostType::FLAT).named("delimiter"),
    ]);
    Value::checked_native("split_once", PARAMS, |_, args| {
        let pieces = match content([&args[0], &args[1]]) {
            Content::Text([text, delimiter]) => split_text_once(text, delimiter)
                .map(|(before, after)| Value::array([before, after])),
            Content::Binary([bytes, delimiter]) => split_bytes_once(bytes, delimiter)
                .map(|(before, after)| Value::array([before, after])),
        };
        Ok(pieces.unwrap_or(Value::Null))
    })
}

pub(super) fn lines_global() -> Value {
    Value::checked_native("lines", ONE_STRING, |_, args| {
        let text = args[0].as_str().expect("the argument is a String");
        Ok(text.lines().map(Value::from).collect())
    })
}

pub(super) fn join_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::ARRAY),
        Param::of(FrostType::FLAT).named("separator"),
    ]);
    Value::checked_native("join", PARAMS, |_, args| {
        let (Value::Array(elements), separator) = (&args[0], &args[1]) else {
            unreachable!()
        };
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
        Ok(match (texts, separator.as_str()) {
            (Some(texts), Some(separator)) => texts.join(separator).into(),
            _ => {
                let pieces: Vec<&[u8]> = elements
                    .iter()
                    .map(|element| element.as_byte_slice().expect("the element is Flat"))
                    .collect();
                let separator = separator.as_byte_slice().expect("the separator is Flat");
                pieces.join(separator).into()
            }
        })
    })
}

pub(super) fn replace_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::FLAT),
        Param::of(FrostType::FLAT).named("find"),
        Param::of(FrostType::FLAT).named("replacement"),
    ]);
    Value::checked_native("replace", PARAMS, |_, args| {
        // An empty `find` matches nothing, so the target comes back unchanged
        // (though as Bytes, if the other arguments make the result Bytes).
        Ok(match content([&args[0], &args[1], &args[2]]) {
            Content::Text([text, "", _]) => Value::from(text),
            Content::Text([text, find, replacement]) => text.replace(find, replacement).into(),
            Content::Binary([bytes, [], _]) => Value::from(bytes),
            Content::Binary([bytes, find, replacement]) => {
                split_bytes(bytes, find).join(replacement).into()
            }
        })
    })
}

/// A one-String global returning `op` applied to its argument.
fn text_op(name: &'static str, op: fn(&str) -> Value) -> Value {
    Value::checked_native(name, ONE_STRING, move |_, args| {
        Ok(op(args[0].as_str().expect("the argument is a String")))
    })
}

/// A one-String global trimming its argument with `trim`, which must return a
/// subslice of its input. An argument with nothing to trim is returned as is.
fn trim_op(name: &'static str, trim: fn(&str) -> &str) -> Value {
    Value::checked_native(name, ONE_STRING, move |_, args| {
        let text = args[0].as_str().expect("the argument is a String");
        let trimmed = trim(text);
        Ok(if trimmed.len() == text.len() {
            args[0].take()
        } else {
            Value::from(trimmed)
        })
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

/// The spec shared by the two-Flat-argument searches.
const HAYSTACK_AND_NEEDLE: Params =
    Params::new(&[Param::of(FrostType::FLAT), Param::of(FrostType::FLAT)]);

/// A search global answering `found(haystack, needle)` over the arguments' bytes.
fn search(name: &'static str, found: fn(&[u8], &[u8]) -> bool) -> Value {
    Value::checked_native(name, HAYSTACK_AND_NEEDLE, move |_, args| {
        let [haystack, needle] =
            [&args[0], &args[1]].map(|value| value.as_byte_slice().expect("the value is Flat"));
        Ok(Value::Bool(found(haystack, needle)))
    })
}

pub(super) fn contains_global() -> Value {
    Value::native("contains", HAYSTACK_AND_NEEDLE.arity(), |ctx, args| {
        if let Value::Array(_) = args[0] {
            return Err(FrostError::from_static(
                "Function contains requires String or Bytes as argument 1, got Array \
                 (use includes to search an Array)",
            ));
        }
        ctx.check_args(args, &HAYSTACK_AND_NEEDLE)?;
        Ok(Value::Bool(match content([&args[0], &args[1]]) {
            Content::Text([text, needle]) => text.contains(needle),
            Content::Binary([bytes, needle]) => contains_bytes(bytes, needle),
        }))
    })
}

pub(super) fn starts_with_global() -> Value {
    search("starts_with", <[u8]>::starts_with)
}

pub(super) fn ends_with_global() -> Value {
    search("ends_with", <[u8]>::ends_with)
}

/// Removes an affix from one edge of its input, if present.
type Strip<T> = for<'a> fn(&'a T, &T) -> Option<&'a T>;

/// A global removing an affix, its second argument, from its first argument with
/// `strip_text` or `strip_bytes`. An argument left with nothing removed is
/// returned as is, though as Bytes if the affix is Bytes.
fn strip_op(
    name: &'static str,
    params: Params,
    strip_text: Strip<str>,
    strip_bytes: Strip<[u8]>,
) -> Value {
    Value::checked_native(name, params, move |_, args| {
        Ok(match content([&args[0], &args[1]]) {
            Content::Text([text, affix]) => match strip_text(text, affix) {
                Some(rest) if rest.len() < text.len() => Value::from(rest),
                _ => args[0].take(),
            },
            Content::Binary([bytes, affix]) => match strip_bytes(bytes, affix) {
                Some(rest) if rest.len() < bytes.len() => Value::from(rest),
                _ if matches!(args[0], Value::Bytes(_)) => args[0].take(),
                _ => Value::from(bytes),
            },
        })
    })
}

pub(super) fn strip_prefix_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::FLAT),
        Param::of(FrostType::FLAT).named("prefix"),
    ]);
    strip_op(
        "strip_prefix",
        PARAMS,
        |text, prefix| text.strip_prefix(prefix),
        <[u8]>::strip_prefix,
    )
}

pub(super) fn strip_suffix_global() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::FLAT),
        Param::of(FrostType::FLAT).named("suffix"),
    ]);
    strip_op(
        "strip_suffix",
        PARAMS,
        |text, suffix| text.strip_suffix(suffix),
        <[u8]>::strip_suffix,
    )
}
