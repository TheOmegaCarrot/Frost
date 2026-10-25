//! MessagePack.
//!
//! [`encode`] gives MessagePack data, Bytes, and [`decode`] reads it back.
//! Scripts call them as `encode(value)` and `decode(bytes)` in
//! `ext.formats.msgpack`. MessagePack has nothing to configure, so `encode`
//! takes no options.
//!
//! # Type mapping
//!
//! | Frost | MessagePack |
//! |---|---|
//! | Null | nil |
//! | Bool | bool |
//! | Int | an int, in its smallest form |
//! | Float | a float64 |
//! | [`SpecialFloat`] | a float64 |
//! | String | a str |
//! | Bytes | a bin |
//! | Array | an array |
//! | Map | a map |
//! | [`Uint64`] | a uint beyond the Int range |
//!
//! Map keys may be of any type a Map key can be. Anything else has no
//! MessagePack form: Functions and other Opaques. Arrays and Maps nest at
//! most 127 deep. A String or Bytes is at most 2^32 - 1 (4294967295) bytes
//! long, and an Array or Map holds at most 2^32 - 1 elements or entries.
//!
//! When decoding, a float32 is the Float or [`SpecialFloat`] it equals, and a
//! str that is not UTF-8 is Bytes, as a bin is. Decoding is an error for:
//!
//! - an extension type, the standard timestamp included;
//! - a Map key of a type a Frost Map key cannot be, such as nil;
//! - a Map holding a key twice;
//! - anything after the value.
//!
//! [`SpecialFloat`]: frostlang::SpecialFloat
//! [`Uint64`]: crate::uint64::Uint64

mod natives;

pub(crate) use natives::module;

use frostlang::{FrostType, MapKey, TypeSet, Value};
use serde::de::DeserializeSeed;

use crate::mapping::{self, BeyondInt, Decoder, EncodeFailure, FormatOpaque, Mapping};
use crate::{DecodeError, EncodeError};

/// The types MessagePack has a form for, Opaques aside.
const TYPES: TypeSet = FrostType::Null
    .or(FrostType::Bool)
    .or(FrostType::Int)
    .or(FrostType::Float)
    .or(FrostType::String)
    .or(FrostType::Bytes)
    .or(FrostType::Array)
    .or(FrostType::Map);

/// MessagePack's data model.
pub(crate) const MSGPACK: Mapping = Mapping {
    name: "MessagePack",
    // Anything with a MessagePack form may be a whole document, including
    // the Opaques in `opaques`.
    document_types: TYPES.or(FrostType::Opaque),
    types: TYPES,
    key_types: MapKey::TYPES,
    opaques: &[FormatOpaque::SpecialFloat, FormatOpaque::Uint64],
    // Ample for data, and shallow enough that decoding stays well within a
    // thread's stack. `rmp-serde` itself reads to 1024 levels.
    max_depth: 127,
    // MessagePack writes every length in 32 bits.
    max_len: u32::MAX as usize,
    beyond_int: BeyondInt::Uint64,
    marked: &[],
};

/// `value` as MessagePack data.
///
/// # Errors
///
/// An [`EncodeError`] if `value` holds anything MessagePack has no form for,
/// anything too long, or nests too deep; see the [module docs](self).
pub fn encode(value: &Value) -> Result<Vec<u8>, EncodeError> {
    let mut data = Vec::new();
    let encoded = mapping::encode(&MSGPACK, value, &mut rmp_serde::Serializer::new(&mut data));
    encoded.map_err(|failure| match failure {
        EncodeFailure::Refused(refusal) => refusal,
        // Writing into memory, `rmp-serde` has no I/O to fail, and the encoder
        // refuses every length it could not write.
        EncodeFailure::Serializer(err) => unreachable!("rmp-serde failed on its own: {err}"),
    })?;
    Ok(data)
}

/// The value the MessagePack data `data` holds.
///
/// # Errors
///
/// A [`DecodeError`] if `data` is not one MessagePack value, or holds what
/// Frost cannot; see the [module docs](self).
pub fn decode(data: &[u8]) -> Result<Value, DecodeError> {
    let mut deserializer = rmp_serde::Deserializer::new(data);
    let decoded = Decoder::new(&MSGPACK).deserialize(&mut deserializer);
    // `rmp-serde` reads straight through `data`, buffering nothing ahead, so
    // what is left unread shows where it stopped. It reads one value, and
    // leaves whatever follows.
    let rest = deserializer.into_inner();
    let bytes_read = data.len() - rest.len();
    let value = decoded.map_err(|err| {
        DecodeError::new(
            MSGPACK.name,
            format!("{}, {} in", reason(&err), byte_count(bytes_read)),
        )
    })?;
    if !rest.is_empty() {
        return Err(DecodeError::new(
            MSGPACK.name,
            format!("{} after the value", byte_count(rest.len())),
        ));
    }
    Ok(value)
}

/// What `error` says is wrong with the data, in a script author's terms. The
/// error then says how far reading got, as in
/// `the data ends early, 2 bytes in`.
fn reason(error: &rmp_serde::decode::Error) -> String {
    use rmp_serde::decode::Error;
    match error {
        Error::InvalidMarkerRead(err) | Error::InvalidDataRead(err)
            if err.kind() == std::io::ErrorKind::UnexpectedEof =>
        {
            "the data ends early".to_string()
        }
        // The decoder reads every value with `deserialize_any`, which accepts
        // every marker but the reserved 0xc1, so a type mismatch can only be
        // that byte.
        Error::TypeMismatch(_) => "byte 0xc1, which MessagePack reserves".to_string(),
        other => other.to_string(),
    }
}

/// A count of `n` bytes, as text: `1 byte`, `2 bytes`.
fn byte_count(n: usize) -> String {
    match n {
        1 => "1 byte".to_string(),
        n => format!("{n} bytes"),
    }
}
