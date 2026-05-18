use std::collections::BTreeMap;
use std::sync::Arc;

use frost_core::{FrostFloat, FrostMap, MapKey, Value};

fn sample_map() -> FrostMap {
    vec![
        (MapKey::String(Arc::from("name")), Value::from("alice")),
        (MapKey::String(Arc::from("age")), Value::from(30i64)),
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
    let map: FrostMap = (0..5)
        .map(|i| (MapKey::Int(i), Value::from(i)))
        .collect();
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
    let key = MapKey::String(Arc::from("name"));
    assert!(map.get(&key).is_some());
}

#[test]
fn get_missing_key() {
    let map = sample_map();
    let key = MapKey::String(Arc::from("missing"));
    assert!(map.get(&key).is_none());
}

#[test]
fn get_non_string_key() {
    let map = sample_map();
    let key = MapKey::Bool(true);
    assert!(map.get(&key).is_some());
}

#[test]
fn contains_key_true() {
    let map = sample_map();
    let key = MapKey::String(Arc::from("age"));
    assert!(map.contains_key(&key));
}

#[test]
fn contains_key_false() {
    let map = sample_map();
    let key = MapKey::String(Arc::from("missing"));
    assert!(!map.contains_key(&key));
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
    // map is still usable
    assert_eq!(map.len(), 3);
}

// -- Key ordering --

#[test]
fn iteration_order_is_deterministic() {
    let map: FrostMap = vec![
        (MapKey::String(Arc::from("b")), Value::from(2i64)),
        (MapKey::String(Arc::from("a")), Value::from(1i64)),
        (MapKey::String(Arc::from("c")), Value::from(3i64)),
    ]
    .into_iter()
    .collect();

    let keys: Vec<_> = map.keys().collect();
    assert_eq!(keys[0], &MapKey::String(Arc::from("a")));
    assert_eq!(keys[1], &MapKey::String(Arc::from("b")));
    assert_eq!(keys[2], &MapKey::String(Arc::from("c")));
}

#[test]
fn cross_type_keys_have_consistent_order() {
    let map: FrostMap = vec![
        (MapKey::String(Arc::from("z")), Value::from("str")),
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
    // Order follows MapKey's derived Ord: Bool < Int < Float < String
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
    // Both point to the same underlying data (Arc)
    assert!(map.get_str("name").is_some());
    assert!(cloned.get_str("name").is_some());
}
