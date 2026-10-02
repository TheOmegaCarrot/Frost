//! `std.encoding`: integers in any base, Bytes as Ints, and the Base64, hex,
//! and URL percent-encodings.
//!
//! Every function taking content to encode or decode accepts String or Bytes;
//! a String contributes its UTF-8. A decoder or parser given the right type
//! but content that does not decode returns Null.

use crate::{FrostError, FrostType, Param, Params, StdlibModule, Value};

/// The `std.encoding` module: formatting and parsing integers in bases 2 to 36,
/// converting between Bytes and Arrays of Ints, and Base64 (standard and
/// URL-safe), hex, and URL percent-encoding.
///
/// It only computes: it reads and changes nothing outside the script.
pub fn encoding() -> StdlibModule {
    StdlibModule::new(
        "encoding",
        Value::map([
            ("fmt_int", fmt_int()),
            ("parse_int", parse_int()),
            ("to_ints", to_ints()),
            ("from_ints", from_ints()),
            (
                "b64",
                Value::map([
                    ("encode", b64_encoder("encoding.b64.encode", &STANDARD)),
                    ("decode", b64_decoder("encoding.b64.decode", &STANDARD)),
                    (
                        "urlencode",
                        b64_encoder("encoding.b64.urlencode", &URL_SAFE),
                    ),
                    (
                        "urldecode",
                        b64_decoder("encoding.b64.urldecode", &URL_SAFE),
                    ),
                ]),
            ),
            (
                "hex",
                Value::map([("encode", hex_encode()), ("decode", hex_decode())]),
            ),
            (
                "url",
                Value::map([("encode", url_encode()), ("decode", url_decode())]),
            ),
        ]),
    )
}

/// The spec of a function of one String or Bytes.
const ONE_FLAT: Params = Params::new(&[Param::of(FrostType::FLAT)]);

/// The bytes of a type-checked String or Bytes argument.
fn flat_arg(arg: &Value) -> &[u8] {
    arg.as_byte_slice()
        .expect("type-checked as String or Bytes")
}

// --- Integers ---

/// `base`, the base argument of `function`, if it is from 2 to 36.
fn base_arg(function: &str, base: &Value) -> Result<u32, FrostError> {
    let base = base.as_int().expect("type-checked as an Int");
    match u32::try_from(base) {
        Ok(base @ 2..=36) => Ok(base),
        _ => Err(FrostError::from_string(format!(
            "Function {function} requires a base from 2 to 36, got {base}"
        ))),
    }
}

fn fmt_int() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::INT).named("number"),
        Param::of(FrostType::INT).named("base"),
    ]);
    Value::checked_native("encoding.fmt_int", PARAMS, |_, args| {
        let base = base_arg("encoding.fmt_int", &args[1])?;
        let number = args[0].as_int().expect("type-checked as an Int");

        // Digits come out least significant first; the magnitude of
        // `i64::MIN` fits only unsigned.
        let mut magnitude = number.unsigned_abs();
        let mut digits = Vec::new();
        loop {
            let digit =
                u32::try_from(magnitude % u64::from(base)).expect("a digit is below the base");
            digits.push(char::from_digit(digit, base).expect("a digit is below the base"));
            magnitude /= u64::from(base);
            if magnitude == 0 {
                break;
            }
        }
        if number < 0 {
            digits.push('-');
        }
        Ok(digits.into_iter().rev().collect::<String>().into())
    })
}

fn parse_int() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::STRING).named("text"),
        Param::of(FrostType::INT).named("base"),
    ]);
    Value::checked_native("encoding.parse_int", PARAMS, |_, args| {
        let base = base_arg("encoding.parse_int", &args[1])?;
        let text = args[0].as_str().expect("type-checked as a String");
        Ok(i64::from_str_radix(text, base).map_or(Value::Null, Value::Int))
    })
}

// --- Bytes as Ints ---

fn to_ints() -> Value {
    Value::checked_native("encoding.to_ints", ONE_FLAT, |_, args| {
        Ok(flat_arg(&args[0])
            .iter()
            .map(|&byte| Value::Int(i64::from(byte)))
            .collect())
    })
}

fn from_ints() -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::ARRAY)]);
    Value::checked_native("encoding.from_ints", PARAMS, |_, args| {
        let ints = args[0].as_array().expect("type-checked as an Array");
        // Every element's type is checked before any value: a wrong type is an
        // error wherever it is, while a value outside 0 to 255 makes the result Null.
        if let Some((i, element)) = ints
            .iter()
            .enumerate()
            .find(|(_, element)| element.as_int().is_none())
        {
            return Err(FrostError::from_string(format!(
                "Function encoding.from_ints requires an Array of Ints, but element {i} is {}",
                element.type_name()
            )));
        }
        let bytes: Option<Vec<u8>> = ints
            .iter()
            .map(|element| u8::try_from(element.as_int().expect("checked as an Int")).ok())
            .collect();
        Ok(bytes.map_or(Value::Null, Value::from))
    })
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
    Value::checked_native(name, ONE_FLAT, move |_, args| {
        Ok(b64_encode(flat_arg(&args[0]), alphabet).into())
    })
}

fn b64_decoder(name: &'static str, alphabet: &'static Alphabet) -> Value {
    Value::checked_native(name, ONE_FLAT, move |_, args| {
        Ok(b64_decode(flat_arg(&args[0]), alphabet).map_or(Value::Null, Value::from))
    })
}

// --- Hex ---

/// The value of the hex digit `c`, in either case.
fn hex_digit(c: u8) -> Option<u8> {
    char::from(c)
        .to_digit(16)
        .map(|digit| u8::try_from(digit).expect("a hex digit is below 16"))
}

fn hex_encode() -> Value {
    Value::checked_native("encoding.hex.encode", ONE_FLAT, |_, args| {
        let bytes = flat_arg(&args[0]);
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            for nibble in [byte >> 4, byte & 0xf] {
                out.push(char::from_digit(u32::from(nibble), 16).expect("a nibble is below 16"));
            }
        }
        Ok(out.into())
    })
}

fn hex_decode() -> Value {
    Value::checked_native("encoding.hex.decode", ONE_FLAT, |_, args| {
        let text = flat_arg(&args[0]);
        if !text.len().is_multiple_of(2) {
            return Ok(Value::Null);
        }
        let bytes: Option<Vec<u8>> = text
            .chunks(2)
            .map(|pair| Some(hex_digit(pair[0])? << 4 | hex_digit(pair[1])?))
            .collect();
        Ok(bytes.map_or(Value::Null, Value::from))
    })
}

// --- URL percent-encoding (RFC 3986) ---

/// Whether `byte` is an unreserved character, which percent-encoding leaves as is.
fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

fn url_encode() -> Value {
    Value::checked_native("encoding.url.encode", ONE_FLAT, |_, args| {
        let bytes = flat_arg(&args[0]);
        let mut out = String::with_capacity(bytes.len());
        for &byte in bytes {
            if is_unreserved(byte) {
                out.push(char::from(byte));
            } else {
                // RFC 3986 recommends uppercase hex digits.
                out.push('%');
                for nibble in [byte >> 4, byte & 0xf] {
                    let digit =
                        char::from_digit(u32::from(nibble), 16).expect("a nibble is below 16");
                    out.push(digit.to_ascii_uppercase());
                }
            }
        }
        Ok(out.into())
    })
}

fn url_decode() -> Value {
    Value::checked_native("encoding.url.decode", ONE_FLAT, |_, args| {
        let text = flat_arg(&args[0]);
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
                None => return Ok(Value::Null),
            }
        }
        // The decoded bytes are text only if they are UTF-8.
        Ok(String::from_utf8(bytes).map_or(Value::Null, Value::from))
    })
}
