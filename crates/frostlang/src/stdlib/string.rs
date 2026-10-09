//! `std.string`: searching, stripping affixes, splitting into characters,
//! classifying, and padding text, and reading and writing in-memory buffers as
//! streams.
//!
//! Positions and widths count code points. The searching and stripping functions
//! and `is_empty` also take Bytes, in any mix with a String, as the global
//! `contains` does; where Bytes are involved, positions count bytes instead.

use std::io::Cursor;

use crate::stdlib::stream::{self, Buffer, Kind};
use crate::{Arity, FrostBytes, FrostError, FrostType, Param, Params, StdlibModule, Value};

/// The `std.string` module: finding and counting substrings, stripping a prefix
/// or suffix, splitting into characters, classifying characters, padding and
/// centering text, and in-memory buffers read and written as streams.
///
/// It reaches nothing outside the script.
pub fn string() -> StdlibModule {
    StdlibModule::new(
        "string",
        Value::map([
            ("index_of", index_of("string.index_of", First)),
            ("last_index_of", index_of("string.last_index_of", Last)),
            ("count", count()),
            ("chars", chars()),
            ("is_empty", is_empty()),
            ("strip_prefix", strip_prefix()),
            ("strip_suffix", strip_suffix()),
            ("is_ascii", is_ascii()),
            ("is_digit", classifier("string.is_digit", is_digit)),
            (
                "is_alpha",
                classifier("string.is_alpha", char::is_alphabetic),
            ),
            (
                "is_alphanumeric",
                classifier("string.is_alphanumeric", char::is_alphanumeric),
            ),
            (
                "is_whitespace",
                classifier("string.is_whitespace", char::is_whitespace),
            ),
            (
                "is_uppercase",
                case_classifier("string.is_uppercase", char::is_uppercase),
            ),
            (
                "is_lowercase",
                case_classifier("string.is_lowercase", char::is_lowercase),
            ),
            ("pad_left", padder("string.pad_left", Side::Left)),
            ("pad_right", padder("string.pad_right", Side::Right)),
            ("center", padder("string.center", Side::Both)),
            ("reader", buffer_reader()),
            ("writer", buffer_writer()),
        ]),
    )
}

/// `reader(content)`: a reader over a String or Bytes.
fn buffer_reader() -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::FLAT)]);
    Value::checked_native("string.reader", PARAMS, |_, args| {
        let content = match args[0].take() {
            Value::String(text) => FrostBytes::from(text),
            Value::Bytes(octets) => octets,
            other => unreachable!("type-checked as Flat, got {}", other.type_name()),
        };
        Ok(stream::reader(Cursor::new(content), Kind::Buffer))
    })
}

/// `writer()`: a writer that keeps what is written, for reading back.
fn buffer_writer() -> Value {
    Value::native("string.writer", Arity::Exact(0), |_, _| {
        Ok(stream::writer(Buffer::default(), Kind::Buffer))
    })
}

const ONE_STRING: Params = Params::new(&[Param::of(FrostType::STRING)]);
const TWO_FLATS: Params = Params::new(&[Param::of(FrostType::FLAT), Param::of(FrostType::FLAT)]);

/// The text of a type-checked String argument.
fn string_arg(arg: &Value) -> &str {
    arg.as_str().expect("type-checked as a String")
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
    Value::checked_native(name, TWO_FLATS, move |_, args| {
        let index = match (&args[0], &args[1]) {
            (Value::String(text), Value::String(needle)) => {
                let byte = match occurrence {
                    First => text.find(&**needle),
                    Last => text.rfind(&**needle),
                };
                byte.map(|byte| text[..byte].chars().count())
            }
            (haystack, needle) => find_bytes(
                haystack.as_byte_slice().expect("type-checked as Flat"),
                needle.as_byte_slice().expect("type-checked as Flat"),
                occurrence,
            ),
        };
        Ok(index.map_or(Value::Null, |index| {
            Value::Int(i64::try_from(index).expect("an index fits in an Int"))
        }))
    })
}

fn count() -> Value {
    Value::checked_native("string.count", TWO_FLATS, |_, args| {
        let haystack = args[0].as_byte_slice().expect("type-checked as Flat");
        let needle = args[1].as_byte_slice().expect("type-checked as Flat");
        if needle.is_empty() {
            return Err(FrostError::from_static(
                "Function string.count requires argument 2 to be non-empty",
            ));
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
    })
}

fn is_empty() -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::FLAT)]);
    Value::checked_native("string.is_empty", PARAMS, |_, args| {
        let content = args[0].as_byte_slice().expect("type-checked as Flat");
        Ok(Value::Bool(content.is_empty()))
    })
}

// --- Stripping ---

/// Removes an affix from one edge of its input, if present.
type Strip<T> = for<'a> fn(&'a T, &T) -> Option<&'a T>;

/// A function removing an affix, its second argument, from its first argument
/// with `strip_text` or `strip_bytes`. The result is a String only when both
/// arguments are, else Bytes. An argument left with nothing removed is returned
/// as is, though as Bytes if the affix is Bytes.
fn stripper(
    name: &'static str,
    params: Params,
    strip_text: Strip<str>,
    strip_bytes: Strip<[u8]>,
) -> Value {
    Value::checked_native(name, params, move |_, args| {
        Ok(match (args[0].as_str(), args[1].as_str()) {
            (Some(text), Some(affix)) => match strip_text(text, affix) {
                Some(rest) if rest.len() < text.len() => Value::from(rest),
                _ => args[0].take(),
            },
            _ => {
                let bytes = args[0].as_byte_slice().expect("type-checked as Flat");
                let affix = args[1].as_byte_slice().expect("type-checked as Flat");
                match strip_bytes(bytes, affix) {
                    Some(rest) if rest.len() < bytes.len() => Value::from(rest),
                    _ if matches!(args[0], Value::Bytes(_)) => args[0].take(),
                    _ => Value::from(bytes),
                }
            }
        })
    })
}

fn strip_prefix() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::FLAT),
        Param::of(FrostType::FLAT).named("prefix"),
    ]);
    stripper(
        "string.strip_prefix",
        PARAMS,
        |text, prefix| text.strip_prefix(prefix),
        <[u8]>::strip_prefix,
    )
}

fn strip_suffix() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::FLAT),
        Param::of(FrostType::FLAT).named("suffix"),
    ]);
    stripper(
        "string.strip_suffix",
        PARAMS,
        |text, suffix| text.strip_suffix(suffix),
        <[u8]>::strip_suffix,
    )
}

// --- Characters ---

fn chars() -> Value {
    Value::checked_native("string.chars", ONE_STRING, |_, args| {
        Ok(string_arg(&args[0])
            .chars()
            .map(|c| Value::from(c.to_string()))
            .collect())
    })
}

/// `is_ascii(text)`: whether a String holds nothing outside ASCII. Unlike the
/// other classifiers, it passes the empty String, which holds nothing at all.
fn is_ascii() -> Value {
    Value::checked_native("string.is_ascii", ONE_STRING, |_, args| {
        Ok(Value::Bool(string_arg(&args[0]).is_ascii()))
    })
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

/// A function testing whether a String has a character, and every character
/// passes `test`.
fn classifier(name: &'static str, test: fn(char) -> bool) -> Value {
    Value::checked_native(name, ONE_STRING, move |_, args| {
        let text = string_arg(&args[0]);
        Ok(Value::Bool(!text.is_empty() && text.chars().all(test)))
    })
}

/// A function testing whether a String has a character with case, and every
/// such character passes `case`. A character without case is ignored. A
/// title-case character, such as `ǅ`, has case but is neither upper nor lower.
fn case_classifier(name: &'static str, case: fn(char) -> bool) -> Value {
    Value::checked_native(name, ONE_STRING, move |_, args| {
        let mut cased = string_arg(&args[0])
            .chars()
            .filter(|&c| has_case(c))
            .peekable();
        Ok(Value::Bool(cased.peek().is_some() && cased.all(case)))
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
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::STRING),
        Param::of(FrostType::INT).named("width"),
        Param::of(FrostType::STRING).named("fill").optional(),
    ]);
    Value::checked_native(name, PARAMS, move |_, args| {
        let width = args[1].as_int().expect("type-checked as an Int");
        let Ok(width) = usize::try_from(width) else {
            return Err(FrostError::from_string(format!(
                "Function {name} requires argument 2 (width) to be at least 0, got {width}"
            )));
        };
        let fill = match args.get(2).map(string_arg) {
            None => ' ',
            Some(fill) => {
                let mut chars = fill.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => c,
                    _ => {
                        return Err(FrostError::from_string(format!(
                            "Function {name} requires a single character as argument 3 (fill), \
                             got {}",
                            args[2].to_debug_string()
                        )));
                    }
                }
            }
        };

        let text = string_arg(&args[0]);
        let padding = width.saturating_sub(text.chars().count());
        if padding == 0 {
            return Ok(args[0].take());
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
        padded.push_str(text);
        padded.extend(std::iter::repeat_n(fill, right));
        Ok(padded.into())
    })
}
