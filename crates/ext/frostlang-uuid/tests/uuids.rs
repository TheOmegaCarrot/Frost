//! UUIDs: generating v4 and v7, converting between text and Bytes, and reading
//! the version and timestamp.
//!
//! Generated UUIDs depend on the operating system's randomness and clock, so
//! their tests check properties. The examples are from RFC 9562, Appendix A.

use crate::common::{
    assert_arity, assert_nulls, assert_raises, assert_true, assert_values, int, now_millis, string,
};

/// The RFC 9562 example v7 UUID, as canonical text.
const V7: &str = "017f22e2-79b0-7cc3-98c4-dc0c0c07398f";

/// [`V7`] as a Frost Bytes literal.
const V7_BYTES: &str = "x'017f22e279b07cc398c4dc0c0c07398f'";

/// The time [`V7`] records: 2022-02-22 19:22:22 UTC.
const V7_MILLIS: i64 = 1_645_557_742_000;

/// Assert `text` is canonical UUID text: 32 lowercase hex digits, hyphenated
/// as 8-4-4-4-12, of `version` and the RFC 9562 variant.
fn assert_canonical(text: &str, version: char) {
    assert_eq!(text.len(), 36, "{text:?} has 36 characters");
    for (i, c) in text.char_indices() {
        if [8, 13, 18, 23].contains(&i) {
            assert_eq!(c, '-', "{text:?} has a hyphen at {i}");
        } else {
            assert!(
                c.is_ascii_digit() || ('a'..='f').contains(&c),
                "{text:?} has a lowercase hex digit at {i}"
            );
        }
    }
    assert_eq!(
        text.as_bytes()[14],
        version as u8,
        "{text:?} is version {version}"
    );
    assert!(
        "89ab".contains(char::from(text.as_bytes()[19])),
        "{text:?} has the RFC 9562 variant"
    );
}

/// The Unix time in milliseconds that the canonical text of a v7 UUID records
/// in its first 48 bits, read independently of the extension.
fn embedded_millis(text: &str) -> u64 {
    let hex: String = text.chars().filter(|&c| c != '-').take(12).collect();
    u64::from_str_radix(&hex, 16).expect("hex digits")
}

// --- v4 ---

#[test]
fn v4_makes_canonical_version_4_text() {
    for _ in 0..100 {
        assert_canonical(&string("uuid.v4()"), '4');
    }
}

#[test]
fn v4_makes_a_new_uuid_every_time() {
    assert_true(&["len(unique(map range(1000) with fn i -> uuid.v4())) == 1000"]);
}

#[test]
fn v4_takes_no_arguments() {
    assert_arity("v4", 0, &[1, 2]);
}

// --- v7 ---

#[test]
fn v7_makes_canonical_version_7_text() {
    for _ in 0..100 {
        assert_canonical(&string("uuid.v7()"), '7');
    }
}

#[test]
fn v7_records_the_current_time() {
    let before = now_millis();
    let text = string("uuid.v7()");
    let after = now_millis();
    let recorded = embedded_millis(&text);
    assert!(
        (before..=after).contains(&recorded),
        "{text:?} records {recorded}, which is from {before} to {after}"
    );
}

#[test]
fn v7_uuids_strictly_increase() {
    // A thousand in a tight loop share milliseconds, so this checks the order
    // within a millisecond too.
    assert_true(&[r"
        def ids = map range(1000) with fn i -> uuid.v7()
        all(range(1, 1000), fn i -> ids[i - 1] < ids[i])
    "]);
}

#[test]
fn v7_uuids_strictly_increase_across_scripts() {
    let first = string("uuid.v7()");
    let second = string("uuid.v7()");
    assert!(first < second, "{first:?} precedes {second:?}");
}

#[test]
fn v7_takes_no_arguments() {
    assert_arity("v7", 0, &[1, 2]);
}

// --- to_bytes, to_string ---

#[test]
fn to_bytes_reads_every_text_form_in_any_case() {
    let forms = [
        "017f22e2-79b0-7cc3-98c4-dc0c0c07398f",
        "017F22E2-79B0-7CC3-98C4-DC0C0C07398F",
        "017f22E2-79b0-7Cc3-98C4-dc0c0C07398f",
        "017f22e279b07cc398c4dc0c0c07398f",
        "017F22E279B07CC398C4DC0C0C07398F",
        "{017f22e2-79b0-7cc3-98c4-dc0c0c07398f}",
        "{017F22E2-79B0-7CC3-98C4-DC0C0C07398F}",
        "urn:uuid:017f22e2-79b0-7cc3-98c4-dc0c0c07398f",
        "urn:uuid:017F22E2-79B0-7CC3-98C4-DC0C0C07398F",
    ];
    for form in forms {
        assert_values(&[(&format!("uuid.to_bytes('{form}')"), V7_BYTES)]);
    }
}

#[test]
fn to_string_writes_canonical_text() {
    assert_values(&[
        (&format!("uuid.to_string({V7_BYTES})"), &format!("'{V7}'")),
        (
            "uuid.to_string(x'00000000000000000000000000000000')",
            "'00000000-0000-0000-0000-000000000000'",
        ),
        (
            "uuid.to_string(x'ffffffffffffffffffffffffffffffff')",
            "'ffffffff-ffff-ffff-ffff-ffffffffffff'",
        ),
        // Any 16 Bytes are a UUID, whatever their version and variant.
        (
            "uuid.to_string(x'0123456789abcdef0123456789abcdef')",
            "'01234567-89ab-cdef-0123-456789abcdef'",
        ),
    ]);
}

#[test]
fn to_bytes_and_to_string_round_trip() {
    assert_true(&[
        r"
        def ids = (map range(100) with fn i -> uuid.v4()) + (map range(100) with fn i -> uuid.v7())
        all(ids, fn id -> uuid.to_string(uuid.to_bytes(id)) == id)
        ",
        r"
        def bytes = x'0123456789abcdef0123456789abcdef'
        uuid.to_bytes(uuid.to_string(bytes)) == bytes
        ",
    ]);
    // Text in any other form comes back canonical.
    assert_values(&[(
        "uuid.to_string(uuid.to_bytes('{017F22E2-79B0-7CC3-98C4-DC0C0C07398F}'))",
        &format!("'{V7}'"),
    )]);
}

#[test]
fn to_bytes_returns_null_for_text_that_is_not_a_uuid() {
    assert_nulls(&[
        "uuid.to_bytes('')",
        // A digit short, and a digit over.
        "uuid.to_bytes('017f22e2-79b0-7cc3-98c4-dc0c0c07398')",
        "uuid.to_bytes('017f22e2-79b0-7cc3-98c4-dc0c0c07398f0')",
        "uuid.to_bytes('017f22e279b07cc398c4dc0c0c07398')",
        // Not hex.
        "uuid.to_bytes('017f22e2-79b0-7cc3-98c4-dc0c0c07398g')",
        // Hyphens out of place, and partly missing.
        "uuid.to_bytes('017f22e279b0-7cc3-98c4-dc0c-0c07398f')",
        "uuid.to_bytes('017f22e2-79b07cc3-98c4-dc0c0c07398f')",
        // Surrounding space.
        "uuid.to_bytes(' 017f22e2-79b0-7cc3-98c4-dc0c0c07398f')",
        "uuid.to_bytes('017f22e2-79b0-7cc3-98c4-dc0c0c07398f ')",
        // An unclosed brace.
        "uuid.to_bytes('{017f22e2-79b0-7cc3-98c4-dc0c0c07398f')",
        // A ULID.
        "uuid.to_bytes('01FWHE4YDGFK1SHH6W1G60EECF')",
    ]);
}

#[test]
fn to_string_returns_null_unless_given_16_bytes() {
    assert_nulls(&[
        "uuid.to_string(x'')",
        "uuid.to_string(x'017f22e279b07cc398c4dc0c0c0739')",
        "uuid.to_string(x'017f22e279b07cc398c4dc0c0c07398f00')",
    ]);
}

#[test]
fn to_bytes_and_to_string_check_their_argument_types() {
    assert_raises(&[
        (
            "uuid.to_bytes(x'017f22e279b07cc398c4dc0c0c07398f')",
            "Function uuid.to_bytes requires String as argument 1 (text), got Bytes",
        ),
        (
            "uuid.to_bytes(null)",
            "Function uuid.to_bytes requires String as argument 1 (text), got Null",
        ),
        (
            "uuid.to_string('017f22e2-79b0-7cc3-98c4-dc0c0c07398f')",
            "Function uuid.to_string requires Bytes as argument 1 (bytes), got String",
        ),
        (
            "uuid.to_string([1, 2])",
            "Function uuid.to_string requires Bytes as argument 1 (bytes), got Array",
        ),
    ]);
    assert_arity("to_bytes", 1, &[0, 2]);
    assert_arity("to_string", 1, &[0, 2]);
}

// --- version ---

#[test]
fn version_reads_the_version_of_text_or_bytes() {
    assert_values(&[
        (&format!("uuid.version('{V7}')"), "7"),
        (&format!("uuid.version({V7_BYTES})"), "7"),
        // RFC 9562's v1 and v4 examples.
        ("uuid.version('c232ab00-9414-11ec-b3c8-9f6bdeced846')", "1"),
        ("uuid.version('919108f7-52d1-4320-9bac-f847db4148a8')", "4"),
        ("uuid.version(x'919108f752d143209bacf847db4148a8')", "4"),
        // Any form of text.
        (
            "uuid.version('{919108F7-52D1-4320-9BAC-F847DB4148A8}')",
            "4",
        ),
        // The version field is read whatever its value, defined or not.
        ("uuid.version('00000000-0000-0000-8000-000000000000')", "0"),
        ("uuid.version('ffffffff-ffff-ffff-bfff-ffffffffffff')", "15"),
        ("uuid.version(uuid.v4())", "4"),
        ("uuid.version(uuid.v7())", "7"),
    ]);
}

#[test]
fn version_returns_null_outside_the_rfc_9562_layout() {
    assert_nulls(&[
        // The nil and max UUIDs.
        "uuid.version('00000000-0000-0000-0000-000000000000')",
        "uuid.version('ffffffff-ffff-ffff-ffff-ffffffffffff')",
        // V7's version field, with the NCS, Microsoft, and reserved variants.
        "uuid.version('017f22e2-79b0-7cc3-08c4-dc0c0c07398f')",
        "uuid.version('017f22e2-79b0-7cc3-c8c4-dc0c0c07398f')",
        "uuid.version('017f22e2-79b0-7cc3-e8c4-dc0c0c07398f')",
    ]);
}

#[test]
fn version_returns_null_for_what_is_not_a_uuid() {
    assert_nulls(&[
        "uuid.version('')",
        "uuid.version('017f22e2-79b0-7cc3-98c4-dc0c0c07398')",
        "uuid.version(x'')",
        "uuid.version(x'017f22e279b07cc398c4dc0c0c0739')",
        "uuid.version(x'017f22e279b07cc398c4dc0c0c07398f00')",
    ]);
}

#[test]
fn version_checks_its_argument_type() {
    assert_raises(&[
        (
            "uuid.version(7)",
            "Function uuid.version requires String or Bytes as argument 1 (id), got Int",
        ),
        (
            "uuid.version(null)",
            "Function uuid.version requires String or Bytes as argument 1 (id), got Null",
        ),
    ]);
    assert_arity("version", 1, &[0, 2]);
}

// --- timestamp ---

#[test]
fn timestamp_reads_the_time_a_v7_uuid_records() {
    assert_eq!(int(&format!("uuid.timestamp('{V7}')")), V7_MILLIS);
    assert_eq!(int(&format!("uuid.timestamp({V7_BYTES})")), V7_MILLIS);
    assert_eq!(
        int("uuid.timestamp('urn:uuid:017F22E2-79B0-7CC3-98C4-DC0C0C07398F')"),
        V7_MILLIS
    );
    // The largest time 48 bits hold.
    assert_eq!(
        int("uuid.timestamp('ffffffff-ffff-7fff-bfff-ffffffffffff')"),
        (1 << 48) - 1
    );
}

#[test]
fn timestamp_reads_the_time_v7_made() {
    let before = now_millis();
    let recorded = int("uuid.timestamp(uuid.v7())");
    let after = now_millis();
    let recorded = u64::try_from(recorded).expect("a time after the epoch");
    assert!(
        (before..=after).contains(&recorded),
        "{recorded} is from {before} to {after}"
    );
}

#[test]
fn timestamp_returns_null_for_other_than_a_v7_uuid() {
    assert_nulls(&[
        // RFC 9562's v1 example records a time, but not as v7 does.
        "uuid.timestamp('c232ab00-9414-11ec-b3c8-9f6bdeced846')",
        "uuid.timestamp('919108f7-52d1-4320-9bac-f847db4148a8')",
        "uuid.timestamp(uuid.v4())",
        // V7's version field, with the NCS variant.
        "uuid.timestamp('017f22e2-79b0-7cc3-08c4-dc0c0c07398f')",
        "uuid.timestamp('00000000-0000-0000-0000-000000000000')",
        "uuid.timestamp('ffffffff-ffff-ffff-ffff-ffffffffffff')",
        // Not a UUID.
        "uuid.timestamp('')",
        "uuid.timestamp(x'017f22e279b07cc398c4dc0c0c0739')",
    ]);
}

#[test]
fn timestamp_checks_its_argument_type() {
    assert_raises(&[(
        "uuid.timestamp(1645557742000)",
        "Function uuid.timestamp requires String or Bytes as argument 1 (id), got Int",
    )]);
    assert_arity("timestamp", 1, &[0, 2]);
}
