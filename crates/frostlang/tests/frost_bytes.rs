//! `FrostBytes`, the payload of a Bytes value, through its public API.

use std::collections::HashSet;
use std::sync::Arc;

use frostlang::{FrostBytes, FrostString, MapKey, Value};

#[test]
fn it_is_the_bytes_it_was_made_from() {
    let bytes = FrostBytes::from(&[1u8, 2, 3][..]);
    assert_eq!(bytes.as_slice(), [1, 2, 3]);
    assert_eq!(&*bytes, [1, 2, 3], "it dereferences to the bytes");
    assert_eq!(bytes.len(), 3, "slice methods apply through deref");
    assert_eq!(bytes, &[1u8, 2, 3][..]);
}

#[test]
fn every_way_of_making_one_gives_the_same_bytes() {
    let expected = FrostBytes::from(&[0u8, 255][..]);
    for (how, made) in [
        ("&[u8]", FrostBytes::from(&[0u8, 255][..])),
        ("[u8; N]", FrostBytes::from([0u8, 255])),
        ("Vec<u8>", FrostBytes::from(vec![0u8, 255])),
        ("Box<[u8]>", FrostBytes::from(Box::<[u8]>::from([0u8, 255]))),
        ("Arc<[u8]>", FrostBytes::from(Arc::<[u8]>::from([0u8, 255]))),
    ] {
        assert_eq!(made, expected, "{how}");
    }
}

#[test]
fn it_gives_back_its_bytes() {
    let bytes = FrostBytes::from([7u8, 8]);
    assert_eq!(Vec::from(bytes.clone()), [7, 8]);
    assert_eq!(&*Arc::<[u8]>::from(bytes), [7, 8]);
}

#[test]
fn text_becomes_its_utf8_bytes_without_a_copy() {
    let text = FrostString::from("h\u{e9}");
    let bytes = FrostBytes::from(text.clone());
    assert_eq!(bytes, "h\u{e9}".as_bytes());
    assert!(
        std::ptr::eq(bytes.as_slice().as_ptr(), text.as_ptr()),
        "the bytes share the text's storage"
    );
}

#[test]
fn it_orders_as_its_bytes_do() {
    let mut all = [vec![2u8], vec![], vec![1, 9], vec![1]].map(FrostBytes::from);
    all.sort();
    assert_eq!(
        all,
        [vec![], vec![1u8], vec![1, 9], vec![2]].map(FrostBytes::from)
    );
}

#[test]
fn it_is_found_by_its_bytes_in_a_set() {
    // `Borrow<[u8]>` with a matching hash: a set of them answers for a `&[u8]`.
    let set: HashSet<FrostBytes> = [[1u8], [2]].into_iter().map(FrostBytes::from).collect();
    assert!(set.contains(&[1u8][..]));
    assert!(!set.contains(&[3u8][..]));
}

#[test]
fn it_debug_formats_as_its_bytes() {
    assert_eq!(format!("{:?}", FrostBytes::from([1u8, 2])), "[1, 2]");
}

#[test]
fn it_is_the_payload_of_a_bytes_value_and_key() {
    let Value::Bytes(bytes) = Value::from(&[5u8][..]) else {
        panic!("a Bytes Value");
    };
    assert_eq!(bytes, &[5u8][..]);
    assert_eq!(Value::from(bytes.clone()), Value::from(&[5u8][..]));
    assert_eq!(MapKey::from(bytes), MapKey::from(&[5u8][..]));
}
