//! `std.encoding`: integers in any base, numbers to fixed decimal places, Bytes
//! as Ints, and the Base64, hex, and URL percent-encodings.
//!
//! Every function taking content to encode or decode accepts String or Bytes;
//! a String contributes its UTF-8. A decoder or parser given the right type
//! but content that does not decode returns Null.

use crate::native::{Flat, Numeric};
use crate::{
    FrostArray, FrostError, FrostResult, FrostString, NativeCtx, StdlibModule, Value,
    frost_closure, frost_fn,
};

/// The `std.encoding` module: formatting and parsing integers in bases 2 to 36,
/// formatting numbers to a fixed number of decimal places,
/// converting between Bytes and Arrays of Ints, and Base64 (standard and
/// URL-safe), hex, and URL percent-encoding.
///
/// It only computes: it reads and changes nothing outside the script.
pub fn encoding() -> StdlibModule {
    StdlibModule::builder("encoding")
        .function(fmt_int_frost())
        .function(parse_int_frost())
        .function(fmt_fixed_frost())
        .function(to_ints_frost())
        .function(from_ints_frost())
        .module("b64", |b64| {
            b64.value("encode", b64_encoder("std.encoding.b64.encode", &STANDARD))
                .value("decode", b64_decoder("std.encoding.b64.decode", &STANDARD))
                .value(
                    "urlencode",
                    b64_encoder("std.encoding.b64.urlencode", &URL_SAFE),
                )
                .value(
                    "urldecode",
                    b64_decoder("std.encoding.b64.urldecode", &URL_SAFE),
                )
        })
        .module("hex", |hex| {
            hex.function(hex_encode_frost())
                .function(hex_decode_frost())
        })
        .module("url", |url| {
            url.function(url_encode_frost())
                .function(url_decode_frost())
        })
        .build()
}

// --- Integers ---

/// `base`, the base argument of `function`, if it is from 2 to 36.
fn base_arg(function: &str, base: i64) -> Result<u32, FrostError> {
    match u32::try_from(base) {
        Ok(base @ 2..=36) => Ok(base),
        _ => Err(FrostError::from_string(format!(
            "Function {function} requires a base from 2 to 36, got {base}"
        ))),
    }
}

#[frost_fn]
fn fmt_int(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    number: i64,
    base: i64,
) -> Result<String, FrostError> {
    let base = base_arg(ctx.name(), base)?;

    // Digits come out least significant first; the magnitude of
    // `i64::MIN` fits only unsigned.
    let mut magnitude = number.unsigned_abs();
    let mut digits = Vec::new();
    loop {
        let digit = u32::try_from(magnitude % u64::from(base)).expect("a digit is below the base");
        digits.push(char::from_digit(digit, base).expect("a digit is below the base"));
        magnitude /= u64::from(base);
        if magnitude == 0 {
            break;
        }
    }
    if number < 0 {
        digits.push('-');
    }
    Ok(digits.into_iter().rev().collect())
}

#[frost_fn]
fn parse_int(#[frost(ctx)] ctx: &mut NativeCtx<'_>, text: FrostString, base: i64) -> FrostResult {
    let base = base_arg(ctx.name(), base)?;
    Ok(i64::from_str_radix(&text, base).map_or(Value::Null, Value::Int))
}

// --- Fixed decimal places ---

/// `fmt_fixed(number, places)`: `number` in decimal with exactly `places` digits
/// after the point, and no point when `places` is 0. A Float rounds from its
/// exact binary value, with exact ties going to the even digit, so `2.675`
/// (stored just below) gives `2.67` and `0.125` gives `0.12`. An Int is written
/// exactly. A result of zero never has a sign.
#[frost_fn]
fn fmt_fixed(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    number: Numeric,
    places: i64,
) -> Result<String, FrostError> {
    // Rust's formatting accepts at most `u16::MAX` places.
    let Ok(places) = u16::try_from(places) else {
        return Err(FrostError::from_string(format!(
            "Function {} requires places from 0 to {}, got {places}",
            ctx.name(),
            u16::MAX
        )));
    };
    let places = usize::from(places);
    Ok(match number {
        Numeric::Int(n) if places == 0 => n.to_string(),
        Numeric::Int(n) => format!("{n}.{}", "0".repeat(places)),
        Numeric::Float(f) => {
            let mut fixed = format!("{:.*}", places, f.get());
            // A negative Float that rounds to zero keeps its sign; drop it.
            if fixed.starts_with('-') && fixed[1..].bytes().all(|b| matches!(b, b'0' | b'.')) {
                fixed.remove(0);
            }
            fixed
        }
    })
}

// --- Bytes as Ints ---

#[frost_fn]
fn to_ints(#[frost(unnamed)] content: Flat) -> Value {
    content
        .as_bytes()
        .iter()
        .map(|&byte| Value::Int(i64::from(byte)))
        .collect()
}

#[frost_fn]
fn from_ints(#[frost(ctx)] ctx: &mut NativeCtx<'_>, ints: FrostArray) -> FrostResult {
    // Every element's type is checked before any value: a wrong type is an
    // error wherever it is, while a value outside 0 to 255 makes the result Null.
    if let Some((i, element)) = ints
        .iter()
        .enumerate()
        .find(|(_, element)| element.as_int().is_none())
    {
        return Err(FrostError::from_string(format!(
            "Function {} requires an Array of Ints, but element {i} is {}",
            ctx.name(),
            element.type_name()
        )));
    }
    let bytes: Option<Vec<u8>> = ints
        .iter()
        .map(|element| u8::try_from(element.as_int().expect("checked as an Int")).ok())
        .collect();
    Ok(bytes.map_or(Value::Null, Value::from))
}

// --- Base64 (RFC 4648) ---

/// A Base64 alphabet. Both alphabets share their first 62 characters
/// (`A` to `Z`, `a` to `z`, `0` to `9`) and differ in the last two.
struct Alphabet {
    /// The characters for 62 and 63.
    last_two: [u8; 2],
}

/// The standard alphabet: `+` and `/`.
const STANDARD: Alphabet = Alphabet { last_two: *b"+/" };

/// The URL- and filename-safe alphabet: `-` and `_`.
const URL_SAFE: Alphabet = Alphabet { last_two: *b"-_" };

impl Alphabet {
    /// The character for `sextet`, which is below 64.
    fn char_of(&self, sextet: u32) -> char {
        let sextet = u8::try_from(sextet).expect("a sextet is below 64");
        char::from(match sextet {
            0..=25 => b'A' + sextet,
            26..=51 => b'a' + (sextet - 26),
            52..=61 => b'0' + (sextet - 52),
            62 => self.last_two[0],
            _ => self.last_two[1],
        })
    }

    /// The sextet `c` stands for, if it is in the alphabet.
    fn sextet_of(&self, c: u8) -> Option<u32> {
        let sextet = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            _ if c == self.last_two[0] => 62,
            _ if c == self.last_two[1] => 63,
            _ => return None,
        };
        Some(u32::from(sextet))
    }
}

/// `bytes` in Base64, padded with `=` to a multiple of four characters.
fn b64_encode(bytes: &[u8], alphabet: &Alphabet) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        // Up to three bytes, high byte first, as one 24-bit group.
        let group = chunk
            .iter()
            .zip([16, 8, 0])
            .fold(0, |group, (&byte, shift)| group | u32::from(byte) << shift);
        // N bytes need N + 1 sextets to carry them; padding fills the rest.
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(alphabet.char_of(group >> shift & 0x3f));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The bytes that `text` encodes in padded Base64, or `None` if it encodes none.
///
/// Only the canonical encoding decodes: padding must be correct, and the bits
/// past the last whole byte must be zero.
fn b64_decode(text: &[u8], alphabet: &Alphabet) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(4) {
        return None;
    }
    let groups = text.len() / 4;
    let mut out = Vec::with_capacity(groups * 3);
    for (index, chunk) in text.chunks(4).enumerate() {
        let padding = chunk.iter().rev().take_while(|&&c| c == b'=').count();
        if padding > 2 || (padding > 0 && index + 1 != groups) {
            return None;
        }
        let group = chunk[..4 - padding]
            .iter()
            .zip([18, 12, 6, 0])
            .try_fold(0, |group, (&c, shift)| {
                Some(group | alphabet.sextet_of(c)? << shift)
            })?;
        let byte_count = 3 - padding;
        let unused_bits = 24 - 8 * byte_count;
        if group & ((1 << unused_bits) - 1) != 0 {
            return None;
        }
        out.extend_from_slice(&group.to_be_bytes()[1..=byte_count]);
    }
    Some(out)
}

fn b64_encoder(name: &'static str, alphabet: &'static Alphabet) -> Value {
    frost_closure!(name, move |#[frost(unnamed)] content: Flat| {
        b64_encode(content.as_bytes(), alphabet)
    })
}

fn b64_decoder(name: &'static str, alphabet: &'static Alphabet) -> Value {
    frost_closure!(name, move |#[frost(unnamed)] content: Flat| {
        b64_decode(content.as_bytes(), alphabet).map_or(Value::Null, Value::from)
    })
}

// --- Hex ---

/// The value of the hex digit `c`, in either case.
fn hex_digit(c: u8) -> Option<u8> {
    char::from(c)
        .to_digit(16)
        .map(|digit| u8::try_from(digit).expect("a hex digit is below 16"))
}

#[frost_fn(name = "encode")]
fn hex_encode(#[frost(unnamed)] content: Flat) -> String {
    let bytes = content.as_bytes();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        for nibble in [byte >> 4, byte & 0xf] {
            out.push(char::from_digit(u32::from(nibble), 16).expect("a nibble is below 16"));
        }
    }
    out
}

#[frost_fn(name = "decode")]
fn hex_decode(#[frost(unnamed)] content: Flat) -> Value {
    let text = content.as_bytes();
    if !text.len().is_multiple_of(2) {
        return Value::Null;
    }
    let bytes: Option<Vec<u8>> = text
        .chunks(2)
        .map(|pair| Some(hex_digit(pair[0])? << 4 | hex_digit(pair[1])?))
        .collect();
    bytes.map_or(Value::Null, Value::from)
}

// --- URL percent-encoding (RFC 3986) ---

/// Whether `byte` is an unreserved character, which percent-encoding leaves as is.
fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

#[frost_fn(name = "encode")]
fn url_encode(#[frost(unnamed)] content: Flat) -> String {
    let bytes = content.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        if is_unreserved(byte) {
            out.push(char::from(byte));
        } else {
            // RFC 3986 recommends uppercase hex digits.
            out.push('%');
            for nibble in [byte >> 4, byte & 0xf] {
                let digit = char::from_digit(u32::from(nibble), 16).expect("a nibble is below 16");
                out.push(digit.to_ascii_uppercase());
            }
        }
    }
    out
}

#[frost_fn(name = "decode")]
fn url_decode(#[frost(unnamed)] content: Flat) -> Value {
    let text = content.as_bytes();
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.iter();
    while let Some(&byte) = rest.next() {
        if byte != b'%' {
            bytes.push(byte);
            continue;
        }
        let escaped = match (rest.next(), rest.next()) {
            (Some(&high), Some(&low)) => hex_digit(high).zip(hex_digit(low)),
            _ => None,
        };
        match escaped {
            Some((high, low)) => bytes.push(high << 4 | low),
            None => return Value::Null,
        }
    }
    // The decoded bytes are text only if they are UTF-8.
    String::from_utf8(bytes).map_or(Value::Null, Value::from)
}
