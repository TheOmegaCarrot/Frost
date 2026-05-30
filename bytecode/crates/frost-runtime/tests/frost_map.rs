use std::collections::BTreeMap;
use std::sync::Arc;

use frost_runtime::{FrostFloat, FrostMap, MapKey, Value};

fn str_key(s: &str) -> MapKey {
    MapKey::String(Arc::from(s.as_bytes()))
}

fn sample_map() -> FrostMap {
    vec![
        (str_key("name"), Value::from("alice")),
        (str_key("age"), Value::from(30i64)),
        (MapKey::Bool(true), Value::from("yes")),
    ]
    .into_iter()
    .collect()
}

// -- Construction --

#[test]
fn empty_map() {
    let map = FrostMap::empty();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

#[test]
fn default_is_empty() {
    let map = FrostMap::default();
    assert!(map.is_empty());
}

#[test]
fn from_btreemap() {
    let mut bt = BTreeMap::new();
    bt.insert(MapKey::Int(1), Value::from("one"));
    bt.insert(MapKey::Int(2), Value::from("two"));
    let map = FrostMap::from(bt);
    assert_eq!(map.len(), 2);
}

#[test]
fn from_arc_btreemap() {
    let mut bt = BTreeMap::new();
    bt.insert(MapKey::Int(1), Value::from("one"));
    let arc = Arc::new(bt);
    let map = FrostMap::from(arc);
    assert_eq!(map.len(), 1);
    assert!(map.get(&MapKey::Int(1)).is_some());
}

#[test]
fn collect_from_iterator() {
    let map: FrostMap = (0..5).map(|i| (MapKey::Int(i), Value::from(i))).collect();
    assert_eq!(map.len(), 5);
}

#[test]
fn collect_duplicate_keys_last_wins() {
    let map: FrostMap = vec![
        (MapKey::Int(1), Value::from("first")),
        (MapKey::Int(1), Value::from("second")),
    ]
    .into_iter()
    .collect();
    assert_eq!(map.len(), 1);
}

// -- Access --

#[test]
fn get_existing_key() {
    let map = sample_map();
    assert!(map.get(&str_key("name")).is_some());
}

#[test]
fn get_missing_key() {
    let map = sample_map();
    assert!(map.get(&str_key("missing")).is_none());
}

#[test]
fn get_non_string_key() {
    let map = sample_map();
    assert!(map.get(&MapKey::Bool(true)).is_some());
}

#[test]
fn contains_key_true() {
    let map = sample_map();
    assert!(map.contains_key(&str_key("age")));
}

#[test]
fn contains_key_false() {
    let map = sample_map();
    assert!(!map.contains_key(&str_key("missing")));
}

// -- String convenience access --

#[test]
fn get_str_existing() {
    let map = sample_map();
    assert!(map.get_str("name").is_some());
}

#[test]
fn get_str_missing() {
    let map = sample_map();
    assert!(map.get_str("missing").is_none());
}

#[test]
fn get_str_does_not_find_non_string_keys() {
    let map: FrostMap = vec![(MapKey::Int(42), Value::from("val"))]
        .into_iter()
        .collect();
    assert!(map.get_str("42").is_none());
}

// -- Size --

#[test]
fn len() {
    let map = sample_map();
    assert_eq!(map.len(), 3);
}

#[test]
fn is_empty_false() {
    let map = sample_map();
    assert!(!map.is_empty());
}

// -- Iteration --

#[test]
fn iter_yields_all_pairs() {
    let map = sample_map();
    let pairs: Vec<_> = map.iter().collect();
    assert_eq!(pairs.len(), 3);
}

#[test]
fn keys_yields_all_keys() {
    let map = sample_map();
    let keys: Vec<_> = map.keys().collect();
    assert_eq!(keys.len(), 3);
}

#[test]
fn values_yields_all_values() {
    let map = sample_map();
    let values: Vec<_> = map.values().collect();
    assert_eq!(values.len(), 3);
}

#[test]
fn for_loop_borrows() {
    let map = sample_map();
    let mut count = 0;
    for (_, _) in &map {
        count += 1;
    }
    assert_eq!(count, 3);
    assert_eq!(map.len(), 3);
}

// -- Key ordering --

#[test]
fn iteration_order_is_deterministic() {
    let map: FrostMap = vec![
        (str_key("b"), Value::from(2i64)),
        (str_key("a"), Value::from(1i64)),
        (str_key("c"), Value::from(3i64)),
    ]
    .into_iter()
    .collect();

    let keys: Vec<_> = map.keys().collect();
    assert_eq!(keys[0], &str_key("a"));
    assert_eq!(keys[1], &str_key("b"));
    assert_eq!(keys[2], &str_key("c"));
}

#[test]
fn cross_type_keys_have_consistent_order() {
    let map: FrostMap = vec![
        (str_key("z"), Value::from("str")),
        (MapKey::Bool(false), Value::from("bool")),
        (MapKey::Int(99), Value::from("int")),
        (
            MapKey::Float(FrostFloat::new(1.5).unwrap()),
            Value::from("float"),
        ),
    ]
    .into_iter()
    .collect();

    let keys: Vec<_> = map.keys().collect();
    assert!(matches!(keys[0], MapKey::Bool(_)));
    assert!(matches!(keys[1], MapKey::Int(_)));
    assert!(matches!(keys[2], MapKey::Float(_)));
    assert!(matches!(keys[3], MapKey::String(_)));
}

// -- Clone semantics --

#[test]
fn clone_shares_data() {
    let map = sample_map();
    let cloned = map.clone();
    assert_eq!(cloned.len(), map.len());
    assert!(map.get_str("name").is_some());
    assert!(cloned.get_str("name").is_some());
}
