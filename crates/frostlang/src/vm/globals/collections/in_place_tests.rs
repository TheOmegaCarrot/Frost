//! White-box tests that path edits and `deep_merge` edit a uniquely owned
//! structure in place at every level.
//!
//! A copy and an edit in place give equal results, and a structure copied from
//! one that is then dropped ends up as unshared as the original, so no Frost
//! script can tell them apart. Each test instead compares where each level's
//! contents live before and after.

use std::sync::Arc;

use crate::{FrostMap, MapKey, Value, ValueMap};

use crate::vm::globals::collections::{edit_path, merge_deeply};

/// Where an Array or Map keeps its contents.
#[derive(Debug, PartialEq)]
struct Location {
    /// The allocation its holders share.
    shared: *const (),
    /// For an Array, the buffer of its elements within that allocation.
    elements: Option<*const Value>,
}

/// Where `value`, an Array or Map, keeps its contents.
fn location(value: &Value) -> Location {
    match value {
        Value::Array(array) => Location {
            shared: Arc::as_ptr(&array.inner).cast(),
            elements: Some(array.as_slice().as_ptr()),
        },
        Value::Map(map) => Location {
            shared: Arc::as_ptr(&map.inner).cast(),
            elements: None,
        },
        other => panic!("only an Array or Map has contents to locate, not {other:?}"),
    }
}

/// The location of `structure` and of each structure along `keys` within it,
/// stopping before the place at the end of the path.
fn locations_along(structure: &Value, keys: &[MapKey]) -> Vec<Location> {
    let mut locations = vec![location(structure)];
    let mut current = structure;
    for key in &keys[..keys.len() - 1] {
        current = match (current, key) {
            (Value::Map(map), key) => map.get(key),
            (Value::Array(array), MapKey::Int(i)) => array.frost_get(*i),
            _ => None,
        }
        .unwrap_or_else(|| {
            panic!("the path should lead through {structure:?}, but {key:?} does not")
        });
        locations.push(location(current));
    }
    locations
}

/// `{a: [{[0]: 0, ..., [39]: 39, ...entries}], z: {y: 2}}`: every level a
/// structure of its own, the innermost Map too large for the compact form.
fn nested(entries: Vec<(&str, Value)>) -> Value {
    let innermost: ValueMap = (0..40)
        .map(|n| (MapKey::Int(n), Value::from(n)))
        .chain(
            entries
                .into_iter()
                .map(|(key, value)| (MapKey::from(key), value)),
        )
        .collect();
    Value::map([
        ("a", Value::array([Value::from(innermost)])),
        ("z", Value::map([("y", Value::from(2))])),
    ])
}

/// [`nested`] with `b` holding an Array of its own.
fn original() -> Value {
    nested(vec![("b", Value::array([1]))])
}

/// Paths into [`original`], each with the structure setting 9 there makes: one
/// to the `b` the innermost Map holds, and one to a key it lacks, which the
/// edit adds.
fn paths() -> [([MapKey; 3], Value); 2] {
    [
        (
            [MapKey::from("a"), MapKey::Int(0), MapKey::from("b")],
            nested(vec![("b", Value::from(9))]),
        ),
        (
            [MapKey::from("a"), MapKey::Int(0), MapKey::from("new")],
            nested(vec![("b", Value::array([1])), ("new", Value::from(9))]),
        ),
    ]
}

#[test]
fn a_path_edit_keeps_every_level_of_a_unique_structure_where_it_was() {
    for (path, expected) in paths() {
        let structure = original();
        let before = locations_along(&structure, &path);
        let edited = edit_path("dig_set", structure, &path, |_| Ok(Value::from(9)))
            .expect("the path leads through the structure");
        assert_eq!(
            edited, expected,
            "the edit should set the place at {path:?}"
        );
        assert_eq!(
            locations_along(&edited, &path),
            before,
            "each level along {path:?} of a uniquely owned structure should be edited \
             where it was, not copied"
        );
    }
}

#[test]
fn a_path_edit_copies_every_level_of_a_shared_structure() {
    for (path, _) in paths() {
        let structure = original();
        let before = locations_along(&structure, &path);
        // Held twice, the top level must be copied; beneath it, each level is
        // shared by the original and the copy, so each must be copied as well.
        let kept = structure.clone();
        let edited = edit_path("dig_set", structure, &path, |_| Ok(Value::from(9)))
            .expect("the path leads through the structure");
        let after = locations_along(&edited, &path);
        for (level, (before, after)) in before.iter().zip(&after).enumerate() {
            assert_ne!(
                before.shared, after.shared,
                "level {level} along {path:?}, shared, should have been copied"
            );
        }
        assert_eq!(
            kept,
            original(),
            "the structure held elsewhere should be unchanged"
        );
    }
}

/// `{a: {b: {[key]: value}}}`, as a Map.
fn nested_map(key: &str, value: i64) -> FrostMap {
    Value::map([(
        "a",
        Value::map([("b", Value::map([(key, Value::from(value))]))]),
    )])
    .try_into_map()
    .expect("built as a Map")
}

#[test]
fn deep_merge_keeps_every_merged_level_of_a_unique_left_side_where_it_was() {
    let left = nested_map("c", 1);
    let path = [MapKey::from("a"), MapKey::from("b"), MapKey::from("c")];
    let before = locations_along(&Value::Map(left.clone()), &path);
    let merged = Value::Map(merge_deeply(left, nested_map("d", 2)));
    assert_eq!(
        merged,
        Value::map([(
            "a",
            Value::map([(
                "b",
                Value::map([("c", Value::from(1)), ("d", Value::from(2))]),
            )]),
        )]),
        "the Maps at each level should merge"
    );
    assert_eq!(
        locations_along(&merged, &path),
        before,
        "each merged level of a uniquely owned left side should be edited where it was, not copied"
    );
}

#[test]
fn deep_merge_copies_a_shared_left_side() {
    let left = nested_map("c", 1);
    let path = [MapKey::from("a"), MapKey::from("b"), MapKey::from("c")];
    let before = locations_along(&Value::Map(left.clone()), &path);
    let kept = left.clone();
    let merged = Value::Map(merge_deeply(left, nested_map("d", 2)));
    let after = locations_along(&merged, &path);
    for (level, (before, after)) in before.iter().zip(&after).enumerate() {
        assert_ne!(
            before.shared, after.shared,
            "level {level}, shared, should have been copied"
        );
    }
    assert_eq!(
        kept,
        nested_map("c", 1),
        "the left side held elsewhere should be unchanged"
    );
}
