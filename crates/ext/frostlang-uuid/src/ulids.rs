//! The ULID functions of `ext.uuid`, under `ulid`.

use std::sync::{Mutex, PoisonError};

use frostlang::Value;
use ulid::{Generator, Overflow, Ulid};

use crate::{BYTES, ID, NO_PARAMS, TEXT, id_arg, millis_value};

/// The one generator for the process, so that every ULID made is greater than
/// the last, wherever it was made.
static GENERATOR: Mutex<Generator> = Mutex::new(Generator::new());

/// The canonical text of `ulid`: uppercase Crockford base32.
fn text(ulid: Ulid) -> Value {
    ulid.to_string().into()
}

/// The ULID that `text` spells, in either case.
fn parse(text: &str) -> Option<Ulid> {
    // Twenty-six base32 characters hold 130 bits, two more than a ULID.
    // A first character above 7 spells a value too large, which the `ulid`
    // crate decodes by silently dropping the excess.
    if !text.starts_with(|first: char| ('0'..='7').contains(&first)) {
        return None;
    }
    Ulid::from_string(text).ok()
}

/// The ULID that `id`, type-checked as String or Bytes, holds.
fn ulid_arg(id: &Value) -> Option<Ulid> {
    id_arg(id, parse, Ulid::from_bytes)
}

pub(crate) fn new() -> Value {
    Value::checked_native("uuid.ulid.new", NO_PARAMS, |_, _| {
        // The generator holds only the last ULID made, which a panic cannot
        // leave half-written.
        let mut generator = GENERATOR.lock().unwrap_or_else(PoisonError::into_inner);
        // Overflow takes 2^80 ULIDs in one millisecond; incrementing carries
        // into the next millisecond, which keeps the order.
        let ulid = generator
            .generate()
            .unwrap_or_else(Overflow::commit_overflow_increment);
        Ok(text(ulid))
    })
}

pub(crate) fn to_bytes() -> Value {
    Value::checked_native("uuid.ulid.to_bytes", TEXT, |_, args| {
        let text = args[0].as_str().expect("type-checked as a String");
        Ok(parse(text).map_or(Value::Null, |ulid| ulid.to_bytes().as_slice().into()))
    })
}

pub(crate) fn to_string() -> Value {
    Value::checked_native("uuid.ulid.to_string", BYTES, |_, args| {
        let bytes = args[0].as_bytes().expect("type-checked as Bytes");
        Ok(<[u8; 16]>::try_from(bytes).map_or(Value::Null, |bytes| text(Ulid::from_bytes(bytes))))
    })
}

/// `timestamp(id)`: the Unix time in milliseconds that a ULID records.
pub(crate) fn timestamp() -> Value {
    Value::checked_native("uuid.ulid.timestamp", ID, |_, args| {
        Ok(ulid_arg(&args[0]).map_or(Value::Null, |ulid| millis_value(ulid.timestamp_ms())))
    })
}
