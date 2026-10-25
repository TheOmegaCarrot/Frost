//! `Uint64` from Rust: the type, and the helpers between unsigned integers
//! and Frost values, as a host uses them.

use frostlang::{SpecialFloat, Value};
use frostlang_formats::uint64::{self, Uint64};

use crate::common::uint64;

/// The first unsigned integer beyond the Int range.
const FIRST_BEYOND: u64 = i64::MAX as u64 + 1;

#[test]
fn a_uint64_is_only_beyond_the_int_range() {
    assert_eq!(Uint64::new(0), None);
    assert_eq!(Uint64::new(i64::MAX as u64), None, "an Int holds it");
    assert_eq!(
        Uint64::new(FIRST_BEYOND).map(Uint64::get),
        Some(FIRST_BEYOND)
    );
    assert_eq!(Uint64::new(u64::MAX).map(Uint64::get), Some(u64::MAX));
}

#[test]
fn a_uint64_displays_its_decimal_text() {
    assert_eq!(uint64(u64::MAX).to_string(), "18446744073709551615");
}

#[test]
fn uint64s_are_equal_by_value() {
    assert_eq!(uint64(u64::MAX), uint64(u64::MAX));
    assert_ne!(uint64(u64::MAX), uint64(u64::MAX - 1));
    assert_eq!(
        Value::opaque(uint64(u64::MAX)),
        Value::opaque(uint64(u64::MAX)),
        "as Opaques too"
    );
}

#[test]
fn to_value_gives_an_int_if_it_fits_one() {
    assert_eq!(uint64::to_value(0), Value::Int(0));
    assert_eq!(uint64::to_value(i64::MAX as u64), Value::Int(i64::MAX));
    assert_eq!(
        uint64::to_value(FIRST_BEYOND),
        Value::opaque(uint64(FIRST_BEYOND))
    );
    assert_eq!(uint64::to_value(u64::MAX), Value::opaque(uint64(u64::MAX)));
}

#[test]
fn from_value_reads_an_unsigned_integer() {
    for n in [0, 5, i64::MAX as u64, FIRST_BEYOND, u64::MAX] {
        assert_eq!(uint64::from_value(&uint64::to_value(n)), Some(n), "{n}");
    }
}

#[test]
fn from_value_reads_nothing_else() {
    let special = Value::opaque(SpecialFloat::new(f64::INFINITY).expect("infinite"));
    for value in [
        Value::Int(-1),
        Value::Int(i64::MIN),
        Value::from_f64(5.0),
        Value::from("5"),
        Value::Null,
        special,
    ] {
        assert_eq!(uint64::from_value(&value), None, "{value:?}");
    }
}
