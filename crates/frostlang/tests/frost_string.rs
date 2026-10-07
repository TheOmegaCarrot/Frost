//! `FrostString`, the payload of a String value, through its public API.

use std::collections::HashSet;
use std::sync::Arc;

use frostlang::{FrostString, MapKey, Value};

#[test]
fn it_is_the_text_it_was_made_from() {
    let text = FrostString::from("snow");
    assert_eq!(text.as_str(), "snow");
    assert_eq!(&*text, "snow", "it dereferences to the text");
    assert_eq!(text.len(), 4, "`str` methods apply through deref");
    assert_eq!(text, "snow");
    assert_eq!(text, *"snow");
}

#[test]
fn every_way_of_making_one_gives_the_same_text() {
    let expected = FrostString::from("ice");
    for (how, made) in [
        ("&str", FrostString::from("ice")),
        ("String", FrostString::from(String::from("ice"))),
        ("Box<str>", FrostString::from(Box::<str>::from("ice"))),
        ("Arc<str>", FrostString::from(Arc::<str>::from("ice"))),
    ] {
        assert_eq!(made, expected, "{how}");
    }
}

#[test]
fn it_gives_back_its_text() {
    let text = FrostString::from("melt");
    assert_eq!(String::from(text.clone()), "melt");
    assert_eq!(&*Arc::<str>::from(text), "melt");
}

#[test]
fn a_clone_shares_the_text() {
    let text = FrostString::from("shared");
    let clone = text.clone();
    assert!(std::ptr::eq(text.as_str(), clone.as_str()));
}

#[test]
fn it_orders_as_its_text_does() {
    let mut texts = ["b", "", "ab", "a"].map(FrostString::from);
    texts.sort();
    assert_eq!(texts, ["", "a", "ab", "b"].map(FrostString::from));
}

#[test]
fn it_is_found_by_its_text_in_a_set() {
    // `Borrow<str>` with a matching hash: a set of them answers for a `&str`.
    let set: HashSet<FrostString> = ["a", "b"].into_iter().map(FrostString::from).collect();
    assert!(set.contains("a"));
    assert!(!set.contains("c"));
}

#[test]
fn it_formats_as_its_text() {
    let text = FrostString::from("say \"hi\"");
    assert_eq!(text.to_string(), "say \"hi\"", "Display is the text");
    assert_eq!(
        format!("{text:?}"),
        r#""say \"hi\"""#,
        "Debug is the text's"
    );
}

#[test]
fn it_serializes_as_its_text() {
    let text = FrostString::from("frost");
    assert_eq!(serde_json::to_string(&text).unwrap(), r#""frost""#);
    let back: FrostString = serde_json::from_str(r#""frost""#).unwrap();
    assert_eq!(back, text);
}

#[test]
fn it_is_the_payload_of_a_string_value_and_key() {
    let Value::String(text) = Value::from("v") else {
        panic!("a String Value");
    };
    assert_eq!(text, "v");
    assert_eq!(Value::from(text.clone()), Value::from("v"));
    assert_eq!(MapKey::from(text), MapKey::from("v"));
}
