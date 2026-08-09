//! Tests for `Value::Bytes`: the type that carries arbitrary bytes now that
//! `String` is UTF-8 by construction.
//!
//! The distinction is the point, so most of these pin a boundary: Bytes and
//! String never mix under an operator, never compare equal, and never order
//! against each other. Byte sequences that happen to spell readable text are
//! still Bytes; nothing infers text from content.

use std::cmp::Ordering;

use frost_runtime::{FrostType, MapKey, Value};

fn bytes(b: &[u8]) -> Value {
    Value::from(b.to_vec())
}

// ============================================================
// Identity and classification
// ============================================================

#[test]
fn bytes_is_its_own_type() {
    let v = bytes(&[0xff, 0xfe]);
    assert!(v.is_bytes());
    assert!(!v.is_string());
    assert_eq!(v.frost_type(), FrostType::Bytes);
    assert_eq!(v.type_name(), "Bytes");
}

#[test]
fn readable_bytes_are_still_bytes() {
    // Content never implies text: these bytes spell "hi", and it changes nothing.
    let v = bytes(b"hi");
    assert!(v.is_bytes());
    assert_eq!(v.as_str(), None);
}

#[test]
fn bytes_is_primitive_and_nonnull() {
    let v = bytes(&[1]);
    assert!(v.is_primitive());
    assert!(v.is_nonnull());
    assert!(!v.is_structured());
}

#[test]
fn empty_bytes_is_truthy() {
    // Only null and false are falsy; emptiness is not falsiness.
    assert!(bytes(&[]).is_truthy());
}

// ============================================================
// Equality
// ============================================================

#[test]
fn bytes_compare_by_value() {
    assert_eq!(bytes(&[1, 2, 3]), bytes(&[1, 2, 3]));
    assert_ne!(bytes(&[1, 2, 3]), bytes(&[1, 2, 4]));
    assert_ne!(bytes(&[1, 2]), bytes(&[1, 2, 3]));
}

#[test]
fn bytes_never_equals_a_string_of_the_same_content() {
    // The two types are distinct even where their bytes agree: this is the whole
    // reason the split exists.
    assert_ne!(bytes(b"abc"), Value::from("abc"));
    assert_ne!(Value::from("abc"), bytes(b"abc"));
}

// ============================================================
// Concatenation
// ============================================================

#[test]
fn bytes_concatenate() {
    let joined = bytes(&[1, 2]).add(&bytes(&[3])).unwrap();
    assert_eq!(joined, bytes(&[1, 2, 3]));
}

#[test]
fn concatenating_empty_bytes_is_identity() {
    assert_eq!(bytes(&[1, 2]).add(&bytes(&[])).unwrap(), bytes(&[1, 2]));
    assert_eq!(bytes(&[]).add(&bytes(&[1, 2])).unwrap(), bytes(&[1, 2]));
}

#[test]
fn strings_still_concatenate() {
    let joined = Value::from("ab").add(&Value::from("cd")).unwrap();
    assert_eq!(joined, Value::from("abcd"));
}

#[test]
fn mixing_string_and_bytes_under_plus_is_an_error() {
    // No implicit direction: converting is the caller's decision, made explicitly.
    // The message must name both operand types: "incompatible" alone would pass
    // for every `+` type error and so would prove nothing about this rule.
    let err = Value::from("ab").add(&bytes(b"cd")).unwrap_err();
    assert!(err.message().contains("String + Bytes"), "{}", err.message());

    let err = bytes(b"cd").add(&Value::from("ab")).unwrap_err();
    assert!(err.message().contains("Bytes + String"), "{}", err.message());
}

// ============================================================
// Ordering
// ============================================================

#[test]
fn bytes_order_lexicographically() {
    assert_eq!(
        bytes(&[1, 2]).compare(&bytes(&[1, 3])).unwrap(),
        Ordering::Less
    );
    assert_eq!(
        bytes(&[1, 3]).compare(&bytes(&[1, 2])).unwrap(),
        Ordering::Greater
    );
    assert_eq!(
        bytes(&[1, 2]).compare(&bytes(&[1, 2])).unwrap(),
        Ordering::Equal
    );
}

#[test]
fn a_prefix_orders_before_its_extension() {
    assert_eq!(
        bytes(&[1, 2]).compare(&bytes(&[1, 2, 0])).unwrap(),
        Ordering::Less
    );
}

#[test]
fn high_bytes_order_above_ascii() {
    // Ordering is over raw byte values, so 0xff is greater, not a decode error.
    assert_eq!(
        bytes(&[0x7f]).compare(&bytes(&[0xff])).unwrap(),
        Ordering::Less
    );
}

#[test]
fn ordering_a_string_against_bytes_is_an_error() {
    assert!(Value::from("abc").compare(&bytes(b"abc")).is_err());
    assert!(bytes(b"abc").compare(&Value::from("abc")).is_err());
}

// ============================================================
// Map keys
// ============================================================

#[test]
fn bytes_can_key_a_map() {
    let map = Value::map([(vec![0xff, 0x00], Value::Int(1))]);
    let m = map.as_map().expect("a Map");
    assert_eq!(
        m.get(&MapKey::Bytes([0xff, 0x00].into())),
        Some(&Value::Int(1))
    );
}

#[test]
fn a_bytes_key_is_distinct_from_a_string_key() {
    // Same bytes, different key: a Map can hold both without collision.
    let map = Value::map([
        (MapKey::from("k"), Value::Int(1)),
        (MapKey::from(b"k".to_vec()), Value::Int(2)),
    ]);
    let m = map.as_map().expect("a Map");
    assert_eq!(m.len(), 2);
    assert_eq!(m.get_str("k"), Some(&Value::Int(1)));
    assert_eq!(
        m.get(&MapKey::Bytes(b"k".to_vec().into())),
        Some(&Value::Int(2))
    );
}

#[test]
fn a_bytes_value_converts_to_a_bytes_key() {
    let key = MapKey::try_from(bytes(&[1, 2])).expect("Bytes is a valid key");
    assert_eq!(key, MapKey::Bytes([1, 2].into()));
}

// ============================================================
// Rendering
// ============================================================

#[test]
fn bytes_render_as_a_bytes_literal() {
    assert_eq!(bytes(&[0xff, 0x00]).to_frost_string(), "x'ff00'");
}

#[test]
fn readable_bytes_still_render_as_a_literal() {
    // Binary never renders as text, even when it could: a reader can always tell
    // the two apart, which a content-dependent rendering would not allow.
    assert_eq!(bytes(b"hi").to_frost_string(), "x'6869'");
    assert_eq!(Value::from("hi").to_frost_string(), "hi");
}

#[test]
fn bytes_render_the_same_in_every_tier() {
    // Unlike String, whose bare and in-structure renderings differ.
    let v = bytes(&[0x0a]);
    assert_eq!(v.to_frost_string(), "x'0a'");
    assert_eq!(v.to_pretty_string(), "x'0a'");
    assert_eq!(v.to_debug_string(), "x'0a'");
}

#[test]
fn empty_bytes_render_as_an_empty_literal() {
    assert_eq!(bytes(&[]).to_frost_string(), "x''");
}
