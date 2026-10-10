//! ULIDs, under `uuid.ulid`: generating them, converting between text and
//! Bytes, and reading the timestamp.
//!
//! Generated ULIDs depend on the operating system's randomness and clock, so
//! their tests check properties. The example is the ULID specification's.

use crate::common::{
    assert_arity, assert_nulls, assert_raises, assert_true, assert_values, int, now_millis, string,
};

/// The ULID specification's example, as canonical text.
const EXAMPLE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

/// [`EXAMPLE`] as a Frost Bytes literal.
const EXAMPLE_BYTES: &str = "x'01563e3ab5d3d6764c61efb99302bd5b'";

/// The time [`EXAMPLE`] records.
const EXAMPLE_MILLIS: i64 = 1_469_922_850_259;

/// Crockford's base32 alphabet, which canonical ULID text is written in.
const ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The value of base32 `text`, read independently of the extension.
fn decode(text: &str) -> u128 {
    text.chars().fold(0, |value, c| {
        let digit = ALPHABET.find(c).expect("a base32 character");
        value * 32 + u128::try_from(digit).expect("a digit is below 32")
    })
}

/// Assert `text` is canonical ULID text: 26 characters of uppercase Crockford
/// base32, the first of them at most 7.
fn assert_canonical(text: &str) {
    assert_eq!(text.len(), 26, "{text:?} has 26 characters");
    assert!(
        text.chars().all(|c| ALPHABET.contains(c)),
        "{text:?} is uppercase Crockford base32"
    );
    assert!(
        ('0'..='7').contains(&text.chars().next().expect("not empty")),
        "{text:?} begins at most 7"
    );
}

// --- new ---

#[test]
fn new_makes_canonical_text() {
    for _ in 0..100 {
        assert_canonical(&string("uuid.ulid.new()"));
    }
}

#[test]
fn new_records_the_current_time() {
    let before = now_millis();
    let text = string("uuid.ulid.new()");
    let after = now_millis();
    // The time is the first 48 of the 128 bits.
    let recorded = u64::try_from(decode(&text) >> 80).expect("48 bits fit 64");
    assert!(
        (before..=after).contains(&recorded),
        "{text:?} records {recorded}, which is from {before} to {after}"
    );
}

#[test]
fn new_ulids_strictly_increase() {
    // A thousand in a tight loop share milliseconds, so this checks the order
    // within a millisecond too.
    assert_true(&[r"
        def ids = map range(1000) with fn i -> uuid.ulid.new()
        all(range(1, 1000), fn i -> ids[i - 1] < ids[i])
    "]);
}

#[test]
fn new_ulids_strictly_increase_across_scripts() {
    let first = string("uuid.ulid.new()");
    let second = string("uuid.ulid.new()");
    assert!(first < second, "{first:?} precedes {second:?}");
}

#[test]
fn new_takes_no_arguments() {
    assert_arity("ulid.new", 0, &[1, 2]);
}

// --- to_bytes, to_string ---

#[test]
fn to_bytes_reads_text_in_any_case() {
    assert_values(&[
        (&format!("uuid.ulid.to_bytes('{EXAMPLE}')"), EXAMPLE_BYTES),
        (
            "uuid.ulid.to_bytes('01arz3ndektsv4rrffq69g5fav')",
            EXAMPLE_BYTES,
        ),
        (
            "uuid.ulid.to_bytes('01aRz3NdEkTsV4rRfFq69G5fAv')",
            EXAMPLE_BYTES,
        ),
        (
            "uuid.ulid.to_bytes('00000000000000000000000000')",
            "x'00000000000000000000000000000000'",
        ),
        (
            "uuid.ulid.to_bytes('7ZZZZZZZZZZZZZZZZZZZZZZZZZ')",
            "x'ffffffffffffffffffffffffffffffff'",
        ),
    ]);
}

#[test]
fn to_string_writes_canonical_text() {
    assert_values(&[
        (
            &format!("uuid.ulid.to_string({EXAMPLE_BYTES})"),
            &format!("'{EXAMPLE}'"),
        ),
        (
            "uuid.ulid.to_string(x'00000000000000000000000000000000')",
            "'00000000000000000000000000'",
        ),
        (
            "uuid.ulid.to_string(x'ffffffffffffffffffffffffffffffff')",
            "'7ZZZZZZZZZZZZZZZZZZZZZZZZZ'",
        ),
    ]);
}

#[test]
fn to_bytes_and_to_string_round_trip() {
    assert_true(&[
        r"
        def ids = map range(100) with fn i -> uuid.ulid.new()
        all(ids, fn id -> uuid.ulid.to_string(uuid.ulid.to_bytes(id)) == id)
        ",
        r"
        def bytes = x'0123456789abcdef0123456789abcdef'
        uuid.ulid.to_bytes(uuid.ulid.to_string(bytes)) == bytes
        ",
    ]);
    // Lowercase text comes back canonical.
    assert_values(&[(
        "uuid.ulid.to_string(uuid.ulid.to_bytes('01arz3ndektsv4rrffq69g5fav'))",
        &format!("'{EXAMPLE}'"),
    )]);
}

#[test]
fn to_bytes_returns_null_for_text_that_is_not_a_ulid() {
    assert_nulls(&[
        "uuid.ulid.to_bytes('')",
        // A character short, and a character over.
        "uuid.ulid.to_bytes('01ARZ3NDEKTSV4RRFFQ69G5FA')",
        "uuid.ulid.to_bytes('01ARZ3NDEKTSV4RRFFQ69G5FAV0')",
        // Letters outside the alphabet, which Crockford's base32 leaves out.
        "uuid.ulid.to_bytes('01ARZ3NDEKTSV4RRFFQ69G5FAI')",
        "uuid.ulid.to_bytes('01ARZ3NDEKTSV4RRFFQ69G5FAL')",
        "uuid.ulid.to_bytes('01ARZ3NDEKTSV4RRFFQ69G5FAO')",
        "uuid.ulid.to_bytes('01ARZ3NDEKTSV4RRFFQ69G5FAU')",
        // Punctuation and surrounding space.
        "uuid.ulid.to_bytes('01ARZ3NDEK-TSV4RRFFQ69G5FA')",
        "uuid.ulid.to_bytes(' 01ARZ3NDEKTSV4RRFFQ69G5FAV')",
        "uuid.ulid.to_bytes('01ARZ3NDEKTSV4RRFFQ69G5FAV ')",
        // A UUID.
        "uuid.ulid.to_bytes('017f22e2-79b0-7cc3-98c4-dc0c0c07398f')",
    ]);
}

#[test]
fn to_bytes_returns_null_for_text_too_large_for_128_bits() {
    // Twenty-six characters hold 130 bits, so a first character above 7 spells
    // a value too large. Each must be rejected, not have its excess dropped.
    assert_nulls(&[
        "uuid.ulid.to_bytes('80000000000000000000000000')",
        "uuid.ulid.to_bytes('8ZZZZZZZZZZZZZZZZZZZZZZZZZ')",
        "uuid.ulid.to_bytes('ZZZZZZZZZZZZZZZZZZZZZZZZZZ')",
        "uuid.ulid.to_bytes('zzzzzzzzzzzzzzzzzzzzzzzzzz')",
        "uuid.ulid.to_bytes('91ARZ3NDEKTSV4RRFFQ69G5FAV')",
    ]);
}

#[test]
fn to_string_returns_null_unless_given_16_bytes() {
    assert_nulls(&[
        "uuid.ulid.to_string(x'')",
        "uuid.ulid.to_string(x'01563e3ab5d3d6764c61efb99302bd')",
        "uuid.ulid.to_string(x'01563e3ab5d3d6764c61efb99302bd5b00')",
    ]);
}

#[test]
fn to_bytes_and_to_string_check_their_argument_types() {
    assert_raises(&[
        (
            "uuid.ulid.to_bytes(x'01563e3ab5d3d6764c61efb99302bd5b')",
            "Function uuid.ulid.to_bytes requires String as argument 1 (text), got Bytes",
        ),
        (
            "uuid.ulid.to_bytes(null)",
            "Function uuid.ulid.to_bytes requires String as argument 1 (text), got Null",
        ),
        (
            "uuid.ulid.to_string('01ARZ3NDEKTSV4RRFFQ69G5FAV')",
            "Function uuid.ulid.to_string requires Bytes as argument 1 (bytes), got String",
        ),
        (
            "uuid.ulid.to_string({})",
            "Function uuid.ulid.to_string requires Bytes as argument 1 (bytes), got Map",
        ),
    ]);
    assert_arity("ulid.to_bytes", 1, &[0, 2]);
    assert_arity("ulid.to_string", 1, &[0, 2]);
}

// --- timestamp ---

#[test]
fn timestamp_reads_the_time_a_ulid_records() {
    assert_eq!(
        int(&format!("uuid.ulid.timestamp('{EXAMPLE}')")),
        EXAMPLE_MILLIS
    );
    assert_eq!(
        int(&format!("uuid.ulid.timestamp({EXAMPLE_BYTES})")),
        EXAMPLE_MILLIS
    );
    assert_eq!(
        int("uuid.ulid.timestamp('01arz3ndektsv4rrffq69g5fav')"),
        EXAMPLE_MILLIS
    );
    assert_eq!(int("uuid.ulid.timestamp('00000000000000000000000000')"), 0);
    // The largest time 48 bits hold.
    assert_eq!(
        int("uuid.ulid.timestamp('7ZZZZZZZZZZZZZZZZZZZZZZZZZ')"),
        (1 << 48) - 1
    );
}

#[test]
fn timestamp_reads_the_time_new_made() {
    let before = now_millis();
    let recorded = int("uuid.ulid.timestamp(uuid.ulid.new())");
    let after = now_millis();
    let recorded = u64::try_from(recorded).expect("a time after the epoch");
    assert!(
        (before..=after).contains(&recorded),
        "{recorded} is from {before} to {after}"
    );
}

#[test]
fn timestamp_returns_null_for_what_is_not_a_ulid() {
    assert_nulls(&[
        "uuid.ulid.timestamp('')",
        "uuid.ulid.timestamp('01ARZ3NDEKTSV4RRFFQ69G5FA')",
        "uuid.ulid.timestamp('80000000000000000000000000')",
        "uuid.ulid.timestamp(x'')",
        "uuid.ulid.timestamp(x'01563e3ab5d3d6764c61efb99302bd')",
    ]);
}

#[test]
fn timestamp_checks_its_argument_type() {
    assert_raises(&[
        (
            "uuid.ulid.timestamp(1469922850259)",
            "Function uuid.ulid.timestamp requires String or Bytes as argument 1 (id), got Int",
        ),
        (
            "uuid.ulid.timestamp(null)",
            "Function uuid.ulid.timestamp requires String or Bytes as argument 1 (id), got Null",
        ),
    ]);
    assert_arity("ulid.timestamp", 1, &[0, 2]);
}

// --- ULIDs and UUIDs ---

#[test]
fn ulids_and_uuids_share_their_bytes() {
    // RFC 9562's example v7 UUID, written as a ULID.
    assert_values(&[
        (
            "uuid.to_string(uuid.ulid.to_bytes('01FWHE4YDGFK1SHH6W1G60EECF'))",
            "'017f22e2-79b0-7cc3-98c4-dc0c0c07398f'",
        ),
        (
            "uuid.ulid.to_string(uuid.to_bytes('017f22e2-79b0-7cc3-98c4-dc0c0c07398f'))",
            "'01FWHE4YDGFK1SHH6W1G60EECF'",
        ),
        // Both lay out their time in the same 48 bits.
        (
            "uuid.ulid.timestamp(uuid.to_bytes('017f22e2-79b0-7cc3-98c4-dc0c0c07398f'))",
            "1645557742000",
        ),
    ]);
}
