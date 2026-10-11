//! The UUID functions of `ext.uuid`.

use frostlang::native::Flat;
use frostlang::{FrostBytes, FrostString, Value, frost_fn};
use uuid::{Uuid, Variant, Version};

use crate::{id_arg, millis_value};

/// The canonical text of `uuid`: lowercase hex, hyphenated.
fn text(uuid: Uuid) -> Value {
    uuid.hyphenated().to_string().into()
}

/// The UUID that `id` holds.
fn uuid_arg(id: &Flat) -> Option<Uuid> {
    id_arg(id, |text| Uuid::try_parse(text).ok(), Uuid::from_bytes)
}

/// Whether `uuid` is in the RFC 9562 layout, where it has a version.
fn is_rfc_9562(uuid: &Uuid) -> bool {
    uuid.get_variant() == Variant::RFC4122
}

#[frost_fn]
pub(crate) fn v4() -> Value {
    text(Uuid::new_v4())
}

#[frost_fn]
pub(crate) fn v7() -> Value {
    text(Uuid::now_v7())
}

#[frost_fn]
pub(crate) fn to_bytes(text: FrostString) -> Value {
    Uuid::try_parse(text.as_str()).map_or(Value::Null, |uuid| uuid.as_bytes().as_slice().into())
}

#[frost_fn]
pub(crate) fn to_string(bytes: FrostBytes) -> Value {
    Uuid::from_slice(&bytes).map_or(Value::Null, text)
}

/// `version(id)`: the version field of a UUID in the RFC 9562 layout, whatever
/// its value. Any other layout, including the nil and max UUIDs, has no version.
#[frost_fn]
pub(crate) fn version(id: Flat) -> Value {
    uuid_arg(&id)
        .filter(is_rfc_9562)
        .map_or(Value::Null, |uuid| {
            Value::Int(i64::try_from(uuid.get_version_num()).expect("a version is 4 bits"))
        })
}

/// `timestamp(id)`: the Unix time in milliseconds that a v7 UUID records.
#[frost_fn]
pub(crate) fn timestamp(id: Flat) -> Value {
    let millis = uuid_arg(&id)
        .filter(|uuid| is_rfc_9562(uuid) && uuid.get_version() == Some(Version::SortRand))
        .and_then(|uuid| uuid.get_timestamp())
        .map(|timestamp| {
            let (seconds, nanos) = timestamp.to_unix();
            seconds * 1000 + u64::from(nanos / 1_000_000)
        });
    millis.map_or(Value::Null, millis_value)
}
