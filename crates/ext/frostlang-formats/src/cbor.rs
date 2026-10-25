//! CBOR, as RFC 8949 defines it.
//!
//! [`encode`] gives CBOR data, Bytes, and [`decode`] reads it back. Scripts
//! call them as `encode(value)` and `decode(bytes)` in `ext.formats.cbor`.
//! CBOR has nothing to configure, so `encode` takes no options.
//!
//! # Type mapping
//!
//! | Frost | CBOR |
//! |---|---|
//! | Null | null |
//! | Bool | a boolean |
//! | Int | an integer, in its smallest form |
//! | Float | a float64 |
//! | [`SpecialFloat`] | a float64 |
//! | String | a text string |
//! | Bytes | a byte string |
//! | Array | an array |
//! | Map | a map |
//! | [`Uint64`] | an unsigned integer beyond the Int range |
//!
//! Map keys may be of any type a Map key can be. Anything else has no CBOR
//! form: Functions and other Opaques. Arrays and Maps nest at most 127 deep.
//! Encoding gives every string, array, and map a definite length.
//!
//! When decoding:
//!
//! - a float16 or float32 is the Float or [`SpecialFloat`] it equals;
//! - an indefinite-length string, array, or map is read as a definite-length
//!   one is;
//! - a bignum (tag 2 or 3) of a definite-length byte string is the integer
//!   it holds, as an Int or [`Uint64`];
//! - the self-described CBOR tag (55799), which may start the data, is
//!   skipped.
//!
//! Decoding is an error for:
//!
//! - an integer below the Int range, or a bignum beyond both the Int and
//!   [`Uint64`] ranges;
//! - a bignum of an indefinite-length byte string, or any other tag;
//! - `undefined`, or any other simple value but a boolean or null;
//! - a text string that is not UTF-8;
//! - a Map key of a type a Frost Map key cannot be, such as null;
//! - a Map holding a key twice;
//! - anything after the value.
//!
//! [`SpecialFloat`]: frostlang::SpecialFloat
//! [`Uint64`]: crate::uint64::Uint64

mod natives;

pub(crate) use natives::module;

use frostlang::{FrostType, MapKey, TypeSet, Value};

use crate::mapping::{self, BeyondInt, EncodeFailure, FormatOpaque, Mapping};
use crate::{DecodeError, EncodeError};

/// The types CBOR has a form for, Opaques aside.
const TYPES: TypeSet = FrostType::Null
    .or(FrostType::Bool)
    .or(FrostType::Int)
    .or(FrostType::Float)
    .or(FrostType::String)
    .or(FrostType::Bytes)
    .or(FrostType::Array)
    .or(FrostType::Map);

/// CBOR's data model.
pub(crate) const CBOR: Mapping = Mapping {
    name: "CBOR",
    // Anything with a CBOR form may be a whole document, including the
    // Opaques in `opaques`.
    document_types: TYPES.or(FrostType::Opaque),
    types: TYPES,
    key_types: MapKey::TYPES,
    opaques: &[FormatOpaque::SpecialFloat, FormatOpaque::Uint64],
    // Ample for data, and shallow enough that decoding stays well within a
    // thread's stack; `minicbor-serde` sets no limit of its own.
    max_depth: 127,
    // CBOR writes lengths in up to 64 bits.
    max_len: usize::MAX,
    beyond_int: BeyondInt::Uint64,
    marked: &[],
};

/// The self-described CBOR tag, 55799, as it starts data that carries it.
const SELF_DESCRIBED: [u8; 3] = [0xd9, 0xd9, 0xf7];

/// `value` as CBOR data.
///
/// # Errors
///
/// An [`EncodeError`] if `value` holds anything CBOR has no form for, or
/// nests too deep; see the [module docs](self).
pub fn encode(value: &Value) -> Result<Vec<u8>, EncodeError> {
    let mut data = Vec::new();
    let mut serializer = minicbor_serde::Serializer::new(&mut data);
    // Null serializes as serde's unit, which `minicbor-serde` otherwise writes
    // as an empty array.
    serializer.serialize_unit_as_null(true);
    let encoded = mapping::encode(&CBOR, value, &mut serializer);
    encoded.map_err(|failure| match failure {
        EncodeFailure::Refused(refusal) => refusal,
        // Writing into memory, `minicbor-serde` has no I/O to fail.
        EncodeFailure::Serializer(err) => unreachable!("minicbor-serde failed on its own: {err}"),
    })?;
    Ok(data)
}

/// The value the CBOR data `data` holds.
///
/// # Errors
///
/// A [`DecodeError`] if `data` is not one CBOR value, or holds what Frost
/// cannot; see the [module docs](self).
pub fn decode(data: &[u8]) -> Result<Value, DecodeError> {
    let mut deserializer = minicbor_serde::Deserializer::new(data);
    // Skipped, rather than cut off, so that positions count from the start
    // of `data`.
    if data.starts_with(&SELF_DESCRIBED) {
        deserializer
            .decoder_mut()
            .set_position(SELF_DESCRIBED.len());
    }
    let decoded = mapping::decode(&CBOR, &mut deserializer);
    let bytes_read = deserializer.decoder().position();
    let value = decoded.map_err(|failure| match failure.refusal {
        // The refusal's own text, with its position added: `minicbor-serde`'s
        // error would prefix it with `decode error: ` and give no position.
        Some(reason) => DecodeError::bytes_in(CBOR.name, reason, bytes_read),
        // `minicbor`'s own error, for malformed data or a tag or simple value
        // it does not read; it gives its own position.
        None => DecodeError::new(CBOR.name, failure.error),
    })?;
    let rest = &data[bytes_read..];
    if !rest.is_empty() {
        return Err(DecodeError::trailing_bytes(CBOR.name, rest.len()));
    }
    Ok(value)
}
