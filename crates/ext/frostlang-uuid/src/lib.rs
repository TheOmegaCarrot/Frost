//! UUIDs and ULIDs for Frost scripts: generating them, converting between their
//! text and Bytes forms, and reading what they record.
//!
//! A host installs the extension with [`extension`], and scripts import it as
//! `ext.uuid`:
//!
//! ```
//! use frostlang::ImporterBuilder;
//!
//! let importer = ImporterBuilder::new()
//!     .with_extension(frostlang_uuid::extension())
//!     .expect("no other extension is named `uuid`")
//!     .build();
//! ```

mod ulids;
mod uuids;

use frostlang::{Extension, FrostType, Param, Params, Value};

/// The `ext.uuid` extension.
///
/// For UUIDs (RFC 9562):
///
/// - `v4()` and `v7()` generate a UUID of that version, as canonical text:
///   lowercase hex, hyphenated.
/// - `to_bytes(text)` reads a UUID's text, hyphenated, plain hex, braced, or as
///   a `urn:uuid:` URN, in any case, and gives its 16 Bytes.
/// - `to_string(bytes)` gives the canonical text of 16 Bytes.
/// - `version(id)` gives the version of a UUID in the RFC 9562 layout.
/// - `timestamp(id)` gives the time a v7 UUID records, in milliseconds since
///   the Unix epoch.
///
/// For ULIDs, under `ulid`:
///
/// - `new()` generates a ULID, as 26 characters of uppercase Crockford base32.
/// - `to_bytes(text)` reads a ULID's text, in any case, and gives its 16 Bytes.
/// - `to_string(bytes)` gives the canonical text of 16 Bytes.
/// - `timestamp(id)` gives the time a ULID records, in milliseconds since the
///   Unix epoch.
///
/// Where a function takes an `id`, it may be text or 16 Bytes.
/// A function given the right type, but content that is not an ID or lacks what
/// was asked for, returns Null.
///
/// Generation always draws on the operating system's randomness and the system
/// clock. Within a process, v7 UUIDs are strictly increasing, as are ULIDs,
/// even when several are made in the same millisecond.
pub fn extension() -> Extension {
    Extension::new(
        "uuid",
        Value::map([
            ("v4", uuids::v4()),
            ("v7", uuids::v7()),
            ("to_bytes", uuids::to_bytes()),
            ("to_string", uuids::to_string()),
            ("version", uuids::version()),
            ("timestamp", uuids::timestamp()),
            (
                "ulid",
                Value::map([
                    ("new", ulids::new()),
                    ("to_bytes", ulids::to_bytes()),
                    ("to_string", ulids::to_string()),
                    ("timestamp", ulids::timestamp()),
                ]),
            ),
        ]),
    )
    .expect("`uuid` is a valid extension name")
}

/// The spec of a function of no arguments.
const NO_PARAMS: Params = Params::new(&[]);

/// The spec of a function of an ID's text.
const TEXT: Params = Params::new(&[Param::of(FrostType::STRING).named("text")]);

/// The spec of a function of an ID's Bytes.
const BYTES: Params = Params::new(&[Param::of(FrostType::BYTES).named("bytes")]);

/// The spec of a function of an ID as text or Bytes.
const ID: Params = Params::new(&[Param::of(FrostType::FLAT).named("id")]);

/// The ID that `id`, type-checked as String or Bytes, holds: text read with
/// `parse`, or exactly 16 Bytes read with `from_bytes`.
fn id_arg<T>(
    id: &Value,
    parse: impl FnOnce(&str) -> Option<T>,
    from_bytes: impl FnOnce([u8; 16]) -> T,
) -> Option<T> {
    match id {
        Value::String(text) => parse(text.as_str()),
        Value::Bytes(bytes) => <[u8; 16]>::try_from(&bytes[..]).ok().map(from_bytes),
        other => unreachable!("type-checked as String or Bytes, got {}", other.type_name()),
    }
}

/// A millisecond count of at most 48 bits, which every ID's timestamp is, as an
/// Int.
fn millis_value(millis: u64) -> Value {
    Value::Int(i64::try_from(millis).expect("48 bits fit an Int"))
}
