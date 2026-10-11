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

use frostlang::native::Flat;
use frostlang::{Extension, Value};

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
            ("v4", uuids::v4_frost()),
            ("v7", uuids::v7_frost()),
            ("to_bytes", uuids::to_bytes_frost()),
            ("to_string", uuids::to_string_frost()),
            ("version", uuids::version_frost()),
            ("timestamp", uuids::timestamp_frost()),
            (
                "ulid",
                Value::map([
                    ("new", ulids::new_frost()),
                    ("to_bytes", ulids::to_bytes_frost()),
                    ("to_string", ulids::to_string_frost()),
                    ("timestamp", ulids::timestamp_frost()),
                ]),
            ),
        ]),
    )
    .expect("`uuid` is a valid extension name")
}

/// The ID that `id` holds: text read with `parse`, or exactly 16 Bytes read with
/// `from_bytes`.
fn id_arg<T>(
    id: &Flat,
    parse: impl FnOnce(&str) -> Option<T>,
    from_bytes: impl FnOnce([u8; 16]) -> T,
) -> Option<T> {
    match id {
        Flat::String(text) => parse(text.as_str()),
        Flat::Bytes(bytes) => <[u8; 16]>::try_from(&bytes[..]).ok().map(from_bytes),
    }
}

/// A millisecond count of at most 48 bits, which every ID's timestamp is, as an
/// Int.
fn millis_value(millis: u64) -> Value {
    Value::Int(i64::try_from(millis).expect("48 bits fit an Int"))
}
