use std::sync::Arc;

use frostlang::bytecode::{CompiledFunction, FormatVersion};
use frostlang::{
    Arity, Closure, FrostArray, FrostFloat, FrostMap, FrostOpaque, MapKey, NativeFunction,
    OpaqueHandle, Value, ValueMap,
};

use crate::deep;
use crate::script::Script;

/// A minimal opaque payload for identity-equality tests.
#[derive(Debug)]
struct Marker(i64);

impl FrostOpaque for Marker {
    fn type_name() -> &'static str {
        "Marker"
    }

    fn try_to_string(&self) -> Option<String> {
        Some(format!("marker {}", self.0))
    }
}

fn str_key(s: &str) -> MapKey {
    MapKey::from(s)
}

// -- Same-type equality --

#[test]
fn null_equals_null() {
    assert_eq!(Value::Null, Value::Null);
}

#[test]
fn bool_equality() {
    assert_eq!(Value::from(true), Value::from(true));
    assert_eq!(Value::from(false), Value::from(false));
    assert_ne!(Value::from(true), Value::from(false));
}

#[test]
fn int_equality() {
    assert_eq!(Value::from(42i64), Value::from(42i64));
    assert_ne!(Value::from(42i64), Value::from(43i64));
}

#[test]
fn float_equality() {
    let a: Value = 3.14.try_into().unwrap();
    let b: Value = 3.14.try_into().unwrap();
    let c: Value = 2.71.try_into().unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn string_equality() {
    assert_eq!(Value::from("hello"), Value::from("hello"));
    assert_ne!(Value::from("hello"), Value::from("world"));
}

#[test]
fn empty_string_equals_empty_string() {
    assert_eq!(Value::from(""), Value::from(""));
}

// -- Cross-type inequality --

#[test]
fn int_not_equal_to_float() {
    let i = Value::from(3i64);
    let f: Value = 3.0.try_into().unwrap();
    assert_ne!(i, f);
}

#[test]
fn null_not_equal_to_false() {
    assert_ne!(Value::Null, Value::from(false));
}

#[test]
fn null_not_equal_to_zero() {
    assert_ne!(Value::Null, Value::from(0i64));
}

#[test]
fn null_not_equal_to_empty_string() {
    assert_ne!(Value::Null, Value::from(""));
}

#[test]
fn bool_not_equal_to_int() {
    assert_ne!(Value::from(true), Value::from(1i64));
    assert_ne!(Value::from(false), Value::from(0i64));
}

// -- Array equality --

#[test]
fn empty_arrays_equal() {
    let a = Value::from(FrostArray::from(vec![]));
    let b = Value::from(FrostArray::from(vec![]));
    assert_eq!(a, b);
}

#[test]
fn arrays_same_elements() {
    let a = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]));
    let b = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]));
    assert_eq!(a, b);
}

#[test]
fn arrays_different_elements() {
    let a = Value::from(FrostArray::from(vec![Value::from(1i64)]));
    let b = Value::from(FrostArray::from(vec![Value::from(2i64)]));
    assert_ne!(a, b);
}

#[test]
fn arrays_different_lengths() {
    let a = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]));
    let b = Value::from(FrostArray::from(vec![Value::from(1i64)]));
    assert_ne!(a, b);
}

#[test]
fn nested_array_equality() {
    let inner = FrostArray::from(vec![Value::from(1i64)]);
    let a = Value::from(FrostArray::from(vec![Value::from(inner.clone())]));
    let b = Value::from(FrostArray::from(vec![Value::from(inner)]));
    assert_eq!(a, b);
}

// -- Map equality --

#[test]
fn empty_maps_equal() {
    assert_eq!(
        Value::from(FrostMap::empty()),
        Value::from(FrostMap::empty())
    );
}

#[test]
fn maps_same_entries() {
    let a: FrostMap = vec![(str_key("x"), Value::from(1i64))]
        .into_iter()
        .collect();
    let b: FrostMap = vec![(str_key("x"), Value::from(1i64))]
        .into_iter()
        .collect();
    assert_eq!(Value::from(a), Value::from(b));
}

#[test]
fn maps_different_values() {
    let a: FrostMap = vec![(str_key("x"), Value::from(1i64))]
        .into_iter()
        .collect();
    let b: FrostMap = vec![(str_key("x"), Value::from(2i64))]
        .into_iter()
        .collect();
    assert_ne!(Value::from(a), Value::from(b));
}

#[test]
fn maps_different_keys() {
    let a: FrostMap = vec![(str_key("x"), Value::from(1i64))]
        .into_iter()
        .collect();
    let b: FrostMap = vec![(str_key("y"), Value::from(1i64))]
        .into_iter()
        .collect();
    assert_ne!(Value::from(a), Value::from(b));
}

#[test]
fn maps_different_sizes() {
    let a: FrostMap = vec![
        (str_key("x"), Value::from(1i64)),
        (str_key("y"), Value::from(2i64)),
    ]
    .into_iter()
    .collect();
    let b: FrostMap = vec![(str_key("x"), Value::from(1i64))]
        .into_iter()
        .collect();
    assert_ne!(Value::from(a), Value::from(b));
}

// -- Arc short-circuit --

#[test]
fn same_arc_array_is_equal() {
    let arr = FrostArray::from(vec![Value::from(1i64)]);
    let a = Value::from(arr.clone());
    let b = Value::from(arr);
    assert_eq!(a, b);
}

#[test]
fn same_arc_map_is_equal() {
    let map: FrostMap = vec![(str_key("k"), Value::from(1i64))]
        .into_iter()
        .collect();
    let a = Value::from(map.clone());
    let b = Value::from(map);
    assert_eq!(a, b);
}

// -- Function / opaque identity (compared by Arc identity, never structurally) --

fn a_native() -> Arc<NativeFunction> {
    Arc::new(NativeFunction::new(
        "f",
        Arity::Exact(0),
        |_, _: &mut [Value]| Ok(Value::Null),
    ))
}

fn a_closure() -> Arc<Closure> {
    let f = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "f".to_string(),
        origin: None,
        code: Vec::new(),
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    f.assert_well_formed().into_closure().unwrap()
}

#[test]
fn native_functions_compare_by_identity() {
    let n = a_native();
    assert_eq!(Value::NativeFunction(n.clone()), Value::NativeFunction(n));
    assert_ne!(
        Value::NativeFunction(a_native()),
        Value::NativeFunction(a_native())
    );
}

#[test]
fn closures_compare_by_identity() {
    let c = a_closure();
    // The same closure equals itself.
    assert_eq!(Value::Closure(c.clone()), Value::Closure(c));
    // Distinct closures, even structurally identical, are never equal.
    assert_ne!(Value::Closure(a_closure()), Value::Closure(a_closure()));
}

#[test]
fn an_opaque_without_equals_compares_by_identity() {
    let o = OpaqueHandle::new(Marker(42));
    assert_eq!(Value::Opaque(o.clone()), Value::Opaque(o));
    // Distinct instances, even with equal payloads, are never equal.
    assert_ne!(Value::opaque(Marker(1)), Value::opaque(Marker(1)));
}

/// An opaque payload that defers `==` to its derived `Eq`.
#[derive(Debug, PartialEq, Eq)]
struct Tag(i64);

impl FrostOpaque for Tag {
    fn type_name() -> &'static str {
        "Tag"
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }

    fn equals(&self, other: &dyn FrostOpaque) -> bool {
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| self == other)
    }
}

/// An opaque payload claiming to equal anything, to show what Frost never asks it.
#[derive(Debug)]
struct EqualsAnything;

impl FrostOpaque for EqualsAnything {
    fn type_name() -> &'static str {
        "EqualsAnything"
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }

    fn equals(&self, _: &dyn FrostOpaque) -> bool {
        true
    }
}

#[test]
fn an_opaque_with_equals_compares_by_it() {
    assert_eq!(Value::opaque(Tag(1)), Value::opaque(Tag(1)));
    assert_ne!(Value::opaque(Tag(1)), Value::opaque(Tag(2)));
}

#[test]
fn opaques_compare_by_equals_inside_a_structure() {
    let holding = |n| Value::array([Value::map([("tag", Value::opaque(Tag(n)))])]);
    assert_eq!(holding(1), holding(1));
    assert_ne!(holding(1), holding(2));
}

#[test]
fn opaques_of_different_types_are_never_equal() {
    // `equals` is never asked about a payload of another type, either way round.
    assert_eq!(Value::opaque(EqualsAnything), Value::opaque(EqualsAnything));
    assert_ne!(Value::opaque(EqualsAnything), Value::opaque(Marker(1)));
    assert_ne!(Value::opaque(Marker(1)), Value::opaque(EqualsAnything));
}

#[test]
fn an_opaque_never_equals_a_non_opaque() {
    assert_ne!(Value::opaque(EqualsAnything), Value::Null);
    assert_ne!(Value::opaque(EqualsAnything), Value::from("EqualsAnything"));
}

#[test]
fn a_function_never_equals_a_non_function() {
    assert_ne!(Value::Closure(a_closure()), Value::Null);
    assert_ne!(Value::NativeFunction(a_native()), Value::from(0i64));
}

// -- Negative zero --

/// Equal values are indistinguishable, and the zeros render differently, so they are unequal
/// wherever they sit.
#[test]
fn negative_zero_does_not_equal_positive_zero() {
    let pos: Value = 0.0.try_into().unwrap();
    let neg: Value = (-0.0f64).try_into().unwrap();
    assert_ne!(pos, neg);
    assert_eq!(neg, (-0.0f64).try_into().unwrap());
    assert_ne!(Value::array([pos.clone()]), Value::array([neg.clone()]));
    assert_ne!(
        Value::map([("k", pos.clone())]),
        Value::map([("k", neg.clone())])
    );
    let keyed = |key: f64| Value::map([(FrostFloat::new(key).unwrap(), Value::Null)]);
    assert_ne!(keyed(0.0), keyed(-0.0));
}

// -- Deeply nested values --
//
// Each is compared on a small stack (see [`deep`](crate::deep)). The assertions report
// no values, since rendering one so deep is no part of what these test.

/// A level of nesting, built around the level below.
type Wrap = fn(Value) -> Value;

/// Each of the ways the deep cases nest, by name.
const SHAPES: [(&str, Wrap); 3] = [
    ("Arrays", deep::in_an_array),
    ("Maps", deep::in_a_map),
    ("Arrays and Maps in turn", |below| {
        deep::in_a_map(deep::in_an_array(below))
    }),
];

/// Asserts that `inner` and `other_inner`, which `what` describes, each nested `depth`
/// levels deep in each shape, built apart, compare as `equal`.
fn assert_nested_equality(what: &str, depth: usize, inner: Value, other_inner: Value, equal: bool) {
    assert_nested_equality_in(&SHAPES, what, depth, inner, other_inner, equal);
}

/// Asserts that `inner` and `other_inner`, which `what` describes, each nested `depth`
/// levels deep in each of `shapes`, built apart, compare as `equal`, either way round.
fn assert_nested_equality_in(
    shapes: &[(&str, Wrap)],
    what: &str,
    depth: usize,
    inner: Value,
    other_inner: Value,
    equal: bool,
) {
    deep::on_a_small_stack(|| {
        for (shape, wrap) in shapes {
            let l = deep::nested(depth, inner.clone(), wrap);
            let r = deep::nested(depth, other_inner.clone(), wrap);
            let context = format!("{what} in {shape} {depth} levels deep");
            assert_eq!(l == r, equal, "{context}: `l == r`");
            assert_eq!(r == l, equal, "{context}: `r == l`");
            assert_eq!(l != r, !equal, "{context}: `l != r`");
            assert_eq!(r != l, !equal, "{context}: `r != l`");
        }
    });
}

/// The depths around where `==` stops recursing and starts looping, and one far past.
const DEPTHS: [usize; 11] = [1, 2, 31, 32, 33, 34, 63, 64, 65, 66, deep::DEPTH];

#[test]
fn deeply_nested_values_alike_are_equal() {
    for depth in DEPTHS {
        assert_nested_equality("1 and 1", depth, Value::Int(1), Value::Int(1), true);
    }
}

#[test]
fn deeply_nested_values_differing_innermost_are_unequal() {
    for depth in DEPTHS {
        assert_nested_equality("1 and 2", depth, Value::Int(1), Value::Int(2), false);
        assert_nested_equality("1 and null", depth, Value::Int(1), Value::Null, false);
    }
}

#[test]
fn deeply_nested_values_differing_in_length_innermost_are_unequal() {
    for depth in DEPTHS {
        let one = Value::from(vec![Value::Int(1)]);
        let two = Value::from(vec![Value::Int(1), Value::Int(1)]);
        assert_nested_equality("[1] and [1, 1]", depth, one, two, false);
    }
}

#[test]
fn deeply_nested_maps_differing_in_a_key_innermost_are_unequal() {
    for depth in DEPTHS {
        let key = |name: &str| Value::Map(FrostMap::from_iter([(str_key(name), Value::Int(1))]));
        assert_nested_equality("{a: 1} and {b: 1}", depth, key("a"), key("b"), false);
    }
}

/// Values built apart, deeper than `==` recurses, that share a deep part are equal, and
/// unequal when they differ outside it.
#[test]
fn deeply_nested_values_sharing_a_deep_part_compare_by_the_rest() {
    let shared = deep::nested(deep::DEPTH, Value::Int(1), deep::in_an_array);
    let with = |other: i64| Value::from(vec![shared.clone(), Value::Int(other)]);
    assert_nested_equality("[shared, 1] and [shared, 1]", 40, with(1), with(1), true);
    assert_nested_equality("[shared, 1] and [shared, 2]", 40, with(1), with(2), false);
}

/// Which sibling of the deeper level a level of [`spine_with_siblings`] changes.
#[derive(Clone, Copy, PartialEq)]
enum Sibling {
    Before,
    After,
}

/// A value `depth` levels deep, each level holding an Int before the level below and one
/// after, in an Array or a Map. Each Int is its level's, counting from the innermost,
/// except the one `changed` names, which is `-1`.
fn spine_with_siblings(depth: usize, in_maps: bool, changed: Option<(usize, Sibling)>) -> Value {
    (0..depth).fold(Value::Null, |below, level| {
        let sibling = |which| {
            let changed = changed == Some((level, which));
            Value::Int(if changed { -1 } else { level as i64 })
        };
        let (before, after) = (sibling(Sibling::Before), sibling(Sibling::After));
        if in_maps {
            Value::Map(FrostMap::from_iter([
                (str_key("a"), before),
                (str_key("below"), below),
                (str_key("z"), after),
            ]))
        } else {
            Value::from(vec![before, below, after])
        }
    })
}

/// A difference beside the deeper level, before or after it, is found at any level: as
/// `==` recurses, as it loops, and where it takes up a level again once the one below is
/// done.
#[test]
fn deeply_nested_values_differing_beside_a_deeper_level_are_unequal() {
    let depth = deep::DEPTH;
    deep::on_a_small_stack(|| {
        for in_maps in [false, true] {
            let shape = if in_maps { "Maps" } else { "Arrays" };
            let alike = spine_with_siblings(depth, in_maps, None);
            let also_alike = spine_with_siblings(depth, in_maps, None);
            assert!(alike == also_alike, "{shape}: alike");

            for level in [
                0,
                1,
                depth / 2,
                depth - 33,
                depth - 32,
                depth - 31,
                depth - 1,
            ] {
                for (which, name) in [(Sibling::Before, "before"), (Sibling::After, "after")] {
                    let changed = spine_with_siblings(depth, in_maps, Some((level, which)));
                    let context = format!("{shape}: changed {name} the deeper level at {level}");
                    assert!(alike != changed, "{context}: `alike != changed`");
                    assert!(changed != alike, "{context}: `changed != alike`");
                }
            }
        }
    });
}

/// Functions and Opaques innermost compare as they do anywhere: by identity, or by an
/// Opaque's `equals`.
#[test]
fn deeply_nested_functions_and_opaques_compare_as_anywhere() {
    let marker = Value::Opaque(OpaqueHandle::new(Marker(1)));
    let closure = Value::Closure(a_closure());
    let native = Value::NativeFunction(a_native());
    let cases = [
        ("one Opaque", marker.clone(), marker, true),
        (
            "two Markers",
            Value::opaque(Marker(1)),
            Value::opaque(Marker(1)),
            false,
        ),
        (
            "Tags alike",
            Value::opaque(Tag(1)),
            Value::opaque(Tag(1)),
            true,
        ),
        (
            "Tags differing",
            Value::opaque(Tag(1)),
            Value::opaque(Tag(2)),
            false,
        ),
        (
            "Opaques of two types",
            Value::opaque(EqualsAnything),
            Value::opaque(Tag(1)),
            false,
        ),
        ("one closure", closure.clone(), closure, true),
        (
            "two closures",
            Value::Closure(a_closure()),
            Value::Closure(a_closure()),
            false,
        ),
        ("one native", native.clone(), native, true),
        (
            "two natives",
            Value::NativeFunction(a_native()),
            Value::NativeFunction(a_native()),
            false,
        ),
    ];
    for depth in [1, 33, deep::DEPTH] {
        for (what, inner, other_inner, equal) in &cases {
            assert_nested_equality(what, depth, inner.clone(), other_inner.clone(), *equal);
        }
    }
}

/// Maps in their large form, holding the level below among many other entries.
#[test]
fn deeply_nested_large_maps_compare_by_content() {
    let in_a_large_map: Wrap = |below| {
        let value = |k| {
            if k == 10 {
                below.clone()
            } else {
                Value::Int(k)
            }
        };
        Value::Map(FrostMap::from_iter(
            (0..20).map(|k| (MapKey::from(k), value(k))),
        ))
    };
    let shapes = [("Maps of 20 entries", in_a_large_map)];
    for depth in [1, 33, 40, 1000] {
        assert_nested_equality_in(
            &shapes,
            "1 and 1",
            depth,
            Value::Int(1),
            Value::Int(1),
            true,
        );
        assert_nested_equality_in(
            &shapes,
            "1 and 2",
            depth,
            Value::Int(1),
            Value::Int(2),
            false,
        );
    }
}

/// `==` pairs up two Maps' values by iterating both in key order, so Maps with the same
/// entries must compare equal however each got them: inserted in any order, or left with
/// them by removing others, which keeps a Map in the form it grew into. Checked both as
/// `==` recurses and as it loops.
#[test]
fn maps_are_compared_by_key_however_each_was_built() {
    let map = |keys: &mut dyn Iterator<Item = i64>, changed: Option<i64>| {
        let entries = keys.map(|k| {
            let value = if Some(k) == changed { -1 } else { k };
            (MapKey::from(k), Value::Int(value))
        });
        Value::from(ValueMap::from_iter(entries))
    };
    let shrunk = |size: i64, changed: Option<i64>| {
        let mut grown = ValueMap::from_iter((0..100).map(|k| (MapKey::from(k), Value::Int(k))));
        grown.retain(|key, _| *key < MapKey::from(size));
        if let Some(k) = changed {
            grown.insert(MapKey::from(k), Value::Int(-1));
        }
        Value::from(grown)
    };

    for size in [1, 2, 16, 17, 40] {
        let ascending = map(&mut (0..size), None);
        let built = [
            ("descending", map(&mut (0..size).rev(), None)),
            ("shrunk", shrunk(size, None)),
        ];
        let changed = [
            ("descending", map(&mut (0..size).rev(), Some(size / 2))),
            ("shrunk", shrunk(size, Some(size / 2))),
        ];
        for depth in [0, 1, 32, 33, 40] {
            for (how, other) in &built {
                let what = format!("Maps of {size} entries, one {how}");
                assert_nested_equality(&what, depth, ascending.clone(), other.clone(), true);
            }
            for (how, other) in &changed {
                let what = format!("Maps of {size} entries, one {how} with a value changed");
                assert_nested_equality(&what, depth, ascending.clone(), other.clone(), false);
            }
        }
    }
}

/// Each way a script compares values for equality.
#[test]
fn a_script_compares_deeply_nested_values() {
    let source = format!(
        r"
defn nest(v, n) -> if n == 0: v else: nest([v], n - 1)
def a = nest(1, {depth})
def b = nest(1, {depth})
def c = nest(2, {depth})
[
    a == b,
    a != c,
    match a {{ (c) => 'c', (b) => 'b', _ => 'neither' }},
    len(unique([a, b, c])),
    contains([c, b], a),
]
",
        depth = deep::DEPTH
    );
    let tail = deep::on_a_small_stack(|| Script::new(&source).run());
    assert!(
        tail == Value::from(vec![
            Value::Bool(true),
            Value::Bool(true),
            Value::from("b"),
            Value::Int(2),
            Value::Bool(true),
        ]),
        "{source}"
    );
}
