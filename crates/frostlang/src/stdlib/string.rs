//! `std.string`: searching, stripping affixes, splitting into characters,
//! classifying, and padding text, and reading and writing in-memory buffers as
//! streams.
//!
//! Positions and widths count code points. The searching and stripping functions
//! and `is_empty` also take Bytes, in any mix with a String, as the global
//! `contains` does; where Bytes are involved, positions count bytes instead.

use std::io::Cursor;

use crate::native::{Flat, Optional};
use crate::stdlib::stream::{self, Buffer, Kind};
use crate::{
    FrostBytes, FrostError, FrostResult, FrostString, NativeCtx, StdlibModule, Value,
    frost_closure, frost_fn,
};

/// The `std.string` module: finding and counting substrings, stripping a prefix
/// or suffix, splitting into characters, classifying characters, padding and
/// centering text, and in-memory buffers read and written as streams.
///
/// It reaches nothing outside the script.
pub fn string() -> StdlibModule {
    StdlibModule::builder("string")
        .value("index_of", index_of("std.string.index_of", First))
        .value("last_index_of", index_of("std.string.last_index_of", Last))
        .function(count_frost())
        .function(chars_frost())
        .function(is_empty_frost())
        .function(strip_prefix_frost())
        .function(strip_suffix_frost())
        .function(is_ascii_frost())
        .value("is_digit", classifier("std.string.is_digit", is_digit))
        .value(
            "is_alpha",
            classifier("std.string.is_alpha", char::is_alphabetic),
        )
        .value(
            "is_alphanumeric",
            classifier("std.string.is_alphanumeric", char::is_alphanumeric),
        )
        .value(
            "is_whitespace",
            classifier("std.string.is_whitespace", char::is_whitespace),
        )
        .value(
            "is_uppercase",
            case_classifier("std.string.is_uppercase", char::is_uppercase),
        )
        .value(
            "is_lowercase",
            case_classifier("std.string.is_lowercase", char::is_lowercase),
        )
        .value("pad_left", padder("std.string.pad_left", Side::Left))
        .value("pad_right", padder("std.string.pad_right", Side::Right))
        .value("center", padder("std.string.center", Side::Both))
        .function(reader_frost())
        .function(writer_frost())
        .build()
}

/// `reader(content)`: a reader over a String or Bytes.
#[frost_fn]
fn reader(#[frost(unnamed)] content: Flat) -> Value {
    let content = match content {
        Flat::String(text) => FrostBytes::from(text),
        Flat::Bytes(octets) => octets,
    };
    stream::reader(Cursor::new(content), Kind::Buffer)
}

/// `writer()`: a writer that keeps what is written, for reading back.
#[frost_fn]
fn writer() -> Value {
    stream::writer(Buffer::default(), Kind::Buffer)
}

// --- Searching ---

/// Which occurrence `index_of` finds.
#[derive(Clone, Copy)]
enum Occurrence {
    First,
    Last,
}
use Occurrence::{First, Last};

/// The start of the `occurrence` of `needle` in `haystack`, by byte.
fn find_bytes(haystack: &[u8], needle: &[u8], occurrence: Occurrence) -> Option<usize> {
    if needle.len() > haystack.len() {
        return None;
    }
    let mut starts = 0..=haystack.len() - needle.len();
    let matches = |&start: &usize| &haystack[start..start + needle.len()] == needle;
    match occurrence {
        First => starts.find(matches),
        Last => starts.rev().find(matches),
    }
}

fn index_of(name: &'static str, occurrence: Occurrence) -> Value {
    frost_closure!(name, move |#[frost(unnamed)] haystack: Flat,
                               needle: Flat| {
        let index = match (&haystack, &needle) {
            (Flat::String(text), Flat::String(needle)) => {
                let byte = match occurrence {
                    First => text.find(&**needle),
                    Last => text.rfind(&**needle),
                };
                byte.map(|byte| text[..byte].chars().count())
            }
            _ => find_bytes(haystack.as_bytes(), needle.as_bytes(), occurrence),
        };
        index.map_or(Value::Null, |index| {
            Value::Int(i64::try_from(index).expect("an index fits in an Int"))
        })
    })
}

#[frost_fn]
fn count(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    #[frost(unnamed)] haystack: Flat,
    needle: Flat,
) -> FrostResult {
    let (haystack, needle) = (haystack.as_bytes(), needle.as_bytes());
    if needle.is_empty() {
        return Err(FrostError::from_string(format!(
            "Function {} requires argument 2 (needle) to be non-empty",
            ctx.name()
        )));
    }
    // Counting by byte is exact for text too: a UTF-8 needle matches only
    // at character boundaries of UTF-8 text.
    let mut count = 0;
    let mut rest = haystack;
    while let Some(start) = find_bytes(rest, needle, First) {
        count += 1;
        rest = &rest[start + needle.len()..];
    }
    Ok(Value::Int(count))
}

#[frost_fn]
fn is_empty(#[frost(unnamed)] content: Flat) -> bool {
    content.as_bytes().is_empty()
}

// --- Stripping ---

/// Removes an affix from one edge of its input, if present.
type Strip<T> = for<'a> fn(&'a T, &T) -> Option<&'a T>;

/// `text` with `affix` removed by `strip_text` or `strip_bytes`. The result is
/// a String only when both arguments are, else Bytes. A `text` left with
/// nothing removed is returned as is, though as Bytes if the affix is Bytes.
fn strip(text: Flat, affix: &Flat, strip_text: Strip<str>, strip_bytes: Strip<[u8]>) -> Value {
    match (&text, affix) {
        (Flat::String(whole), Flat::String(affix)) => match strip_text(whole, affix) {
            Some(rest) if rest.len() < whole.len() => Value::from(rest),
            _ => text.into(),
        },
        _ => {
            let bytes = text.as_bytes();
            match strip_bytes(bytes, affix.as_bytes()) {
                Some(rest) if rest.len() < bytes.len() => Value::from(rest),
                _ if matches!(text, Flat::Bytes(_)) => text.into(),
                _ => Value::from(bytes),
            }
        }
    }
}

#[frost_fn]
fn strip_prefix(#[frost(unnamed)] text: Flat, prefix: Flat) -> Value {
    strip(
        text,
        &prefix,
        |text, prefix| text.strip_prefix(prefix),
        <[u8]>::strip_prefix,
    )
}

#[frost_fn]
fn strip_suffix(#[frost(unnamed)] text: Flat, suffix: Flat) -> Value {
    strip(
        text,
        &suffix,
        |text, suffix| text.strip_suffix(suffix),
        <[u8]>::strip_suffix,
    )
}

// --- Characters ---

#[frost_fn]
fn chars(#[frost(unnamed)] text: FrostString) -> Value {
    text.chars().map(|c| Value::from(c.to_string())).collect()
}

/// `is_ascii(text)`: whether a String holds nothing outside ASCII. Unlike the
/// other classifiers, it passes the empty String, which holds nothing at all.
#[frost_fn]
fn is_ascii(#[frost(unnamed)] text: FrostString) -> bool {
    text.is_ascii()
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

/// A function testing whether a String has a character, and every character
/// passes `test`.
fn classifier(name: &'static str, test: fn(char) -> bool) -> Value {
    frost_closure!(name, move |#[frost(unnamed)] text: FrostString| {
        !text.is_empty() && text.chars().all(test)
    })
}

/// A function testing whether a String has a character with case, and every
/// such character passes `case`. A character without case is ignored. A
/// title-case character, such as `ǅ`, has case but is neither upper nor lower.
fn case_classifier(name: &'static str, case: fn(char) -> bool) -> Value {
    frost_closure!(name, move |#[frost(unnamed)] text: FrostString| {
        let mut cased = text.chars().filter(|&c| has_case(c)).peekable();
        cased.peek().is_some() && cased.all(case)
    })
}

/// Whether `c` is upper, lower, or title case.
fn has_case(c: char) -> bool {
    c.is_uppercase() || c.is_lowercase() || is_titlecase(c)
}

/// Whether `c` is title case, such as `ǅ`. The standard library has no such
/// test, but a title-case character is the one kind that is neither upper nor
/// lower case yet changes when mapped to either.
fn is_titlecase(c: char) -> bool {
    !c.is_uppercase() && !c.is_lowercase() && !c.to_uppercase().eq([c]) && !c.to_lowercase().eq([c])
}

// --- Padding ---

/// Where padding goes.
#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
    /// Split between the two, the extra character on the right.
    Both,
}

fn padder(name: &'static str, side: Side) -> Value {
    frost_closure!(name, move |#[frost(unnamed)] text: FrostString,
                               width: u64,
                               fill: Optional<char>| {
        // A width past `usize::MAX`, possible where `usize` is narrower than
        // 64 bits, is too long to make, as `usize::MAX` itself is.
        let width = usize::try_from(width).unwrap_or(usize::MAX);
        let fill = fill.0.unwrap_or(' ');

        let padding = width.saturating_sub(text.chars().count());
        if padding == 0 {
            return Ok(Value::String(text));
        }
        let (left, right) = match side {
            Side::Left => (padding, 0),
            Side::Right => (0, padding),
            Side::Both => (padding / 2, padding - padding / 2),
        };
        // Past `isize::MAX` bytes, allocating the result would panic.
        let size = padding
            .checked_mul(fill.len_utf8())
            .and_then(|filled| filled.checked_add(text.len()))
            .filter(|&size| isize::try_from(size).is_ok())
            .ok_or_else(|| {
                FrostError::from_string(format!("Function {name} cannot make a String that long"))
            })?;
        let mut padded = String::with_capacity(size);
        padded.extend(std::iter::repeat_n(fill, left));
        padded.push_str(&text);
        padded.extend(std::iter::repeat_n(fill, right));
        Ok(Value::from(padded))
    })
}
