//! The UUID functions of `ext.uuid`.

use frostlang::Value;
use uuid::{Uuid, Variant, Version};

use crate::{BYTES, ID, NO_PARAMS, TEXT, id_arg, millis_value};

/// The canonical text of `uuid`: lowercase hex, hyphenated.
fn text(uuid: Uuid) -> Value {
    uuid.hyphenated().to_string().into()
}

/// The UUID that `id`, type-checked as String or Bytes, holds.
fn uuid_arg(id: &Value) -> Option<Uuid> {
    id_arg(id, |text| Uuid::try_parse(text).ok(), Uuid::from_bytes)
}

/// Whether `uuid` is in the RFC 9562 layout, where it has a version.
fn is_rfc_9562(uuid: &Uuid) -> bool {
    uuid.get_variant() == Variant::RFC4122
}

pub(crate) fn v4() -> Value {
    Value::checked_native("uuid.v4", NO_PARAMS, |_, _| Ok(text(Uuid::new_v4())))
}

pub(crate) fn v7() -> Value {
    Value::checked_native("uuid.v7", NO_PARAMS, |_, _| Ok(text(Uuid::now_v7())))
}

pub(crate) fn to_bytes() -> Value {
    Value::checked_native("uuid.to_bytes", TEXT, |_, args| {
        let text = args[0].as_str().expect("type-checked as a String");
        Ok(Uuid::try_parse(text).map_or(Value::Null, |uuid| uuid.as_bytes().as_slice().into()))
    })
}

pub(crate) fn to_string() -> Value {
    Value::checked_native("uuid.to_string", BYTES, |_, args| {
        let bytes = args[0].as_bytes().expect("type-checked as Bytes");
        Ok(Uuid::from_slice(bytes).map_or(Value::Null, text))
    })
}

/// `version(id)`: the version field of a UUID in the RFC 9562 layout, whatever
/// its value. Any other layout, including the nil and max UUIDs, has no version.
pub(crate) fn version() -> Value {
    Value::checked_native("uuid.version", ID, |_, args| {
        Ok(uuid_arg(&args[0])
            .filter(is_rfc_9562)
            .map_or(Value::Null, |uuid| {
                Value::Int(i64::try_from(uuid.get_version_num()).expect("a version is 4 bits"))
            }))
    })
}

/// `timestamp(id)`: the Unix time in milliseconds that a v7 UUID records.
pub(crate) fn timestamp() -> Value {
    Value::checked_native("uuid.timestamp", ID, |_, args| {
        let millis = uuid_arg(&args[0])
            .filter(|uuid| is_rfc_9562(uuid) && uuid.get_version() == Some(Version::SortRand))
            .and_then(|uuid| uuid.get_timestamp())
            .map(|timestamp| {
                let (seconds, nanos) = timestamp.to_unix();
                seconds * 1000 + u64::from(nanos / 1_000_000)
            });
        Ok(millis.map_or(Value::Null, millis_value))
    })
}
