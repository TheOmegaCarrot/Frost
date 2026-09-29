//! `ValueMap`, the mutable form of a Frost Map, through its public API.
//!
//! A `ValueMap` changes its internal form as it grows, so every behavior is checked
//! at a spread of sizes, small and large, and across the growth itself. The sizes
//! do not name the point where the form changes, so the tests hold wherever it is.

use std::collections::{BTreeMap, BTreeSet};

use frost_runtime::{FrostFloat, MapKey, Value, ValueMap, value_map::Entry};

/// Sizes to check every behavior at: empty, small, and well past small.
const SIZES: [i64; 10] = [0, 1, 2, 7, 8, 9, 10, 16, 17, 100];

fn key(i: i64) -> MapKey {
    MapKey::from(format!("k{i}"))
}

/// A Map of `size` entries, `k{i}` to `i`, inserted in descending order.
fn map_of(size: i64) -> ValueMap {
    let mut map = ValueMap::new();
    for i in (0..size).rev() {
        map.insert(key(i), Value::Int(i));
    }
    map
}

/// The Map's entries as a sorted, comparable collection.
fn contents(map: &ValueMap) -> BTreeMap<MapKey, Value> {
    map.iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// The entries `map_of(size)` holds, sorted.
fn entries_of(size: i64) -> BTreeMap<MapKey, Value> {
    (0..size).map(|i| (key(i), Value::Int(i))).collect()
}

// --- Insertion and lookup ---

#[test]
fn every_inserted_entry_is_found() {
    for size in SIZES {
        let map = map_of(size);
        assert_eq!(map.len(), size as usize, "size {size}");
        assert_eq!(map.is_empty(), size == 0, "size {size}");
        for i in 0..size {
            assert_eq!(map.get(&key(i)), Some(&Value::Int(i)), "size {size}");
            assert!(map.contains_key(&key(i)), "size {size}");
            assert_eq!(map[&key(i)], Value::Int(i), "size {size}");
            assert_eq!(
                map.get_key_value(&key(i)),
                Some((&key(i), &Value::Int(i))),
                "size {size}"
            );
        }
        assert_eq!(map.get(&key(size)), None, "size {size}");
        assert!(!map.contains_key(&key(size)), "size {size}");
    }
}

#[test]
fn inserting_a_present_key_replaces_its_value() {
    for size in SIZES.into_iter().filter(|&size| size > 0) {
        let mut map = map_of(size);
        let old = map.insert(key(0), Value::from("new"));
        assert_eq!(old, Some(Value::Int(0)), "size {size}");
        assert_eq!(map.len(), size as usize, "size {size}");
        assert_eq!(map.get(&key(0)), Some(&Value::from("new")), "size {size}");
    }
}

#[test]
fn a_present_key_keeps_its_first_stored_form() {
    // Float `0.0` and `-0.0` are one key, told apart only by their sign.
    let positive = MapKey::Float(FrostFloat::new(0.0).unwrap());
    let negative = MapKey::Float(FrostFloat::new(-0.0).unwrap());
    let stored_sign = |map: &ValueMap| match map.get_key_value(&positive) {
        Some((MapKey::Float(float), _)) => float.get().is_sign_negative(),
        other => panic!("a Float key, not {other:?}"),
    };
    for size in SIZES {
        let mut inserted = map_of(size);
        inserted.insert(positive.clone(), Value::Int(1));
        inserted.insert(negative.clone(), Value::Int(2));
        assert!(!stored_sign(&inserted), "size {size}: insert keeps 0.0");
        assert_eq!(inserted[&positive], Value::Int(2), "size {size}: insert");
        assert_eq!(inserted.len(), size as usize + 1, "size {size}: insert");

        let collected: ValueMap = map_of(size)
            .into_iter()
            .chain([
                (negative.clone(), Value::Int(1)),
                (positive.clone(), Value::Int(2)),
            ])
            .collect();
        assert!(stored_sign(&collected), "size {size}: collect keeps -0.0");
        assert_eq!(collected[&positive], Value::Int(2), "size {size}: collect");

        let mut extended = map_of(size);
        extended.extend([
            (negative.clone(), Value::Int(1)),
            (positive.clone(), Value::Int(2)),
        ]);
        assert!(stored_sign(&extended), "size {size}: extend keeps -0.0");
        assert_eq!(extended[&positive], Value::Int(2), "size {size}: extend");
    }
}

#[test]
fn inserting_a_new_key_returns_nothing() {
    for size in SIZES {
        let mut map = map_of(size);
        assert_eq!(map.insert(key(size), Value::Null), None, "size {size}");
        assert_eq!(map.len(), size as usize + 1, "size {size}");
    }
}

#[test]
fn keys_of_every_type_are_distinct() {
    let float = FrostFloat::new(1.0).unwrap();
    for size in SIZES {
        let mut map = map_of(size);
        let keys = [
            MapKey::from(1i64),
            MapKey::from(float),
            MapKey::from(true),
            MapKey::from("1"),
            MapKey::from(b"1".to_vec()),
        ];
        for (index, key) in keys.iter().enumerate() {
            map.insert(key.clone(), Value::Int(index as i64));
        }
        for (index, key) in keys.iter().enumerate() {
            assert_eq!(map.get(key), Some(&Value::Int(index as i64)), "size {size}");
        }
        assert_eq!(map.len(), size as usize + keys.len(), "size {size}");
    }
}

#[test]
#[should_panic(expected = "no entry found for key")]
fn indexing_a_missing_key_panics() {
    let _ = map_of(3)[&key(3)];
}

// --- Removal ---

#[test]
fn removing_every_entry_one_by_one() {
    for size in SIZES {
        let mut map = map_of(size);
        for i in 0..size {
            assert_eq!(map.remove(&key(i)), Some(Value::Int(i)), "size {size}");
            assert!(!map.contains_key(&key(i)), "size {size}");
            assert_eq!(map.len(), (size - i - 1) as usize, "size {size}");
            let remaining: BTreeMap<_, _> =
                (i + 1..size).map(|j| (key(j), Value::Int(j))).collect();
            assert_eq!(contents(&map), remaining, "size {size}");
        }
        assert!(map.is_empty(), "size {size}");
    }
}

#[test]
fn removing_a_missing_key_returns_nothing() {
    for size in SIZES {
        let mut map = map_of(size);
        assert_eq!(map.remove(&key(size)), None, "size {size}");
        assert_eq!(map.remove_entry(&key(size)), None, "size {size}");
        assert_eq!(map.len(), size as usize, "size {size}");
    }
}

#[test]
fn remove_entry_returns_the_key_too() {
    for size in SIZES.into_iter().filter(|&size| size > 0) {
        let mut map = map_of(size);
        assert_eq!(
            map.remove_entry(&key(0)),
            Some((key(0), Value::Int(0))),
            "size {size}"
        );
    }
}

#[test]
fn retain_keeps_only_what_it_accepts() {
    for size in SIZES {
        let mut map = map_of(size);
        map.retain(|_, value| matches!(value, Value::Int(i) if *i % 2 == 0));
        let evens: BTreeMap<_, _> = entries_of(size)
            .into_iter()
            .filter(|(_, value)| matches!(value, Value::Int(i) if i % 2 == 0))
            .collect();
        assert_eq!(contents(&map), evens, "size {size}");
    }
}

#[test]
fn retain_may_change_the_values_it_keeps() {
    for size in SIZES {
        let mut map = map_of(size);
        map.retain(|_, value| {
            *value = Value::Null;
            true
        });
        assert!(
            map.values().all(|value| *value == Value::Null),
            "size {size}"
        );
        assert_eq!(map.len(), size as usize, "size {size}");
    }
}

#[test]
fn a_cleared_map_is_empty_and_reusable() {
    for size in SIZES {
        let mut map = map_of(size);
        map.clear();
        assert!(map.is_empty(), "size {size}");
        assert_eq!(map.get(&key(0)), None, "size {size}");
        map.extend(entries_of(size));
        assert_eq!(contents(&map), entries_of(size), "size {size}");
    }
}

// --- Mutation in place ---

/// Add `amount` to `value`, an Int.
fn add(value: &mut Value, amount: i64) {
    let Value::Int(i) = value else {
        panic!("an Int, not {value:?}");
    };
    *i += amount;
}

#[test]
fn values_can_be_changed_in_place() {
    // Each way in adds its own amount, so the total shows that every one reached
    // every value.
    for size in SIZES {
        let mut map = map_of(size);
        map.values_mut().for_each(|value| add(value, 1));
        map.iter_mut().for_each(|(_, value)| add(value, 10));
        for (_, value) in &mut map {
            add(value, 100);
        }
        for i in 0..size {
            add(map.get_mut(&key(i)).unwrap(), 1000);
        }
        for i in 0..size {
            assert_eq!(map[&key(i)], Value::Int(i + 1111), "size {size}");
        }
        assert_eq!(map.get_mut(&key(size)), None, "size {size}");
    }
}

#[test]
fn changing_values_keeps_the_order() {
    for size in SIZES {
        let mut map = map_of(size);
        let keys_before: Vec<MapKey> = map.keys().cloned().collect();
        map.values_mut().for_each(|value| *value = Value::Null);
        map.iter_mut()
            .for_each(|(_, value)| *value = Value::Bool(true));
        for i in 0..size {
            map.insert(key(i), Value::Int(-i));
        }
        let keys_after: Vec<MapKey> = map.keys().cloned().collect();
        assert_eq!(keys_after, keys_before, "size {size}");
    }
}

// --- The entry API ---

#[test]
fn a_vacant_entry_inserts() {
    for size in SIZES {
        let mut map = map_of(size);
        match map.entry(key(size)) {
            Entry::Occupied(_) => panic!("size {size}: the key is new"),
            Entry::Vacant(entry) => {
                assert_eq!(entry.key(), &key(size), "size {size}");
                *entry.insert(Value::Int(size)) = Value::Int(-1);
            }
        }
        assert_eq!(map[&key(size)], Value::Int(-1), "size {size}");
        assert_eq!(map.len(), size as usize + 1, "size {size}");
    }
}

#[test]
fn a_vacant_entry_can_give_its_key_back() {
    for size in SIZES {
        let mut map = map_of(size);
        let Entry::Vacant(entry) = map.entry(key(size)) else {
            panic!("size {size}: the key is new");
        };
        assert_eq!(entry.into_key(), key(size), "size {size}");
        assert_eq!(map.len(), size as usize, "size {size}: nothing inserted");
    }
}

#[test]
fn an_occupied_entry_reads_changes_and_removes() {
    for size in SIZES.into_iter().filter(|&size| size > 0) {
        let mut map = map_of(size);
        let Entry::Occupied(mut entry) = map.entry(key(0)) else {
            panic!("size {size}: the key is present");
        };
        assert_eq!(entry.key(), &key(0), "size {size}");
        assert_eq!(entry.get(), &Value::Int(0), "size {size}");
        *entry.get_mut() = Value::Int(5);
        assert_eq!(entry.insert(Value::Int(6)), Value::Int(5), "size {size}");
        assert_eq!(entry.remove_entry(), (key(0), Value::Int(6)), "size {size}");
        assert!(!map.contains_key(&key(0)), "size {size}");
        assert_eq!(map.len(), size as usize - 1, "size {size}");

        if size > 1 {
            let Entry::Occupied(entry) = map.entry(key(size - 1)) else {
                panic!("size {size}: the key is present");
            };
            *entry.into_mut() = Value::Null;
            assert_eq!(map[&key(size - 1)], Value::Null, "size {size}");
        }
    }
}

#[test]
fn entry_defaults_apply_only_when_vacant() {
    for size in SIZES.into_iter().filter(|&size| size > 0) {
        let mut map = map_of(size);
        assert_eq!(
            *map.entry(key(0)).or_insert(Value::Null),
            Value::Int(0),
            "size {size}"
        );
        assert_eq!(
            *map.entry(key(0))
                .or_insert_with(|| panic!("size {size}: not called for a present key")),
            Value::Int(0),
            "size {size}"
        );
        assert_eq!(
            *map.entry(key(size)).or_insert(Value::Int(1)),
            Value::Int(1),
            "size {size}"
        );
        assert_eq!(
            *map.entry(key(size + 1)).or_insert_with(|| Value::Int(2)),
            Value::Int(2),
            "size {size}"
        );
        assert_eq!(
            *map.entry(key(size + 2)).or_default(),
            Value::Null,
            "size {size}"
        );
        assert_eq!(map.len(), size as usize + 3, "size {size}");
    }
}

#[test]
fn or_insert_with_key_gives_the_default_the_entry_key() {
    for size in SIZES {
        let mut map = map_of(size);
        let inserted = map
            .entry(key(size))
            .or_insert_with_key(|key| Value::from(format!("{key:?}")));
        assert_eq!(
            *inserted,
            Value::from(format!("{:?}", key(size))),
            "size {size}: vacant"
        );
        assert_eq!(map.len(), size as usize + 1, "size {size}");

        if size > 0 {
            let present = map
                .entry(key(0))
                .or_insert_with_key(|_| panic!("size {size}: not called for a present key"));
            assert_eq!(*present, Value::Int(0), "size {size}: occupied");
        }
    }
}

#[test]
fn and_modify_changes_only_a_present_value() {
    for size in SIZES.into_iter().filter(|&size| size > 0) {
        let mut map = map_of(size);
        map.entry(key(0))
            .and_modify(|value| *value = Value::from("modified"))
            .or_insert(Value::Null);
        map.entry(key(size))
            .and_modify(|_| panic!("size {size}: not called for a missing key"))
            .or_insert(Value::from("inserted"));
        assert_eq!(map[&key(0)], Value::from("modified"), "size {size}");
        assert_eq!(map[&key(size)], Value::from("inserted"), "size {size}");
        assert_eq!(map.entry(key(0)).key(), &key(0), "size {size}");
    }
}

#[test]
fn counting_with_entries_grows_the_map_correctly() {
    // Each key is seen `i + 1` times, interleaved, as a histogram sees them.
    let mut map = ValueMap::new();
    for round in 0..30 {
        for i in round..30 {
            let count = map.entry(key(i)).or_insert(Value::Int(0));
            if let Value::Int(count) = count {
                *count += 1;
            }
        }
    }
    let counts: BTreeMap<_, _> = (0..30).map(|i| (key(i), Value::Int(i + 1))).collect();
    assert_eq!(contents(&map), counts);
}

// --- Iteration ---

#[test]
fn iteration_yields_every_entry_exactly_once() {
    for size in SIZES {
        let map = map_of(size);
        let entries: Vec<_> = map.iter().collect();
        assert_eq!(entries.len(), size as usize, "size {size}");
        assert_eq!(map.iter().len(), size as usize, "size {size}");
        let distinct: BTreeSet<_> = map.keys().collect();
        assert_eq!(distinct.len(), size as usize, "size {size}");
        assert_eq!(contents(&map), entries_of(size), "size {size}");
    }
}

#[test]
fn keys_values_and_entries_share_one_order() {
    for size in SIZES {
        let map = map_of(size);
        let entries: Vec<_> = map.iter().collect();
        let keys: Vec<_> = map.keys().collect();
        let values: Vec<_> = map.values().collect();
        assert_eq!(
            keys,
            entries.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            "size {size}"
        );
        assert_eq!(
            values,
            entries.iter().map(|(_, value)| *value).collect::<Vec<_>>(),
            "size {size}"
        );
    }
}

#[test]
fn every_iterator_knows_its_length() {
    for size in SIZES {
        let len = size as usize;
        let mut map = map_of(size);
        assert_eq!(map.iter().len(), len, "size {size}: iter");
        assert_eq!(map.keys().len(), len, "size {size}: keys");
        assert_eq!(map.values().len(), len, "size {size}: values");
        assert_eq!(map.iter_mut().len(), len, "size {size}: iter_mut");
        assert_eq!(map.values_mut().len(), len, "size {size}: values_mut");
        assert_eq!(map.clone().into_iter().len(), len, "size {size}: into_iter");
        assert_eq!(map.clone().into_keys().len(), len, "size {size}: into_keys");
        assert_eq!(map.into_values().len(), len, "size {size}: into_values");
    }
}

#[test]
fn every_iterator_stays_finished() {
    // Each is fused: once it has run out, it keeps returning None.
    fn assert_stays_finished(mut iter: impl Iterator, what: &str) {
        iter.by_ref().for_each(drop);
        assert!(iter.next().is_none(), "{what}: first call past the end");
        assert!(iter.next().is_none(), "{what}: second call past the end");
    }
    for size in SIZES {
        let mut map = map_of(size);
        assert_stays_finished(map.iter(), &format!("size {size}: iter"));
        assert_stays_finished(map.keys(), &format!("size {size}: keys"));
        assert_stays_finished(map.values(), &format!("size {size}: values"));
        assert_stays_finished(map.iter_mut(), &format!("size {size}: iter_mut"));
        assert_stays_finished(map.values_mut(), &format!("size {size}: values_mut"));
        assert_stays_finished(map.clone().into_iter(), &format!("size {size}: into_iter"));
        assert_stays_finished(map.clone().into_keys(), &format!("size {size}: into_keys"));
        assert_stays_finished(map.into_values(), &format!("size {size}: into_values"));
    }
}

#[test]
fn a_cloned_iterator_resumes_where_the_original_was() {
    for size in SIZES {
        let map = map_of(size);
        let mut iter = map.iter();
        iter.next();
        let rest: Vec<_> = iter.clone().collect();
        assert_eq!(rest, iter.collect::<Vec<_>>(), "size {size}: iter");

        let mut keys = map.keys();
        keys.next();
        let rest: Vec<_> = keys.clone().collect();
        assert_eq!(rest, keys.collect::<Vec<_>>(), "size {size}: keys");

        let mut values = map.values();
        values.next();
        let rest: Vec<_> = values.clone().collect();
        assert_eq!(rest, values.collect::<Vec<_>>(), "size {size}: values");
    }
}

#[test]
fn a_map_iterates_in_the_same_order_every_time() {
    for size in SIZES {
        let map = map_of(size);
        let first: Vec<_> = map.iter().collect();
        let again: Vec<_> = map.iter().collect();
        let clone = map.clone();
        let through_clone: Vec<_> = clone.iter().collect();
        assert_eq!(first, again, "size {size}");
        assert_eq!(first, through_clone, "size {size}");
    }
}

#[test]
fn owning_iterators_give_up_every_entry() {
    for size in SIZES {
        let pairs: BTreeMap<_, _> = map_of(size).into_iter().collect();
        assert_eq!(pairs, entries_of(size), "size {size}");
        assert_eq!(map_of(size).into_iter().len(), size as usize, "size {size}");

        let keys: BTreeSet<_> = map_of(size).into_keys().collect();
        assert_eq!(keys, entries_of(size).into_keys().collect(), "size {size}");

        let mut values: Vec<_> = map_of(size)
            .into_values()
            .map(|value| match value {
                Value::Int(i) => i,
                other => panic!("an Int, not {other:?}"),
            })
            .collect();
        values.sort_unstable();
        assert_eq!(values, (0..size).collect::<Vec<_>>(), "size {size}");
    }
}

#[test]
fn a_borrowed_map_iterates_in_a_for_loop() {
    let map = map_of(20);
    let mut count = 0;
    for (_, _) in &map {
        count += 1;
    }
    assert_eq!(count, 20);
}

// --- Construction ---

#[test]
fn collecting_keeps_the_last_value_of_a_repeated_key() {
    for size in SIZES {
        let map: ValueMap = (0..size)
            .map(|i| (key(i), Value::Int(-i)))
            .chain((0..size).map(|i| (key(i), Value::Int(i))))
            .collect();
        assert_eq!(contents(&map), entries_of(size), "size {size}");
    }
}

#[test]
fn extending_keeps_the_last_value_of_a_repeated_key() {
    for size in SIZES {
        let mut map = map_of(size);
        map.extend((0..size).map(|i| (key(i), Value::Int(i * 2))));
        let doubled: BTreeMap<_, _> = (0..size).map(|i| (key(i), Value::Int(i * 2))).collect();
        assert_eq!(contents(&map), doubled, "size {size}");
    }
}

#[test]
fn every_way_of_building_gives_the_same_map() {
    for size in SIZES {
        let inserted = map_of(size);
        let collected: ValueMap = entries_of(size).into_iter().collect();
        let from_tree = ValueMap::from(entries_of(size));
        let mut extended = ValueMap::new();
        extended.extend(entries_of(size));
        assert_eq!(collected, inserted, "size {size}");
        assert_eq!(from_tree, inserted, "size {size}");
        assert_eq!(extended, inserted, "size {size}");
    }
    let from_array = ValueMap::from([(key(1), Value::Int(1)), (key(0), Value::Int(0))]);
    assert_eq!(from_array, map_of(2));
}

#[test]
fn a_new_or_default_map_is_empty() {
    for map in [ValueMap::new(), ValueMap::default()] {
        assert!(map.is_empty());
        assert_eq!(map.iter().count(), 0);
    }
}

// --- Equality and formatting ---

#[test]
fn equality_does_not_depend_on_order() {
    for size in SIZES {
        let descending = map_of(size);
        let ascending: ValueMap = entries_of(size).into_iter().collect();
        assert_eq!(descending, ascending, "size {size}");
    }
}

#[test]
fn equality_does_not_depend_on_how_the_map_got_its_entries() {
    // Grown large, then shrunk back: equal to a Map built at the smaller size.
    for size in SIZES {
        let mut shrunk = map_of(100);
        shrunk.retain(|key, _| entries_of(size).contains_key(key));
        assert_eq!(shrunk, map_of(size), "size {size}");
    }
}

#[test]
fn maps_with_different_entries_are_unequal() {
    for size in SIZES.into_iter().filter(|&size| size > 0) {
        let mut changed = map_of(size);
        changed.insert(key(0), Value::Null);
        assert_ne!(changed, map_of(size), "size {size}: a different value");

        let mut smaller = map_of(size);
        smaller.remove(&key(0));
        assert_ne!(smaller, map_of(size), "size {size}: a missing entry");
        assert_ne!(map_of(size), smaller, "size {size}: an extra entry");

        let mut renamed = map_of(size);
        renamed.remove(&key(0));
        renamed.insert(key(size), Value::Int(0));
        assert_ne!(renamed, map_of(size), "size {size}: a different key");
    }
}

#[test]
fn debug_formats_like_a_map() {
    let map = ValueMap::from([(key(1), Value::Int(1))]);
    assert_eq!(
        format!("{map:?}"),
        format!("{{{:?}: {:?}}}", key(1), Value::Int(1))
    );
    assert_eq!(format!("{:?}", ValueMap::new()), "{}");
}

#[test]
fn borrowing_iterators_debug_format_what_they_have_left() {
    for size in SIZES {
        let map = map_of(size);
        let mut iter = map.iter();
        iter.next();
        let rest: Vec<_> = iter.clone().collect();
        assert_eq!(
            format!("{iter:?}"),
            format!("{rest:?}"),
            "size {size}: iter"
        );

        let keys: Vec<_> = map.keys().collect();
        assert_eq!(
            format!("{:?}", map.keys()),
            format!("{keys:?}"),
            "size {size}: keys"
        );

        let values: Vec<_> = map.values().collect();
        assert_eq!(
            format!("{:?}", map.values()),
            format!("{values:?}"),
            "size {size}: values"
        );
    }
}

#[test]
fn entries_debug_format_their_contents() {
    for size in SIZES.into_iter().filter(|&size| size > 0) {
        let mut map = map_of(size);
        let occupied = format!("{:?}", map.entry(key(0)));
        assert!(
            occupied.contains(&format!("{:?}", key(0))) && occupied.contains("Int(0)"),
            "size {size}: {occupied}"
        );
        let vacant = format!("{:?}", map.entry(key(size)));
        assert!(
            vacant.contains(&format!("{:?}", key(size))),
            "size {size}: {vacant}"
        );
    }
}
