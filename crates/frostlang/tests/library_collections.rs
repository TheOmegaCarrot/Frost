//! The Collections globals, from Frost source.
//!
//! Most are natives. `index` and `dig` are generated from Frost (see the
//! runtime's `generated` module), so they index exactly as `structure[key]` does.
//!
//! The harness runs every case under every optimization permutation, so a call
//! over literals is checked both folded and at run time.

use crate::script;
use crate::unshared::unshared;

use frostlang::{FrostOpaque, Value};
use script::assertions::{Library, library_assertions};
use script::{Script, raises, run};

library_assertions!(Library::GLOBALS);

/// Assert each `(source, requires, position, got)` raises `function`'s type error,
/// where `position` is how the error names the argument, such as `argument 2`.
fn assert_type_errors(function: &str, cases: &[(&str, &str, &str, &str)]) {
    for (source, requires, position, got) in cases {
        assert_eq!(
            raises(source),
            format!("Function {function} requires {requires} as {position}, got {got}"),
            "{source:?}"
        );
    }
}

/// The set of types a Map key may have, as type errors name it.
const MAP_KEY: &str = "Bool or Int or Float or String or Bytes";

/// A Map of 40 entries, more than the compact form for small Maps holds,
/// as a Frost expression.
const LARGE_MAP: &str = "map_into(range(40), fn n -> {[n]: n * 2})";

/// `expression` run with `m` bound to [`LARGE_MAP`].
fn over_large_map(expression: &str) -> String {
    format!(
        r"
        def m = {LARGE_MAP}
        {expression}
        "
    )
}

// --- keys, values ---

#[test]
fn keys_and_values_list_a_map() {
    assert_values(&[
        ("keys({a: 1})", "['a']"),
        ("values({a: 1})", "[1]"),
        ("keys({})", "[]"),
        ("values({})", "[]"),
        // A key keeps its type.
        ("keys({[1]: 'x'})", "[1]"),
        ("keys({[x'ff']: 'x'})", "[x'ff']"),
        ("values({a: null})", "[null]"),
    ]);
}

#[test]
fn keys_and_values_list_entries_in_the_same_order() {
    let source = r"
        def m = {a: 1, b: 2, [3]: 'c', [true]: null, [1.5]: [4]}
        def ks = keys(m)
        def vs = values(m)
        [len(ks), len(vs), all(range(len(ks)), fn i -> m[ks[i]] == vs[i])]
    ";
    assert_values(&[(source, "[5, 5, true]")]);
    let source = r"
        def m = {a: 1, b: 2, [3]: 'c', [true]: null, [1.5]: [4]}
        all(['a', 'b', 3, true, 1.5], fn k -> contains(keys(m), k))
    ";
    assert_values(&[(source, "true")]);
}

#[test]
fn keys_and_values_check_their_argument() {
    for function in ["keys", "values"] {
        assert_type_errors(
            function,
            &[
                (&format!("{function}([1])"), "Map", "argument 1", "Array"),
                (&format!("{function}(null)"), "Map", "argument 1", "Null"),
            ],
        );
        assert_arity(function, "1", &[0, 2]);
    }
}

// --- map_keys, map_values ---

#[test]
fn map_keys_transforms_each_key() {
    assert_values(&[
        ("map_keys({a: 1, b: 2}, to_upper)", "{A: 1, B: 2}"),
        ("map_keys({}, to_upper)", "{}"),
        ("map_keys({abc: 1}, len)", "{[3]: 1}"),
        ("map_keys({[1]: 'x'}, fn k -> k * 10)", "{[10]: 'x'}"),
        // Keys that collide leave one entry.
        ("len(map_keys({a: 1, b: 2}, fn k -> 'same'))", "1"),
    ]);
}

#[test]
fn map_keys_requires_a_valid_key_from_its_function() {
    for (function, got) in [("fn k -> null", "Null"), ("fn k -> [k]", "Array")] {
        assert_eq!(
            raises(&format!("map_keys({{a: 1}}, {function})")),
            format!("Function map_keys requires its function to return a valid Map key, got {got}"),
            "{function}"
        );
    }
}

#[test]
fn map_keys_raises_what_its_function_raises() {
    assert_raises(&[("map_keys({a: 1}, fn k -> error('boom'))", "boom")]);
    let script = script("map_keys({a: 1, b: 2}, fn k -> { print(k); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed().len(), 1, "the first error ends the loop");
}

#[test]
fn map_values_transforms_each_value() {
    assert_values(&[
        ("map_values({a: 1, b: 2}, fn v -> v * 10)", "{a: 10, b: 20}"),
        ("map_values({}, fn v -> v)", "{}"),
        ("map_values({[1]: 'x'}, fn v -> [v])", "{[1]: ['x']}"),
        ("map_values({a: 1}, fn v -> null)", "{a: null}"),
    ]);
    assert_raises_as(&[("map_values({a: 1}, fn v -> v + 'x')", "1 + 'x'")]);
}

#[test]
fn map_keys_and_map_values_check_their_arguments() {
    for function in ["map_keys", "map_values"] {
        assert_type_errors(
            function,
            &[
                (
                    &format!("{function}([1], id)"),
                    "Map",
                    "argument 1",
                    "Array",
                ),
                (
                    &format!("{function}({{}}, 1)"),
                    "Function",
                    "argument 2",
                    "Int",
                ),
            ],
        );
        assert_arity(function, "2", &[0, 1, 3]);
    }
}

// --- to_entries, from_entries ---

#[test]
fn to_entries_lists_key_value_maps() {
    assert_values(&[
        ("to_entries({a: 1})", "[{key: 'a', value: 1}]"),
        ("to_entries({})", "[]"),
        ("to_entries({[2]: null})", "[{key: 2, value: null}]"),
        ("len(to_entries({a: 1, b: 2, c: 3}))", "3"),
    ]);
}

#[test]
fn from_entries_builds_a_map() {
    assert_values(&[
        (
            "from_entries([{key: 'a', value: 1}, {key: 'b', value: 2}])",
            "{a: 1, b: 2}",
        ),
        ("from_entries([])", "{}"),
        ("from_entries([{key: 1, value: null}])", "{[1]: null}"),
        // Extra entries are ignored.
        ("from_entries([{key: 'a', value: 1, note: 'x'}])", "{a: 1}"),
        // The last of a repeated key wins.
        (
            "from_entries([{key: 'x', value: 1}, {key: 'x', value: 2}])",
            "{x: 2}",
        ),
    ]);
}

#[test]
fn from_entries_inverts_to_entries() {
    assert_values(&[(
        "from_entries(to_entries({a: 1, [2]: 'b', [true]: [3], [x'ff']: null}))",
        "{a: 1, [2]: 'b', [true]: [3], [x'ff']: null}",
    )]);
}

#[test]
fn from_entries_requires_well_formed_entries() {
    assert_raises(&[
        (
            "from_entries([{key: 'a', value: 1}, 2])",
            "Function from_entries requires an Array of Maps, but element 1 is Int",
        ),
        (
            "from_entries([{value: 1}])",
            "Function from_entries requires a key in every element, but element 0 has none",
        ),
        (
            "from_entries([{key: 'a'}])",
            "Function from_entries requires a value in every element, but element 0 has none",
        ),
        (
            "from_entries([{key: null, value: 1}])",
            "Function from_entries requires a valid Map key in every element, \
             but the key of element 0 is Null",
        ),
        (
            "from_entries([{key: [1], value: 1}])",
            "Function from_entries requires a valid Map key in every element, \
             but the key of element 0 is Array",
        ),
    ]);
}

#[test]
fn to_entries_and_from_entries_check_their_argument() {
    assert_type_errors(
        "to_entries",
        &[("to_entries([1])", "Map", "argument 1", "Array")],
    );
    assert_type_errors(
        "from_entries",
        &[("from_entries({})", "Array", "argument 1 (entries)", "Map")],
    );
    assert_arity("to_entries", "1", &[0, 2]);
    assert_arity("from_entries", "1", &[0, 2]);
}

// --- dissoc ---

#[test]
fn dissoc_removes_keys() {
    assert_values(&[
        ("dissoc({a: 1, b: 2, c: 3}, 'b')", "{a: 1, c: 3}"),
        ("dissoc({a: 1, b: 2, c: 3}, 'a', 'c')", "{b: 2}"),
        ("dissoc({a: 1}, 'missing')", "{a: 1}"),
        ("dissoc({a: 1})", "{a: 1}"),
        ("{a: 1, b: 2} @ dissoc('b')", "{a: 1}"),
        // Keys of different types are different keys.
        (
            "dissoc({[1]: 'int', ['1']: 'string'}, 1)",
            "{['1']: 'string'}",
        ),
    ]);
}

#[test]
fn dissoc_checks_its_arguments() {
    assert_type_errors(
        "dissoc",
        &[
            ("dissoc([1], 0)", "Map", "argument 1", "Array"),
            ("dissoc({}, null)", MAP_KEY, "argument 2 (keys)", "Null"),
            (
                "dissoc({}, 'a', [1])",
                MAP_KEY,
                "argument 3 (keys)",
                "Array",
            ),
        ],
    );
    assert_arity("dissoc", "at least 1", &[0]);
}

// --- deep_merge ---

#[test]
fn deep_merge_merges_maps_at_every_level() {
    assert_values(&[
        (
            "deep_merge({a: {x: 1, y: 2}}, {a: {y: 3, z: 4}})",
            "{a: {x: 1, y: 3, z: 4}}",
        ),
        (
            "deep_merge({a: {b: {c: 1, d: 2}}, e: 5}, {a: {b: {d: 3}}, f: 6})",
            "{a: {b: {c: 1, d: 3}}, e: 5, f: 6}",
        ),
        ("deep_merge({a: {x: 1}}, {a: {}})", "{a: {x: 1}}"),
        ("deep_merge({a: {}}, {a: {x: 1}})", "{a: {x: 1}}"),
        (
            "deep_merge({[1]: {x: 1}}, {[1]: {y: 2}})",
            "{[1]: {x: 1, y: 2}}",
        ),
        ("{a: {x: 1}} @ deep_merge({a: {y: 2}})", "{a: {x: 1, y: 2}}"),
    ]);
}

#[test]
fn deep_merge_lets_the_right_side_win_any_other_collision() {
    assert_values(&[
        ("deep_merge({a: 1, b: 2}, {b: 3})", "{a: 1, b: 3}"),
        // Arrays are values like any other: never concatenated or merged by index.
        ("deep_merge({a: [1, 2]}, {a: [3]})", "{a: [3]}"),
        ("deep_merge({a: [{x: 1}]}, {a: [{y: 2}]})", "{a: [{y: 2}]}"),
        // A Map meeting anything but a Map is replaced, or replaces.
        ("deep_merge({a: {x: 1}}, {a: 5})", "{a: 5}"),
        ("deep_merge({a: 5}, {a: {x: 1}})", "{a: {x: 1}}"),
        ("deep_merge({a: {x: 1}}, {a: [1]})", "{a: [1]}"),
        ("deep_merge({a: {x: 1}}, {a: null})", "{a: null}"),
        // Keys of different types are different keys.
        (
            "deep_merge({['1']: {x: 1}}, {[1]: {y: 2}})",
            "{['1']: {x: 1}, [1]: {y: 2}}",
        ),
    ]);
}

#[test]
fn deep_merge_is_plus_where_no_two_maps_collide() {
    for (left, right) in [
        ("{}", "{}"),
        ("{a: 1}", "{}"),
        ("{}", "{a: {x: 1}}"),
        ("{a: 1, b: [1]}", "{b: [2], c: 3}"),
        ("{a: {x: 1}}", "{a: 2}"),
    ] {
        assert_values(&[(
            &format!("deep_merge({left}, {right})"),
            &format!("{left} + {right}"),
        )]);
    }
}

#[test]
fn deep_merge_checks_its_arguments() {
    assert_type_errors(
        "deep_merge",
        &[
            ("deep_merge([1], {})", "Map", "argument 1", "Array"),
            ("deep_merge(null, {})", "Map", "argument 1", "Null"),
            ("deep_merge({}, [1])", "Map", "argument 2", "Array"),
            ("deep_merge({}, 5)", "Map", "argument 2", "Int"),
        ],
    );
    assert_arity("deep_merge", "2", &[0, 1, 3]);
}

// --- len ---

#[test]
fn len_measures_a_sequence_or_structure() {
    assert_values(&[
        ("len('abc')", "3"),
        ("len('')", "0"),
        // A String's length is in code points, not bytes.
        (r"len('\u{e9}\u{1f600}')", "2"),
        ("len(x'c3a9')", "2"),
        ("len(x'')", "0"),
        ("len([1, [2, 3]])", "2"),
        ("len([])", "0"),
        ("len({a: 1, b: 2})", "2"),
        ("len({})", "0"),
    ]);
}

#[test]
fn len_checks_its_argument() {
    let measurable = "String or Bytes or Array or Map";
    assert_type_errors(
        "len",
        &[
            ("len(1)", measurable, "argument 1", "Int"),
            ("len(null)", measurable, "argument 1", "Null"),
            ("len(id)", measurable, "argument 1", "Function"),
        ],
    );
    assert_arity("len", "1", &[0, 2]);
}

// --- range ---

#[test]
fn range_counts_up_to_a_stop() {
    assert_values(&[
        ("range(5)", "[0, 1, 2, 3, 4]"),
        ("range(1)", "[0]"),
        ("range(0)", "[]"),
        ("range(-3)", "[]"),
        ("range(2, 5)", "[2, 3, 4]"),
        ("range(-2, 2)", "[-2, -1, 0, 1]"),
        ("range(5, 5)", "[]"),
        ("range(5, 2)", "[]"),
    ]);
}

#[test]
fn range_counts_by_a_step() {
    assert_values(&[
        ("range(0, 10, 2)", "[0, 2, 4, 6, 8]"),
        ("range(0, 9, 3)", "[0, 3, 6]"),
        ("range(5, 0, -1)", "[5, 4, 3, 2, 1]"),
        ("range(5, 0, -2)", "[5, 3, 1]"),
        ("range(0, 5, -1)", "[]"),
        ("range(5, 0, 1)", "[]"),
        ("range(0, 5, 10)", "[0]"),
    ]);
}

#[test]
fn range_stops_at_the_edge_of_the_int_range() {
    assert_values(&[
        (
            "range(9223372036854775800, 9223372036854775807, 5)",
            "[9223372036854775800, 9223372036854775805]",
        ),
        (
            "range(-9223372036854775800, -9223372036854775807 - 1, -5)",
            "[-9223372036854775800, -9223372036854775805]",
        ),
    ]);
}

#[test]
fn range_requires_a_nonzero_step() {
    assert_raises(&[
        (
            "range(0, 5, 0)",
            "Function range requires a step other than 0",
        ),
        (
            "range(5, 5, 0)",
            "Function range requires a step other than 0",
        ),
    ]);
}

#[test]
fn range_checks_its_arguments() {
    assert_type_errors(
        "range",
        &[
            ("range('a')", "Int", "argument 1", "String"),
            ("range(1.0)", "Int", "argument 1", "Float"),
            ("range(1, null)", "Int", "argument 2", "Null"),
            ("range(1, 2, 1.0)", "Int", "argument 3", "Float"),
        ],
    );
    assert_arity("range", "between 1 and 3", &[0, 4]);
}

// --- nulls ---

#[test]
fn nulls_makes_an_array_of_nulls() {
    assert_values(&[("nulls(3)", "[null, null, null]"), ("nulls(0)", "[]")]);
    assert_raises(&[
        (
            "nulls(-1)",
            "Function nulls requires argument 1 (count) to be at least 0, got -1",
        ),
        // A length past what memory can address is an error, not a crash.
        (
            "nulls(9223372036854775807)",
            "Function nulls cannot make a sequence that long",
        ),
    ]);
    assert_type_errors(
        "nulls",
        &[("nulls('3')", "Int", "argument 1 (count)", "String")],
    );
    assert_arity("nulls", "1", &[0, 2]);
}

// --- repeat ---

#[test]
fn repeat_makes_an_array_of_copies() {
    assert_values(&[
        ("repeat(0, 3)", "[0, 0, 0]"),
        ("repeat('x', 4)", "['x', 'x', 'x', 'x']"),
        ("repeat(null, 0)", "[]"),
        ("repeat(7, 1)", "[7]"),
        // A sequence or structure is one element, not spliced in.
        ("repeat('ab', 2)", "['ab', 'ab']"),
        ("repeat(x'ff', 2)", "[x'ff', x'ff']"),
        ("repeat([1, 2], 2)", "[[1, 2], [1, 2]]"),
        ("repeat({a: 1}, 2)", "[{a: 1}, {a: 1}]"),
        ("0 @ repeat(2)", "[0, 0]"),
        ("repeat(null, 2)", "nulls(2)"),
    ]);
}

#[test]
fn repeat_copies_a_function() {
    assert_values(&[("map(repeat(id, 2), fn f -> f(5))", "[5, 5]")]);
}

#[test]
fn repeat_checks_its_arguments() {
    assert_raises(&[
        (
            "repeat(0, -1)",
            "Function repeat requires argument 2 (count) to be at least 0, got -1",
        ),
        // A length past what memory can address is an error, not a crash.
        (
            "repeat(0, 9223372036854775807)",
            "Function repeat cannot make a sequence that long",
        ),
    ]);
    assert_type_errors(
        "repeat",
        &[
            ("repeat(0, '3')", "Int", "argument 2 (count)", "String"),
            ("repeat(0, 3.0)", "Int", "argument 2 (count)", "Float"),
            ("repeat(0, null)", "Int", "argument 2 (count)", "Null"),
        ],
    );
    assert_arity("repeat", "2", &[0, 1, 3]);
}

// --- tile ---

#[test]
fn tile_joins_copies_of_a_sequence() {
    assert_values(&[
        ("tile('ab', 3)", "'ababab'"),
        ("tile('ab', 1)", "'ab'"),
        ("tile('ab', 0)", "''"),
        ("tile('', 5)", "''"),
        (r"tile('\u{e9}-', 2)", r"'\u{e9}-\u{e9}-'"),
        ("tile(x'ff00', 2)", "x'ff00ff00'"),
        ("tile(x'ff', 0)", "x''"),
        ("tile([1, [2]], 2)", "[1, [2], 1, [2]]"),
        ("tile([], 3)", "[]"),
        ("tile([1], 0)", "[]"),
        ("'-' @ tile(3)", "'---'"),
    ]);
}

#[test]
fn tile_makes_any_number_of_copies_of_nothing_at_once() {
    // Copying an empty sequence one copy at a time would take practically
    // forever.
    assert_values(&[
        ("tile('', 9223372036854775807)", "''"),
        ("tile(x'', 9223372036854775807)", "x''"),
        ("tile([], 9223372036854775807)", "[]"),
    ]);
}

#[test]
fn tile_relates_to_repeat() {
    assert_values(&[
        // Tiling a one-element Array is repeating its element.
        ("tile([7], 3)", "repeat(7, 3)"),
        // Tiling joins what repeating keeps apart.
        ("flatten(repeat([1, 2], 3), 1)", "tile([1, 2], 3)"),
    ]);
}

#[test]
fn tile_checks_its_arguments() {
    assert_raises(&[
        (
            "tile('ab', -1)",
            "Function tile requires argument 2 (count) to be at least 0, got -1",
        ),
        // A length past what memory can address is an error, not a crash.
        (
            "tile('ab', 9223372036854775807)",
            "Function tile cannot make a sequence that long",
        ),
        (
            "tile(x'ff00', 9223372036854775807)",
            "Function tile cannot make a sequence that long",
        ),
        (
            "tile([1], 9223372036854775807)",
            "Function tile cannot make a sequence that long",
        ),
        (
            "tile([[]], 9223372036854775807)",
            "Function tile cannot make a sequence that long",
        ),
    ]);
    let sequence = "String or Bytes or Array";
    assert_type_errors(
        "tile",
        &[
            ("tile({a: 1}, 2)", sequence, "argument 1", "Map"),
            ("tile(5, 2)", sequence, "argument 1", "Int"),
            ("tile('a', 2.0)", "Int", "argument 2 (count)", "Float"),
            ("tile('a', null)", "Int", "argument 2 (count)", "Null"),
        ],
    );
    assert_arity("tile", "2", &[0, 1, 3]);
}

// --- has ---

#[test]
fn has_tests_for_a_map_key() {
    assert_values(&[
        ("has({a: 1}, 'a')", "true"),
        ("has({a: 1}, 'b')", "false"),
        // A key whose value is Null is still present.
        ("has({a: null}, 'a')", "true"),
        ("has({[1]: 'x'}, 1)", "true"),
        // Keys of different types are different keys.
        ("has({[1]: 'x'}, 1.0)", "false"),
        ("has({[1]: 'x'}, '1')", "false"),
        ("has({}, true)", "false"),
    ]);
}

#[test]
fn has_tests_for_an_array_index() {
    assert_values(&[
        ("has([1, 2, 3], 0)", "true"),
        ("has([1, 2, 3], 2)", "true"),
        ("has([1, 2, 3], 3)", "false"),
        // A negative index counts from the end, as indexing does.
        ("has([1, 2, 3], -1)", "true"),
        ("has([1, 2, 3], -3)", "true"),
        ("has([1, 2, 3], -4)", "false"),
        ("has([], 0)", "false"),
        ("has([null], 0)", "true"),
        // The extremes of Int are out of range, not an overflow.
        ("has([1, 2, 3], -9223372036854775807 - 1)", "false"),
        ("has([1, 2, 3], 9223372036854775807)", "false"),
        ("has([], -9223372036854775807 - 1)", "false"),
        ("has([], 9223372036854775807)", "false"),
    ]);
}

#[test]
fn has_checks_its_arguments() {
    assert_raises(&[(
        "has([1], 'a')",
        "Function has requires Int as argument 2 (index) for an Array, got String",
    )]);
    assert_type_errors(
        "has",
        &[
            ("has('abc', 0)", "Structured", "argument 1", "String"),
            ("has({}, null)", MAP_KEY, "argument 2 (index)", "Null"),
            ("has([1], [0])", MAP_KEY, "argument 2 (index)", "Array"),
        ],
    );
    assert_arity("has", "2", &[0, 1, 3]);
}

// --- index, dig ---

#[test]
fn index_makes_a_function_that_indexes() {
    assert_values(&[
        ("index('a')({a: 1})", "1"),
        ("index(0)([5, 6])", "5"),
        ("index(-1)([5, 6])", "6"),
        ("index('b')({a: 1})", "null"),
        ("index(9)([5, 6])", "null"),
        ("map([{n: 1}, {n: 2}], index('n'))", "[1, 2]"),
    ]);
    assert_raises_as(&[("index('a')([1])", "[1]['a']"), ("index(0)(5)", "5[0]")]);
    assert_arity("index", "1", &[0, 2]);
    assert_raises(&[(
        "index('a')()",
        "Function index_fn expects 1 argument, but was called with 0",
    )]);
}

#[test]
fn dig_indexes_through_nested_structures() {
    assert_values(&[
        ("{a: {b: {c: 42}}} @ dig('a', 'b', 'c')", "42"),
        (
            "{items: [{name: 'alice'}, {name: 'bob'}]} @ dig('items', 1, 'name')",
            "'bob'",
        ),
        ("{a: 1} @ dig()", "{a: 1}"),
        ("dig(5)", "5"),
    ]);
}

#[test]
fn dig_stops_at_a_null() {
    assert_values(&[
        ("{a: {b: {c: 42}}} @ dig('a', 'x', 'c')", "null"),
        (
            "{items: [{name: 'alice'}]} @ dig('items', 5, 'name')",
            "null",
        ),
        ("{a: null} @ dig('a', 'b', 'c')", "null"),
        ("dig(null, 'a')", "null"),
    ]);
}

#[test]
fn dig_raises_as_indexing_does() {
    assert_raises_as(&[
        ("{a: 5} @ dig('a', 'b')", "5['b']"),
        ("{a: [1]} @ dig('a', 'b')", "[1]['b']"),
        ("{a: 1} @ dig(null)", "{a: 1}[null]"),
    ]);
    assert_arity("dig", "at least 1", &[0]);
}

// --- dig_set, dig_update ---
//
// Both walk a path as `dig` does, but a write needs a place: every step before
// the last must find an Array or Map, the last may add a key to a Map, and an
// Array is never resized.

#[test]
fn dig_set_replaces_the_value_at_a_path() {
    assert_values(&[
        ("dig_set({a: {b: 1}}, 2, 'a', 'b')", "{a: {b: 2}}"),
        (
            "dig_set({a: {b: 1, c: 3}, d: 4}, 2, 'a', 'b')",
            "{a: {b: 2, c: 3}, d: 4}",
        ),
        ("dig_set([[1, 2], [3]], 9, 0, 1)", "[[1, 9], [3]]"),
        // A path through both Maps and Arrays.
        (
            "dig_set({items: [{name: 'alice'}, {name: 'bob'}]}, 'eve', 'items', 1, 'name')",
            "{items: [{name: 'alice'}, {name: 'eve'}]}",
        ),
        ("dig_set([{a: [1, 2]}], 9, 0, 'a', 0)", "[{a: [9, 2]}]"),
        // A Null or a structure is a value like any other.
        ("dig_set({a: 1}, null, 'a')", "{a: null}"),
        ("dig_set({a: null}, 1, 'a')", "{a: 1}"),
        ("dig_set({a: 1}, {b: [2]}, 'a')", "{a: {b: [2]}}"),
        ("{a: 1} @ dig_set(2, 'a')", "{a: 2}"),
        // A Function is stored, never called.
        ("dig_set({a: 1}, id, 'a').a == id", "true"),
    ]);
}

#[test]
fn dig_set_indexes_an_array_as_reading_does() {
    assert_values(&[
        ("dig_set([1, 2, 3], 9, 0)", "[9, 2, 3]"),
        ("dig_set([1, 2, 3], 9, 2)", "[1, 2, 9]"),
        // A negative index counts from the end.
        ("dig_set([1, 2, 3], 9, -1)", "[1, 2, 9]"),
        ("dig_set([1, 2, 3], 9, -3)", "[9, 2, 3]"),
        ("dig_set([[1, 2], [3, 4]], 9, -1, -2)", "[[1, 2], [9, 4]]"),
    ]);
}

#[test]
fn dig_set_adds_a_key_to_a_map_at_the_end_of_the_path() {
    assert_values(&[
        ("dig_set({}, 1, 'a')", "{a: 1}"),
        ("dig_set({a: {}}, 1, 'a', 'b')", "{a: {b: 1}}"),
        ("dig_set([{}], 1, 0, 'b')", "[{b: 1}]"),
        // The value added may itself be a structure, inserting structure deeply.
        ("dig_set({a: {}}, {c: [1]}, 'a', 'b')", "{a: {b: {c: [1]}}}"),
        // An Int key into a Map is a Map key: no Array is made.
        ("dig_set({}, 'x', 0)", "{[0]: 'x'}"),
        ("dig_set({[1]: 'a'}, 'b', 1)", "{[1]: 'b'}"),
        // Keys of different types are different keys.
        ("dig_set({['1']: 'a'}, 'b', 1)", "{['1']: 'a', [1]: 'b'}"),
        ("dig_set({}, 'x', true)", "{[true]: 'x'}"),
        ("dig_set({}, 'x', 1.5)", "{[1.5]: 'x'}"),
        ("dig_set({}, 'x', x'ff')", "{[x'ff']: 'x'}"),
        ("dig_set({[1]: 'a'}, 'b', 1.0)", "{[1]: 'a', [1.0]: 'b'}"),
        (
            "dig_set({[0.0]: 'a'}, 'b', -0.0)",
            "{[0.0]: 'a', [-0.0]: 'b'}",
        ),
    ]);
}

#[test]
fn dig_set_without_a_path_is_its_value() {
    assert_values(&[
        ("dig_set({a: 1}, 5)", "5"),
        ("dig_set([1], {a: 1})", "{a: 1}"),
        // With nothing to index, the first argument may be anything.
        ("dig_set(null, 5)", "5"),
        ("dig_set('text', 5)", "5"),
    ]);
}

#[test]
fn dig_set_takes_a_dynamic_path_through_call() {
    let source = r"
        def path = ['a', 1, 'b']
        call(dig_set, [{a: [0, {b: 1}]}, 5] + path)
    ";
    assert_values(&[
        (source, "{a: [0, {b: 5}]}"),
        (
            "call(dig_set, [{a: {b: 1}}, 2] + split('a.b', '.'))",
            "{a: {b: 2}}",
        ),
        ("call(dig_set, [{a: 1}, 5] + [])", "5"),
    ]);
}

#[test]
fn dig_update_replaces_the_value_at_a_path_with_its_function_of_it() {
    assert_values(&[
        (
            "dig_update({a: {n: 1}}, fn n -> n + 1, 'a', 'n')",
            "{a: {n: 2}}",
        ),
        (
            "dig_update([1, [2, 3]], fn x -> x * 10, 1, -1)",
            "[1, [2, 30]]",
        ),
        (
            "dig_update({items: [{tags: ['a']}]}, fn t -> t + ['b'], 'items', 0, 'tags')",
            "{items: [{tags: ['a', 'b']}]}",
        ),
        ("dig_update({a: 1}, fn x -> null, 'a')", "{a: null}"),
        ("dig_update({a: 1}, fn x -> id, 'a').a == id", "true"),
        ("{a: 1} @ dig_update(fn x -> x + 1, 'a')", "{a: 2}"),
        // Any Function will do, a native as well as a lambda.
        ("dig_update({a: [1, 2]}, len, 'a')", "{a: 2}"),
    ]);
}

#[test]
fn dig_update_calls_its_function_with_one_argument() {
    assert_values(&[("dig_update({a: 1}, $($$), 'a')", "{a: [1]}")]);
    assert_raises(&[(
        "dig_update({a: 1}, fn -> 5, 'a')",
        "Function <lambda> expects 0 arguments, but was called with 1",
    )]);
}

#[test]
fn dig_update_passes_null_for_a_key_the_map_lacks() {
    assert_values(&[
        ("dig_update({}, fn x -> [x], 'k')", "{k: [null]}"),
        (
            "dig_update({a: {}}, fn n -> (n or 0) + 1, 'a', 'count')",
            "{a: {count: 1}}",
        ),
        // A present Null is passed as it is.
        ("dig_update({k: null}, fn x -> [x], 'k')", "{k: [null]}"),
    ]);
}

#[test]
fn dig_update_without_a_path_applies_its_function_to_the_whole() {
    assert_values(&[
        ("dig_update({a: 1}, fn m -> m + {b: 2})", "{a: 1, b: 2}"),
        ("dig_update(5, fn x -> x + 1)", "6"),
        ("dig_update(null, fn x -> [x])", "[null]"),
    ]);
}

#[test]
fn dig_update_takes_a_dynamic_path_through_call() {
    let source = r"
        def path = ['a', -1]
        call(dig_update, [{a: [1, 2]}, fn x -> x * 10] + path)
    ";
    assert_values(&[(source, "{a: [1, 20]}")]);
}

#[test]
fn dig_update_calls_its_function_once() {
    assert_eq!(
        printed("dig_update({a: [1]}, fn x -> { print(x); x }, 'a', 0)"),
        ["1"],
        "the function sees the value at the path, once"
    );
    assert_eq!(
        printed("dig_update({}, fn x -> { print(x); x }, 'k')"),
        ["null"],
        "the function sees Null for an absent key, once"
    );
}

#[test]
fn dig_update_raises_what_its_function_raises() {
    assert_raises(&[
        ("dig_update({a: 1}, fn x -> error('boom'), 'a')", "boom"),
        ("dig_update({}, fn x -> error('boom'), 'a')", "boom"),
        ("dig_update(1, fn x -> error('boom'))", "boom"),
    ]);
}

/// Each path-editing global, with a payload it accepts, for tests whose cases
/// hold for both.
const PATH_EDITS: [(&str, &str); 2] = [("dig_set", "1"), ("dig_update", "id")];

/// A call of `function` with `payload` editing `structure` along `keys`, a
/// comma-separated list of Frost expressions.
fn path_edit(function: &str, payload: &str, structure: &str, keys: &str) -> String {
    format!("{function}({structure}, {payload}, {keys})")
}

/// Assert each `(structure, keys, problem)` raises, for both path edits,
/// `Function <name> <problem>`.
fn assert_path_edits_raise(cases: &[(&str, &str, &str)]) {
    for (function, payload) in PATH_EDITS {
        for (structure, keys, problem) in cases {
            assert_eq!(
                raises(&path_edit(function, payload, structure, keys)),
                format!("Function {function} {problem}"),
                "{function} of {structure} along {keys}"
            );
        }
    }
}

#[test]
fn path_edits_index_only_a_map_or_array_at_every_step() {
    assert_path_edits_raise(&[
        ("null", "'a'", "cannot index Null"),
        ("5", "0", "cannot index Int"),
        // A String or Bytes has no places to write, though it may be read by index.
        ("'abc'", "0", "cannot index String"),
        ("x'00'", "0", "cannot index Bytes"),
        // A Null is not replaced by a Map.
        ("{a: null}", "'a', 'b'", "cannot index Null at `a`"),
        ("{a: 5}", "'a', 'b'", "cannot index Int at `a`"),
        ("{a: true}", "'a', 'b'", "cannot index Bool at `a`"),
        ("{a: 1.5}", "'a', 'b'", "cannot index Float at `a`"),
        ("{a: id}", "'a', 'b'", "cannot index Function at `a`"),
        (
            "[{a: [1, 'x']}]",
            "0, 'a', 1, 0",
            "cannot index String at `[0].a[1]`",
        ),
    ]);
}

#[test]
fn path_edit_errors_write_the_path_as_frost_indexes_it() {
    assert_path_edits_raise(&[
        (
            "{[1]: {[true]: 5}}",
            "1, true, 'x'",
            "cannot index Int at `[1][true]`",
        ),
        (
            "{['my-key']: 5}",
            "'my-key', 0",
            r#"cannot index Int at `["my-key"]`"#,
        ),
        // A keyword is no identifier, so it cannot follow a dot.
        ("{['if']: 5}", "'if', 0", r#"cannot index Int at `["if"]`"#),
        ("{a: {b: 5}}", "'a', 'b', 0", "cannot index Int at `a.b`"),
        (
            "{[1.5]: {[x'ff']: 5}}",
            "1.5, x'ff', 0",
            "cannot index Int at `[1.5][x'ff']`",
        ),
    ]);
}

#[test]
fn path_edits_create_no_intermediate_structure() {
    assert_path_edits_raise(&[
        ("{}", "'a', 'b'", "found no value at `a`"),
        ("{a: {}}", "'a', 'b', 'c'", "found no value at `a.b`"),
        ("{a: [{}]}", "'a', 0, 1, 'x'", "found no value at `a[0][1]`"),
        // Keys of different types are different keys.
        ("{['1']: {}}", "1, 'x'", "found no value at `[1]`"),
        // A mistyped String key gets a suggestion, as indexing gives one.
        (
            "{name: {}}",
            "'nmae', 'first'",
            "found no value at `nmae`; did you mean `name`?",
        ),
        (
            "{a: {name: {}}}",
            "'a', 'nmae', 'first'",
            "found no value at `a.nmae`; did you mean `a.name`?",
        ),
    ]);
}

#[test]
fn path_edits_never_resize_an_array() {
    assert_path_edits_raise(&[
        (
            "[1, 2]",
            "2",
            "found no value at `[2]` in an Array of length 2",
        ),
        (
            "[1, 2]",
            "-3",
            "found no value at `[-3]` in an Array of length 2",
        ),
        ("[]", "0", "found no value at `[0]` in an Array of length 0"),
        (
            "{a: [[1]]}",
            "'a', 0, 5",
            "found no value at `a[0][5]` in an Array of length 1",
        ),
        (
            "[1, 2]",
            "9, 'a'",
            "found no value at `[9]` in an Array of length 2",
        ),
        // The farthest indices are out of range, not an overflow.
        (
            "[1, 2]",
            "9223372036854775807",
            "found no value at `[9223372036854775807]` in an Array of length 2",
        ),
        (
            "[1, 2]",
            "-9223372036854775807 - 1",
            "found no value at `[-9223372036854775808]` in an Array of length 2",
        ),
    ]);
}

#[test]
fn path_edits_index_an_array_only_by_int() {
    assert_path_edits_raise(&[
        ("[1]", "'a'", "cannot index Array by String"),
        ("[1]", "0.0", "cannot index Array by Float"),
        ("[1]", "true", "cannot index Array by Bool"),
        (
            "{a: [1]}",
            "'a', x'00'",
            "cannot index Array at `a` by Bytes",
        ),
    ]);
}

#[test]
fn path_edit_errors_are_the_same_however_called() {
    assert_raises(&[
        (
            "{a: {}} @ dig_set(1, 'a', 'b', 'c')",
            "Function dig_set found no value at `a.b`",
        ),
        (
            "call(dig_set, [{a: {}}, 1] + split('a.b.c', '.'))",
            "Function dig_set found no value at `a.b`",
        ),
        (
            "{a: {}} @ dig_update(id, 'a', 'b', 'c')",
            "Function dig_update found no value at `a.b`",
        ),
    ]);
}

#[test]
fn dig_update_checks_the_path_before_calling_its_function() {
    assert_raises(&[
        (
            "dig_update({a: {}}, fn x -> error('called'), 'a', 'b', 'c')",
            "Function dig_update found no value at `a.b`",
        ),
        (
            "dig_update([1], fn x -> error('called'), 1)",
            "Function dig_update found no value at `[1]` in an Array of length 1",
        ),
        (
            "dig_update([1], fn x -> error('called'), 'a')",
            "Function dig_update cannot index Array by String",
        ),
        (
            "dig_update(null, fn x -> error('called'), 'a')",
            "Function dig_update cannot index Null",
        ),
    ]);
}

#[test]
fn dig_set_and_dig_update_check_their_arguments() {
    for (function, payload) in PATH_EDITS {
        assert_type_errors(
            function,
            &[
                (
                    &path_edit(function, payload, "{}", "null"),
                    MAP_KEY,
                    "argument 3 (keys)",
                    "Null",
                ),
                (
                    &path_edit(function, payload, "{}", "'a', [1]"),
                    MAP_KEY,
                    "argument 4 (keys)",
                    "Array",
                ),
                (
                    &path_edit(function, payload, "{}", "'a', 'b', {}"),
                    MAP_KEY,
                    "argument 5 (keys)",
                    "Map",
                ),
            ],
        );
        assert_arity(function, "at least 2", &[0, 1]);
    }
    assert_type_errors(
        "dig_update",
        &[
            (
                "dig_update({a: 1}, 1, 'a')",
                "Function",
                "argument 2",
                "Int",
            ),
            ("dig_update({a: 1}, null)", "Function", "argument 2", "Null"),
        ],
    );
}

// --- any, all, none ---

#[test]
fn quantifiers_test_truthiness_without_a_function() {
    assert_values(&[
        // Only Null and False are falsy.
        ("any([null, false, 0])", "true"),
        ("any([null, false])", "false"),
        ("any([])", "false"),
        ("all([1, 0, ''])", "true"),
        ("all([1, null])", "false"),
        ("all([])", "true"),
        ("none([null, false])", "true"),
        ("none([0])", "false"),
        ("none([])", "true"),
    ]);
}

#[test]
fn quantifiers_test_a_function_on_each_element() {
    assert_values(&[
        ("any([1, 2, 3], fn x -> x > 2)", "true"),
        ("any([1, 2, 3], fn x -> x > 3)", "false"),
        ("all([1, 2, 3], fn x -> x > 0)", "true"),
        ("all([1, 2, 3], fn x -> x > 1)", "false"),
        ("none([1, 2, 3], fn x -> x > 3)", "true"),
        ("none([1, 2, 3], fn x -> x > 2)", "false"),
        ("any([], fn x -> true)", "false"),
        ("all([], fn x -> false)", "true"),
        ("none([], fn x -> true)", "true"),
    ]);
}

#[test]
fn quantifiers_stop_at_the_first_deciding_element() {
    let probe = "fn x -> { print(x); x == 2 }";
    for (source, result, shown) in [
        (format!("any([1, 2, 3], {probe})"), "true", ["1", "2"]),
        (format!("none([1, 2, 3], {probe})"), "false", ["1", "2"]),
        (format!("all([2, 1, 2], {probe})"), "false", ["2", "1"]),
    ] {
        assert_values(&[(&source, result)]);
        assert_eq!(printed(&source), shown, "{source}");
    }
}

#[test]
fn quantifiers_raise_what_their_function_raises() {
    for function in ["any", "all", "none"] {
        let script = script(&format!(
            "{function}([1, 2], fn x -> {{ print(x); error('boom') }})"
        ));
        assert_eq!(script.raises(), "boom", "{function}");
        assert_eq!(script.printed(), ["1"], "{function} stops at the error");
    }
}

#[test]
fn quantifiers_check_their_arguments() {
    for function in ["any", "all", "none"] {
        assert_type_errors(
            function,
            &[
                (
                    &format!("{function}({{a: 1}})"),
                    "Array",
                    "argument 1",
                    "Map",
                ),
                (
                    &format!("{function}([1], 1)"),
                    "Function",
                    "argument 2 (predicate)",
                    "Int",
                ),
            ],
        );
        assert_arity(function, "between 1 and 2", &[0, 3]);
    }
}

// --- find ---

#[test]
fn find_returns_the_first_matching_element() {
    assert_values(&[
        ("find([1, 2, 3, 4], fn x -> x > 2)", "3"),
        ("find([1, 2], fn x -> x > 5)", "null"),
        ("find([], fn x -> true)", "null"),
        ("find([[1], [2]], fn a -> a[0] == 2)", "[2]"),
    ]);
    // It stops at the first match.
    let probe = "find([1, 2, 3], fn x -> { print(x); x == 2 })";
    assert_values(&[(probe, "2")]);
    assert_eq!(printed(probe), ["1", "2"]);
    assert_type_errors(
        "find",
        &[
            ("find({}, id)", "Array", "argument 1", "Map"),
            ("find([1], 1)", "Function", "argument 2 (predicate)", "Int"),
        ],
    );
    assert_arity("find", "2", &[0, 1, 3]);
}

#[test]
fn find_raises_what_its_predicate_raises() {
    let script = script("find([1, 2], fn x -> { print(x); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["1"], "the first error ends the search");
}

// --- find_index ---

#[test]
fn find_index_returns_the_index_of_the_first_match() {
    assert_values(&[
        ("find_index([1, 2, 3, 4], fn x -> x > 2)", "2"),
        ("find_index([5, 6], fn x -> x == 5)", "0"),
        ("find_index([1, 2], fn x -> x > 5)", "null"),
        ("find_index([], fn x -> true)", "null"),
        ("find_index([[1], [2]], fn a -> a[0] == 2)", "1"),
        // The predicate's answer is tested for truthiness: 0 is truthy.
        (
            "find_index([1, 2, 3], fn x -> if x == 2: 0 else: null)",
            "1",
        ),
    ]);
    // It stops at the first match.
    let probe = "find_index([1, 2, 3], fn x -> { print(x); x == 2 })";
    assert_values(&[(probe, "1")]);
    assert_eq!(printed(probe), ["1", "2"]);
}

#[test]
fn find_index_raises_what_its_predicate_raises() {
    let script = script("find_index([1, 2], fn x -> { print(x); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["1"], "the first error ends the search");
}

#[test]
fn find_index_checks_its_arguments() {
    assert_type_errors(
        "find_index",
        &[
            ("find_index({}, id)", "Array", "argument 1", "Map"),
            ("find_index('ab', id)", "Array", "argument 1", "String"),
            (
                "find_index([1], 1)",
                "Function",
                "argument 2 (predicate)",
                "Int",
            ),
        ],
    );
    assert_arity("find_index", "2", &[0, 1, 3]);
}

// --- slice, take, drop, tail, drop_tail, stride ---
//
// A String's elements are its code points, and Bytes' its bytes.

/// The type a sequence argument must have, as type errors name it.
const SEQUENCE: &str = "String or Bytes or Array";

#[test]
fn slice_takes_a_range_of_an_array() {
    assert_values(&[
        ("slice([1, 2, 3, 4, 5], 1, 3)", "[2, 3]"),
        ("slice([1, 2, 3, 4, 5], 2)", "[3, 4, 5]"),
        ("slice([1, 2, 3, 4, 5], 0)", "[1, 2, 3, 4, 5]"),
        ("slice([1, 2, 3, 4, 5], 2, 2)", "[]"),
        ("slice([], 0)", "[]"),
        // A negative index counts from the end.
        ("slice([1, 2, 3, 4, 5], -2)", "[4, 5]"),
        ("slice([1, 2, 3, 4, 5], 1, -1)", "[2, 3, 4]"),
        ("slice([1, 2, 3, 4, 5], -3, -1)", "[3, 4]"),
    ]);
}

#[test]
fn slice_clamps_an_index_out_of_range() {
    assert_values(&[
        ("slice([1, 2, 3], 5)", "[]"),
        ("slice([1, 2, 3], 0, 100)", "[1, 2, 3]"),
        ("slice([1, 2, 3], -100)", "[1, 2, 3]"),
        ("slice([1, 2, 3], -100, -99)", "[]"),
        ("slice([1, 2, 3], -9223372036854775807 - 1)", "[1, 2, 3]"),
        ("slice([1, 2, 3], 0, 9223372036854775807)", "[1, 2, 3]"),
        // A start at or past the end is empty, never reversed.
        ("slice([1, 2, 3], 2, 1)", "[]"),
        ("slice([1, 2, 3], -1, 0)", "[]"),
    ]);
}

#[test]
fn slice_takes_a_range_of_a_string_by_code_point() {
    assert_values(&[
        ("slice('hello', 1, 4)", "'ell'"),
        ("slice('hello', -3)", "'llo'"),
        (r"slice('h\u{e9}llo', 1, 3)", r"'\u{e9}l'"),
        (r"slice('\u{1f600}\u{e9}x', -2)", r"'\u{e9}x'"),
        (r"slice('\u{1f600}\u{e9}x', 0, 1)", r"'\u{1f600}'"),
        ("slice('abc', 3)", "''"),
        ("slice('abc', 1, 1)", "''"),
        ("slice('', 0)", "''"),
    ]);
}

#[test]
fn slice_takes_a_range_of_bytes_by_byte() {
    assert_values(&[
        ("slice(x'00010203', 1, 3)", "x'0102'"),
        ("slice(x'00010203', -1)", "x'03'"),
        // Bytes have no characters to keep whole.
        ("slice(x'c3a9', 1)", "x'a9'"),
        ("slice(x'', 0)", "x''"),
    ]);
}

#[test]
fn slicing_leaves_the_original_unchanged() {
    let source = r"
        def a = [1, 2, 3, 4]
        def s = 'abcd'
        [slice(a, 1), take(a, 1), drop_tail(a, 1), stride(a, 2), a, slice(s, 1), s]
    ";
    assert_values(&[(
        source,
        "[[2, 3, 4], [1], [1, 2, 3], [1, 3], [1, 2, 3, 4], 'bcd', 'abcd']",
    )]);
}

#[test]
fn slice_checks_its_arguments() {
    assert_type_errors(
        "slice",
        &[
            ("slice({}, 0)", SEQUENCE, "argument 1", "Map"),
            ("slice(5, 0)", SEQUENCE, "argument 1", "Int"),
            ("slice('a', '0')", "Int", "argument 2 (start)", "String"),
            ("slice('a', 0, 1.0)", "Int", "argument 3 (end)", "Float"),
            ("slice('a', 0, null)", "Int", "argument 3 (end)", "Null"),
        ],
    );
    assert_arity("slice", "between 2 and 3", &[0, 1, 4]);
}

#[test]
fn take_drop_tail_and_drop_tail_keep_one_end() {
    assert_values(&[
        ("take([1, 2, 3], 2)", "[1, 2]"),
        ("drop([1, 2, 3], 2)", "[3]"),
        ("tail([1, 2, 3], 2)", "[2, 3]"),
        ("drop_tail([1, 2, 3], 2)", "[1]"),
        // Zero: `tail` keeps nothing, where `slice(s, -0)` would keep everything.
        ("take([1, 2, 3], 0)", "[]"),
        ("drop([1, 2, 3], 0)", "[1, 2, 3]"),
        ("tail([1, 2, 3], 0)", "[]"),
        ("drop_tail([1, 2, 3], 0)", "[1, 2, 3]"),
        // A count past the length is clamped to it.
        ("take([1, 2, 3], 5)", "[1, 2, 3]"),
        ("drop([1, 2, 3], 5)", "[]"),
        ("tail([1, 2, 3], 5)", "[1, 2, 3]"),
        ("drop_tail([1, 2, 3], 5)", "[]"),
        // However far past, up to the largest Int.
        ("take([1, 2, 3], 9223372036854775807)", "[1, 2, 3]"),
        ("drop([1, 2, 3], 9223372036854775807)", "[]"),
        ("tail([1, 2, 3], 9223372036854775807)", "[1, 2, 3]"),
        ("drop_tail([1, 2, 3], 9223372036854775807)", "[]"),
        ("take([], 1)", "[]"),
        ("tail([], 1)", "[]"),
    ]);
}

#[test]
fn take_drop_tail_and_drop_tail_count_code_points_and_bytes() {
    assert_values(&[
        (r"take('h\u{e9}llo', 2)", r"'h\u{e9}'"),
        (r"drop('\u{1f600}ab', 1)", "'ab'"),
        (r"tail('\u{1f600}ab', 2)", "'ab'"),
        (r"drop_tail('ab\u{1f600}', 1)", "'ab'"),
        ("take('', 3)", "''"),
        ("take(x'010203', 2)", "x'0102'"),
        ("drop(x'010203', 2)", "x'03'"),
        ("tail(x'010203', 1)", "x'03'"),
        ("drop_tail(x'010203', 1)", "x'0102'"),
        ("take(x'c3a9', 1)", "x'c3'"),
    ]);
}

#[test]
fn take_and_drop_split_a_sequence_as_drop_tail_and_tail_do() {
    for sequence in ["[1, 2, 3]", r"'a\u{e9}\u{1f600}'", "x'010203'"] {
        for n in 0..=4 {
            assert_values(&[
                (
                    &format!("take({sequence}, {n}) + drop({sequence}, {n})"),
                    sequence,
                ),
                (
                    &format!("drop_tail({sequence}, {n}) + tail({sequence}, {n})"),
                    sequence,
                ),
            ]);
        }
    }
}

#[test]
fn take_drop_tail_and_drop_tail_check_their_arguments() {
    for function in ["take", "drop", "tail", "drop_tail"] {
        assert_raises(&[(
            &format!("{function}([1], -1)"),
            &format!("Function {function} requires argument 2 (count) to be at least 0, got -1"),
        )]);
        assert_type_errors(
            function,
            &[
                (
                    &format!("{function}({{}}, 1)"),
                    SEQUENCE,
                    "argument 1",
                    "Map",
                ),
                (
                    &format!("{function}([1], 1.0)"),
                    "Int",
                    "argument 2 (count)",
                    "Float",
                ),
                (
                    &format!("{function}([1], null)"),
                    "Int",
                    "argument 2 (count)",
                    "Null",
                ),
            ],
        );
        assert_arity(function, "2", &[0, 1, 3]);
    }
}

#[test]
fn stride_takes_every_nth_element() {
    assert_values(&[
        ("stride([1, 2, 3, 4, 5, 6], 2)", "[1, 3, 5]"),
        ("stride([1, 2, 3, 4, 5, 6], 4)", "[1, 5]"),
        ("stride([1, 2, 3], 1)", "[1, 2, 3]"),
        ("stride([1, 2, 3], 10)", "[1]"),
        ("stride([1, 2, 3], 9223372036854775807)", "[1]"),
        ("stride([], 3)", "[]"),
        ("stride('abcdef', 2)", "'ace'"),
        (r"stride('\u{e9}x\u{1f600}y', 2)", r"'\u{e9}\u{1f600}'"),
        ("stride('', 2)", "''"),
        ("stride(x'00010203', 3)", "x'0003'"),
    ]);
}

#[test]
fn stride_checks_its_arguments() {
    for step in ["0", "-1"] {
        assert_raises(&[(
            &format!("stride([1], {step})"),
            &format!("Function stride requires argument 2 (step) to be at least 1, got {step}"),
        )]);
    }
    assert_type_errors(
        "stride",
        &[
            ("stride({}, 1)", SEQUENCE, "argument 1", "Map"),
            ("stride([1], 1.0)", "Int", "argument 2 (step)", "Float"),
        ],
    );
    assert_arity("stride", "2", &[0, 1, 3]);
}

// --- slide, chunk ---

#[test]
fn slide_makes_overlapping_windows() {
    assert_values(&[
        ("slide([1, 2, 3, 4], 3)", "[[1, 2, 3], [2, 3, 4]]"),
        ("slide([1, 2], 1)", "[[1], [2]]"),
        ("slide([1, 2], 2)", "[[1, 2]]"),
        ("slide([1, 2], 3)", "[]"),
        ("slide([1, 2], 9223372036854775807)", "[]"),
        ("slide([], 1)", "[]"),
    ]);
}

#[test]
fn chunk_makes_consecutive_chunks() {
    assert_values(&[
        ("chunk([1, 2, 3, 4, 5], 2)", "[[1, 2], [3, 4], [5]]"),
        ("chunk([1, 2, 3, 4], 2)", "[[1, 2], [3, 4]]"),
        ("chunk([1, 2], 5)", "[[1, 2]]"),
        ("chunk([1, 2], 9223372036854775807)", "[[1, 2]]"),
        ("chunk([], 2)", "[]"),
    ]);
}

#[test]
fn slide_and_chunk_check_their_arguments() {
    for function in ["slide", "chunk"] {
        assert_raises(&[
            (
                &format!("{function}([1], 0)"),
                &format!("Function {function} requires argument 2 (size) to be at least 1, got 0"),
            ),
            (
                &format!("{function}([1], -1)"),
                &format!("Function {function} requires argument 2 (size) to be at least 1, got -1"),
            ),
        ]);
        assert_type_errors(
            function,
            &[
                (
                    &format!("{function}('ab', 1)"),
                    "Array",
                    "argument 1",
                    "String",
                ),
                (
                    &format!("{function}([1], 1.0)"),
                    "Int",
                    "argument 2 (size)",
                    "Float",
                ),
            ],
        );
        assert_arity(function, "2", &[0, 1, 3]);
    }
}

// --- reverse ---

#[test]
fn reverse_reverses_a_sequence() {
    assert_values(&[
        ("reverse([1, 2, 3])", "[3, 2, 1]"),
        ("reverse([])", "[]"),
        // Only the outer Array.
        ("reverse([[1, 2], 3])", "[3, [1, 2]]"),
        ("reverse('abc')", "'cba'"),
        ("reverse('')", "''"),
        // A String reverses by code point.
        (r"reverse('\u{e9}x\u{1f600}')", r"'\u{1f600}x\u{e9}'"),
        ("reverse(x'0102ff')", "x'ff0201'"),
        ("reverse(x'')", "x''"),
    ]);
    assert_type_errors(
        "reverse",
        &[
            (
                "reverse({})",
                "String or Bytes or Array",
                "argument 1",
                "Map",
            ),
            (
                "reverse(1)",
                "String or Bytes or Array",
                "argument 1",
                "Int",
            ),
        ],
    );
    assert_arity("reverse", "1", &[0, 2]);
}

// --- take_while, drop_while ---

#[test]
fn take_while_and_drop_while_split_at_the_first_failure() {
    assert_values(&[
        ("take_while([1, 2, 3, 1], fn x -> x < 3)", "[1, 2]"),
        ("drop_while([1, 2, 3, 1], fn x -> x < 3)", "[3, 1]"),
        ("take_while([1, 2], fn x -> true)", "[1, 2]"),
        ("drop_while([1, 2], fn x -> true)", "[]"),
        ("take_while([1, 2], fn x -> false)", "[]"),
        ("drop_while([1, 2], fn x -> false)", "[1, 2]"),
        ("take_while([], fn x -> true)", "[]"),
        ("drop_while([], fn x -> true)", "[]"),
    ]);
}

#[test]
fn take_while_and_drop_while_test_each_element_once() {
    let probe = "fn x -> { print(x); x < 3 }";
    for (function, result) in [("take_while", "[1, 2]"), ("drop_while", "[3, 1]")] {
        let source = format!("{function}([1, 2, 3, 1], {probe})");
        assert_values(&[(&source, result)]);
        assert_eq!(printed(&source), ["1", "2", "3"], "{function}");
    }
}

#[test]
fn take_while_and_drop_while_raise_what_their_predicate_raises() {
    for function in ["take_while", "drop_while"] {
        let script = script(&format!(
            "{function}([1, 2], fn x -> {{ print(x); error('boom') }})"
        ));
        assert_eq!(script.raises(), "boom", "{function}");
        assert_eq!(script.printed(), ["1"], "{function} stops at the error");
    }
}

#[test]
fn take_while_and_drop_while_check_their_arguments() {
    for function in ["take_while", "drop_while"] {
        assert_type_errors(
            function,
            &[
                (
                    &format!("{function}({{}}, id)"),
                    "Array",
                    "argument 1",
                    "Map",
                ),
                (
                    &format!("{function}([1], 1)"),
                    "Function",
                    "argument 2 (predicate)",
                    "Int",
                ),
            ],
        );
        assert_arity(function, "2", &[0, 1, 3]);
    }
}

// --- flatten, flat_map ---

#[test]
fn flatten_flattens_every_level() {
    assert_values(&[
        ("flatten([1, [2, [3, 4]], 5])", "[1, 2, 3, 4, 5]"),
        ("flatten([])", "[]"),
        ("flatten([[], [[]]])", "[]"),
        // Only Arrays are flattened.
        ("flatten([{a: [1]}, 'ab'])", "[{a: [1]}, 'ab']"),
    ]);
}

#[test]
fn flatten_flattens_to_a_depth() {
    assert_values(&[
        ("flatten([1, [2, [3, 4]], 5], 1)", "[1, 2, [3, 4], 5]"),
        ("flatten([1, [2, [3, 4]], 5], 0)", "[1, [2, [3, 4]], 5]"),
        ("flatten([1, [2, [3, 4]], 5], 2)", "[1, 2, 3, 4, 5]"),
        ("flatten([[[[1]]]], 10)", "[1]"),
        ("flatten([[[[1]]]], 9223372036854775807)", "[1]"),
    ]);
}

/// A deeply nested Array flattens in order, to every level or to a depth, on a small
/// stack (see [`deep`](crate::deep)). How deep comes from the host as `levels`, so that
/// the Array is built when the script runs rather than by the compiler.
#[test]
fn flatten_flattens_a_deeply_nested_array() {
    let source = r"
        defn nest(v, n) -> if n == 0: v else: nest([v], n - 1)
        defn spine(v, n) -> if n == 0: v else: spine([n, v, -n], n - 1)
        def flat = flatten(spine(0, levels))
        [
            flatten(nest(1, levels)) == [1],
            flatten(nest(1, levels), levels - 2) == [[1]],
            len(flat) == 2 * levels + 1,
            [flat[0], flat[levels - 1], flat[levels], flat[levels + 1], flat[-1]]
                == [1, levels, 0, -levels, -1],
        ]
    ";
    let tail = crate::deep::on_a_small_stack(|| {
        Script::new(source)
            .capture("levels", Value::from(10_000))
            .run()
    });
    assert_eq!(tail, Value::from(vec![Value::from(true); 4]), "{source}");
}

#[test]
fn flatten_checks_its_arguments() {
    assert_raises(&[(
        "flatten([1], -1)",
        "Function flatten requires argument 2 (depth) to be at least 0, got -1",
    )]);
    assert_type_errors(
        "flatten",
        &[
            ("flatten({})", "Array", "argument 1", "Map"),
            ("flatten([1], 1.0)", "Int", "argument 2 (depth)", "Float"),
        ],
    );
    assert_arity("flatten", "between 1 and 2", &[0, 3]);
}

#[test]
fn flat_map_maps_then_flattens_one_level() {
    assert_values(&[
        (
            "flat_map([1, 2, 3], fn x -> [x, x * 2])",
            "[1, 2, 2, 4, 3, 6]",
        ),
        ("flat_map([1, 2], fn x -> [])", "[]"),
        ("flat_map([], fn x -> [x])", "[]"),
        // A result that is not an Array is kept as is.
        ("flat_map([1, 2], fn x -> x)", "[1, 2]"),
        ("flat_map([1], fn x -> [[x]])", "[[1]]"),
    ]);
    assert_type_errors(
        "flat_map",
        &[
            ("flat_map({}, id)", "Array", "argument 1", "Map"),
            ("flat_map([1], 1)", "Function", "argument 2", "Int"),
        ],
    );
    assert_arity("flat_map", "2", &[0, 1, 3]);
}

#[test]
fn flat_map_raises_what_its_function_raises() {
    let script = script("flat_map([1, 2], fn x -> { print(x); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["1"], "the first error ends the mapping");
}

// --- reject ---

#[test]
fn reject_keeps_what_its_predicate_rejects() {
    assert_values(&[
        ("reject([1, 2, 3, 4], fn x -> x % 2 == 0)", "[1, 3]"),
        ("reject([], fn x -> true)", "[]"),
        // Only Null and False are falsy.
        ("reject([0, null, false, ''], id)", "[null, false]"),
        ("reject({a: 1, b: 2}, fn k, v -> v > 1)", "{a: 1}"),
        ("reject({a: 1}, fn k, v -> k == 'a')", "{}"),
    ]);
}

#[test]
fn reject_is_the_complement_of_select() {
    assert_values(&[(
        "filter([1, 2, 3, 4, 5], fn x -> x > 2) + reject([1, 2, 3, 4, 5], fn x -> x > 2)",
        "[3, 4, 5, 1, 2]",
    )]);
}

#[test]
fn reject_raises_what_its_predicate_raises() {
    assert_raises(&[("reject({a: 1}, fn k, v -> error('boom'))", "boom")]);
    let script = script("reject([1, 2], fn x -> { print(x); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["1"], "the first error ends the loop");
}

#[test]
fn reject_checks_its_arguments() {
    assert_type_errors(
        "reject",
        &[
            ("reject(5, id)", "Structured", "argument 1", "Int"),
            (
                "reject([1], 1)",
                "Function",
                "argument 2 (predicate)",
                "Int",
            ),
        ],
    );
    assert_arity("reject", "2", &[0, 1, 3]);
}

// --- sum, product ---

#[test]
fn sum_adds_the_elements() {
    assert_values(&[
        ("sum([1, 2, 3, 4])", "10"),
        ("sum([1.5, 2.5])", "4.0"),
        ("sum([5])", "5"),
        ("sum([])", "null"),
        // `+` concatenates too.
        ("sum(['a', 'b'])", "'ab'"),
        ("sum([[1], [2]])", "[1, 2]"),
    ]);
    assert_raises_as(&[("sum([1, 'a'])", "1 + 'a'")]);
}

#[test]
fn product_multiplies_the_elements() {
    assert_values(&[
        ("product([1, 2, 3, 4])", "24"),
        ("product([2, 3.0])", "6.0"),
        ("product([5])", "5"),
        ("product([])", "null"),
    ]);
    assert_raises_as(&[("product([2, 'a'])", "2 * 'a'")]);
}

/// A sum or product of Ints reaching either end of the Int range is that Int;
/// one past an end raises the overflow error that `+` or `*` raises.
#[test]
fn sum_and_product_raise_on_int_overflow() {
    assert_values(&[
        ("sum([9223372036854775806, 1])", "9223372036854775807"),
        (
            "sum([-9223372036854775807, -1])",
            "-9223372036854775807 - 1",
        ),
        ("product([7, 1317624576693539401])", "9223372036854775807"),
        (
            "product([-2, 4611686018427387904])",
            "-9223372036854775807 - 1",
        ),
    ]);
    assert_raises_as(&[
        ("sum([9223372036854775807, 1])", "9223372036854775807 + 1"),
        (
            "sum([-9223372036854775807 - 1, -1])",
            "(-9223372036854775807 - 1) + -1",
        ),
        (
            "product([2, 4611686018427387904])",
            "2 * 4611686018427387904",
        ),
        (
            "product([-3, 4611686018427387904])",
            "-3 * 4611686018427387904",
        ),
    ]);
}

#[test]
fn sum_and_product_are_left_to_right_reductions() {
    // `sum` is `reduce` with `+`, and `product` with `*`, so an overflowing
    // running total raises even when a later element would bring the whole back
    // in range.
    assert_raises_as(&[
        (
            "sum([9223372036854775807, 1, -1])",
            "9223372036854775807 + 1",
        ),
        (
            "product([4611686018427387904, 2, 0])",
            "4611686018427387904 * 2",
        ),
    ]);
    // In the other order, every running total fits.
    assert_values(&[
        ("sum([9223372036854775807, -1, 1])", "9223372036854775807"),
        ("product([4611686018427387904, 0, 2])", "0"),
    ]);
}

#[test]
fn sum_and_product_check_their_argument() {
    for function in ["sum", "product"] {
        assert_type_errors(
            function,
            &[
                (&format!("{function}(5)"), "Array", "argument 1", "Int"),
                (
                    &format!("{function}({{a: 1}})"),
                    "Array",
                    "argument 1",
                    "Map",
                ),
            ],
        );
        assert_arity(function, "1", &[0, 2]);
    }
}

// --- sorted, sort_by ---

#[test]
fn sorted_orders_by_less_than() {
    assert_values(&[
        ("sorted([3, 1, 2])", "[1, 2, 3]"),
        ("sorted([])", "[]"),
        ("sorted([1])", "[1]"),
        ("sorted([2, 1.5, 1, -0.5])", "[-0.5, 1, 1.5, 2]"),
        ("sorted(['b', 'c', 'a'])", "['a', 'b', 'c']"),
        ("sorted([x'02', x'01ff', x'01'])", "[x'01', x'01ff', x'02']"),
        ("sorted([[2], [1, 5], [1]])", "[[1], [1, 5], [2]]"),
    ]);
}

#[test]
fn sorted_orders_a_long_array() {
    assert_values(&[
        ("sorted(reverse(range(1000)))", "range(1000)"),
        // A permutation of 0 to 199, as 73 and 200 are coprime.
        (
            "sorted(map(range(200), fn i -> i * 73 % 200))",
            "range(200)",
        ),
    ]);
}

#[test]
fn sorted_is_stable() {
    assert_values(&[
        // An Int and a Float of equal value are neither less than the other.
        ("sorted([1.0, 1, 0])", "[0, 1.0, 1]"),
        ("sorted([1, 1.0, 0])", "[0, 1, 1.0]"),
        (
            "sorted([[1, 'a'], [0, 'b'], [1, 'c'], [0, 'd']], fn x, y -> x[0] < y[0])",
            "[[0, 'b'], [0, 'd'], [1, 'a'], [1, 'c']]",
        ),
    ]);
}

#[test]
fn sorted_orders_by_a_comparator() {
    assert_values(&[
        ("sorted([1, 3, 2], fn a, b -> a > b)", "[3, 2, 1]"),
        (
            "sorted(['bb', 'a', 'ccc'], fn a, b -> len(a) < len(b))",
            "['a', 'bb', 'ccc']",
        ),
        // The comparator's answer is tested for truthiness.
        (
            "sorted([3, 1, 2], fn a, b -> if a < b: 0 else: null)",
            "[1, 2, 3]",
        ),
    ]);
}

#[test]
fn sorted_survives_an_inconsistent_comparator() {
    assert_values(&[
        // Never "before": every element stays where it was.
        ("sorted([3, 1, 2], fn a, b -> false)", "[3, 1, 2]"),
        // Any answer still yields every element exactly once.
        (
            "sorted(sorted([3, 1, 2, 5, 4], fn a, b -> true))",
            "[1, 2, 3, 4, 5]",
        ),
        (
            "sorted(sorted(range(100), fn a, b -> (a * 7 + b * 3) % 5 < 2))",
            "range(100)",
        ),
    ]);
}

#[test]
fn sorted_raises_when_elements_cannot_be_ordered() {
    assert_raises(&[
        ("sorted([null, null])", "Type Null is not orderable"),
        ("sorted([{}, {a: 1}])", "Type Map is not orderable"),
    ]);
    // Mixed types raise as `<` does.
    assert_raises_as(&[("sorted([1, 'a'])", "'a' < 1")]);
}

#[test]
fn sorted_raises_what_its_comparator_raises() {
    assert_raises(&[("sorted([1, 2], fn a, b -> error('boom'))", "boom")]);
    assert_raises_as(&[("sorted([1, 2], fn a -> true)", "(fn a -> true)(1, 2)")]);
    // The first error ends the sort: the comparator is not called again.
    let script = Script::new("sorted([4, 3, 2, 1], fn a, b -> { print('called'); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["called"]);
}

#[test]
fn sorted_checks_its_arguments() {
    assert_type_errors(
        "sorted",
        &[
            ("sorted({})", "Array", "argument 1", "Map"),
            ("sorted('cba')", "Array", "argument 1", "String"),
            (
                "sorted([1], 1)",
                "Function",
                "argument 2 (comparator)",
                "Int",
            ),
        ],
    );
    assert_arity("sorted", "between 1 and 2", &[0, 3]);
}

#[test]
fn sort_by_orders_by_a_projection() {
    assert_values(&[
        (
            "sort_by([{n: 'b'}, {n: 'a'}], index('n'))",
            "[{n: 'a'}, {n: 'b'}]",
        ),
        ("sort_by(['ccc', 'a', 'bb'], len)", "['a', 'bb', 'ccc']"),
        ("sort_by([1, 3, 2], fn x -> -x)", "[3, 2, 1]"),
        ("sort_by([], id)", "[]"),
        // Stable: elements with equal keys keep their order.
        (
            "sort_by(['bb', 'a', 'cc', 'd'], len)",
            "['a', 'd', 'bb', 'cc']",
        ),
    ]);
}

#[test]
fn sort_by_projects_each_element_once() {
    let source = "sort_by([3, 1, 2], fn x -> { print(x); x })";
    assert_values(&[(source, "[1, 2, 3]")]);
    assert_eq!(printed(source), ["3", "1", "2"]);
}

#[test]
fn sort_by_raises_when_keys_cannot_be_ordered() {
    assert_raises(&[(
        "sort_by([1, 2], fn x -> null)",
        "Type Null is not orderable",
    )]);
    // A single key is never compared, so it need not be orderable.
    assert_values(&[("sort_by([1], fn x -> null)", "[1]")]);
}

#[test]
fn sort_by_raises_what_its_projection_raises() {
    let script = script("sort_by([1, 2], fn x -> { print(x); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(
        script.printed(),
        ["1"],
        "the first error ends the projecting"
    );
}

#[test]
fn sort_by_checks_its_arguments() {
    assert_type_errors(
        "sort_by",
        &[
            ("sort_by({}, id)", "Array", "argument 1", "Map"),
            (
                "sort_by([1], 1)",
                "Function",
                "argument 2 (projection)",
                "Int",
            ),
        ],
    );
    assert_arity("sort_by", "2", &[0, 1, 3]);
}

#[test]
fn sorting_leaves_the_original_unchanged() {
    let source = r"
        def a = [3, 1, 2]
        [sorted(a), sort_by(a, id), a]
    ";
    assert_values(&[(source, "[[1, 2, 3], [1, 2, 3], [3, 1, 2]]")]);
}

// --- minimum, maximum ---

#[test]
fn minimum_and_maximum_find_the_extremes_under_less_than() {
    assert_values(&[
        ("minimum([3, 1, 2])", "1"),
        ("maximum([3, 1, 2])", "3"),
        ("minimum([5])", "5"),
        ("maximum([5])", "5"),
        ("minimum([2, 1.5, 3, -0.5])", "-0.5"),
        ("maximum([2, 1.5, 3, -0.5])", "3"),
        ("minimum(['b', 'c', 'a'])", "'a'"),
        ("maximum(['b', 'c', 'a'])", "'c'"),
        ("minimum([x'02', x'01ff', x'01'])", "x'01'"),
        ("maximum([[2], [1, 5], [1]])", "[2]"),
    ]);
}

#[test]
fn minimum_and_maximum_of_nothing_are_null() {
    assert_values(&[("minimum([])", "null"), ("maximum([])", "null")]);
}

#[test]
fn minimum_and_maximum_keep_the_first_of_a_tie() {
    assert_values(&[
        // An Int and a Float of equal value are neither less than the other.
        ("minimum([1.0, 1, 2])", "1.0"),
        ("minimum([1, 1.0, 2])", "1"),
        ("maximum([1.0, 1, 0])", "1.0"),
        ("maximum([1, 1.0, 0])", "1"),
        (
            "minimum(['bb', 'a', 'c'], fn a, b -> len(a) < len(b))",
            "'a'",
        ),
        (
            "maximum(['a', 'bb', 'cc'], fn a, b -> len(a) < len(b))",
            "'bb'",
        ),
    ]);
}

#[test]
fn minimum_and_maximum_compare_ints_and_floats_exactly() {
    // 2^53 + 1 is not a Float; rounded to one, it would tie with 2^53.
    assert_values(&[
        (
            "maximum([9007199254740992.0, 9007199254740993])",
            "9007199254740993",
        ),
        (
            "minimum([9007199254740993, 9007199254740992.0])",
            "9007199254740992.0",
        ),
    ]);
}

#[test]
fn minimum_and_maximum_search_a_long_array() {
    assert_values(&[
        // A permutation of 0 to 199, as 73 and 200 are coprime.
        ("minimum(map(range(200), fn i -> i * 73 % 200))", "0"),
        ("maximum(map(range(200), fn i -> i * 73 % 200))", "199"),
        ("minimum(reverse(range(1000)))", "0"),
        ("maximum(range(1000))", "999"),
    ]);
}

#[test]
fn minimum_and_maximum_order_by_a_comparator() {
    assert_values(&[
        // A comparator answers whether its first argument comes before its
        // second, as `sorted`'s does: here, greatest first.
        ("minimum([1, 3, 2], fn a, b -> a > b)", "3"),
        ("maximum([1, 3, 2], fn a, b -> a > b)", "1"),
        (
            "minimum(['bb', 'a', 'ccc'], fn a, b -> len(a) < len(b))",
            "'a'",
        ),
        (
            "maximum(['bb', 'a', 'ccc'], fn a, b -> len(a) < len(b))",
            "'ccc'",
        ),
        // The comparator's answer is tested for truthiness.
        ("minimum([3, 1, 2], fn a, b -> if a < b: 0 else: null)", "1"),
        ("minimum([], fn a, b -> error('never called'))", "null"),
        ("maximum([7], fn a, b -> error('never called'))", "7"),
        // A comparator may order what `<` cannot.
        ("minimum([{n: 2}, {n: 1}], fn a, b -> a.n < b.n)", "{n: 1}"),
    ]);
}

#[test]
fn minimum_and_maximum_agree_with_sorted_under_a_comparator() {
    let source = r"
        def words = ['pear', 'fig', 'banana', 'kiwi', 'apple']
        defn shorter(a, b) -> len(a) < len(b)
        [minimum(words, shorter) == sorted(words, shorter)[0], maximum(words, shorter)]
    ";
    // The longest word is unique, so ties do not enter into it.
    assert_values(&[(source, "[true, 'banana']")]);
}

#[test]
fn minimum_and_maximum_survive_an_inconsistent_comparator() {
    assert_values(&[
        // Never "before": the first element stands.
        ("minimum([3, 1, 2], fn a, b -> false)", "3"),
        ("maximum([3, 1, 2], fn a, b -> false)", "3"),
        // Any answer still yields one of the elements.
        (
            "contains([3, 1, 2], minimum([3, 1, 2], fn a, b -> true))",
            "true",
        ),
        (
            "contains([3, 1, 2], maximum([3, 1, 2], fn a, b -> true))",
            "true",
        ),
    ]);
}

#[test]
fn minimum_and_maximum_raise_when_elements_cannot_be_ordered() {
    for function in ["minimum", "maximum"] {
        assert_raises(&[
            (
                &format!("{function}([null, null])"),
                "Type Null is not orderable",
            ),
            (
                &format!("{function}([{{}}, {{a: 1}}])"),
                "Type Map is not orderable",
            ),
        ]);
        // A single element is never compared, so it need not be orderable.
        assert_values(&[(&format!("{function}([{{a: 1}}])"), "{a: 1}")]);
    }
    // Mixed types raise as `<` does.
    assert_raises_as(&[
        ("minimum([1, 'a'])", "'a' < 1"),
        ("maximum([1, 'a'])", "1 < 'a'"),
    ]);
}

#[test]
fn minimum_and_maximum_raise_what_their_comparator_raises() {
    for function in ["minimum", "maximum"] {
        assert_raises(&[(
            &format!("{function}([1, 2], fn a, b -> error('boom'))"),
            "boom",
        )]);
        // The first error ends the search: the comparator is not called again.
        let script = script(&format!(
            "{function}([4, 3, 2, 1], fn a, b -> {{ print('called'); error('boom') }})"
        ));
        assert_eq!(script.raises(), "boom");
        assert_eq!(script.printed(), ["called"]);
    }
}

#[test]
fn minimum_and_maximum_check_their_arguments() {
    for function in ["minimum", "maximum"] {
        assert_type_errors(
            function,
            &[
                (&format!("{function}({{}})"), "Array", "argument 1", "Map"),
                (
                    &format!("{function}('cba')"),
                    "Array",
                    "argument 1",
                    "String",
                ),
                (&format!("{function}(3, 5)"), "Array", "argument 1", "Int"),
                (
                    &format!("{function}([1], 1)"),
                    "Function",
                    "argument 2 (comparator)",
                    "Int",
                ),
            ],
        );
        assert_arity(function, "between 1 and 2", &[0, 3]);
    }
}

// --- min_by, max_by ---

#[test]
fn min_by_and_max_by_return_the_element_with_the_extreme_key() {
    assert_values(&[
        ("min_by(['ccc', 'a', 'bb'], len)", "'a'"),
        ("max_by(['ccc', 'a', 'bb'], len)", "'ccc'"),
        ("min_by([{n: 2}, {n: 1}], index('n'))", "{n: 1}"),
        ("max_by([{n: 2}, {n: 1}], index('n'))", "{n: 2}"),
        ("min_by([1, -3, 2], fn x -> -x)", "2"),
        ("max_by([1, -3, 2], fn x -> -x)", "-3"),
        ("min_by([5], fn x -> null)", "5"),
        ("min_by([], id)", "null"),
        ("max_by([], id)", "null"),
    ]);
}

#[test]
fn min_by_and_max_by_keep_the_first_of_a_tie() {
    assert_values(&[
        ("min_by(['bb', 'a', 'c'], len)", "'a'"),
        ("max_by(['aa', 'b', 'cc'], len)", "'aa'"),
        // Keys of equal value but different type tie too.
        (
            "min_by(['x', 'y'], fn s -> if s == 'x': 1 else: 1.0)",
            "'x'",
        ),
        (
            "max_by(['x', 'y'], fn s -> if s == 'x': 1.0 else: 1)",
            "'x'",
        ),
    ]);
}

#[test]
fn min_by_and_max_by_project_each_element_once() {
    for function in ["min_by", "max_by"] {
        let source = format!("{function}([3, 1, 2], fn x -> {{ print(x); x }})");
        assert_eq!(printed(&source), ["3", "1", "2"], "{function}");
    }
}

#[test]
fn min_by_and_max_by_raise_when_keys_cannot_be_ordered() {
    for function in ["min_by", "max_by"] {
        assert_raises(&[(
            &format!("{function}([1, 2], fn x -> null)"),
            "Type Null is not orderable",
        )]);
    }
    assert_raises_as(&[("min_by([1, 2], fn x -> if x == 1: 1 else: 'a')", "'a' < 1")]);
}

#[test]
fn min_by_and_max_by_raise_what_their_projection_raises() {
    for function in ["min_by", "max_by"] {
        let script = script(&format!(
            "{function}([1, 2], fn x -> {{ print(x); error('boom') }})"
        ));
        assert_eq!(script.raises(), "boom");
        assert_eq!(
            script.printed(),
            ["1"],
            "the first error ends the projecting"
        );
    }
}

#[test]
fn min_by_and_max_by_check_their_arguments() {
    for function in ["min_by", "max_by"] {
        assert_type_errors(
            function,
            &[
                (
                    &format!("{function}({{}}, id)"),
                    "Array",
                    "argument 1",
                    "Map",
                ),
                (
                    &format!("{function}([1], 1)"),
                    "Function",
                    "argument 2 (projection)",
                    "Int",
                ),
            ],
        );
        assert_arity(function, "2", &[0, 1, 3]);
    }
}

#[test]
fn extremes_leave_the_original_unchanged() {
    let source = r"
        def a = [3, 1, 2]
        [minimum(a), maximum(a), min_by(a, id), max_by(a, id), a]
    ";
    assert_values(&[(source, "[1, 3, 1, 3, [3, 1, 2]]")]);
}

// --- unique, unique_by ---

/// An Opaque whose `==` compares only its `key`, so two equal Tags can still
/// be told apart by `id` in how they render.
#[derive(Debug)]
struct Tag {
    key: u32,
    id: u32,
}

impl FrostOpaque for Tag {
    fn type_name() -> &'static str {
        "Tag"
    }

    fn try_to_string(&self) -> Option<String> {
        Some(format!("{}#{}", self.key, self.id))
    }

    fn equals(&self, other: &dyn FrostOpaque) -> bool {
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| self.key == other.key)
    }
}

#[test]
fn unique_keeps_the_first_of_each_value_in_order() {
    assert_values(&[
        ("unique([1, 2, 1, 3, 2])", "[1, 2, 3]"),
        ("unique([3, 3, 3])", "[3]"),
        ("unique([])", "[]"),
        ("unique(['b', 'a', 'b'])", "['b', 'a']"),
        ("unique([true, false, true])", "[true, false]"),
        ("unique([x'01', x'01', x'02'])", "[x'01', x'02']"),
        ("unique(map(range(1000), fn i -> i % 7))", "range(7)"),
    ]);
}

#[test]
fn unique_follows_equality() {
    assert_values(&[
        // `1 == 1.0` is false, and a String never equals Bytes.
        ("unique([1, 1.0, 1, 1.0])", "[1, 1.0]"),
        ("unique(['a', x'61', 'a'])", "['a', x'61']"),
        // `0.0 == -0.0` is false.
        ("unique([0.0, -0.0, 0.0, -0.0])", "[0.0, -0.0]"),
    ]);
}

#[test]
fn unique_compares_values_that_cannot_be_map_keys() {
    assert_values(&[
        ("unique([null, null])", "[null]"),
        ("unique([[1], [1], [2], [1]])", "[[1], [2]]"),
        ("unique([{a: 1}, {a: 2}, {a: 1}])", "[{a: 1}, {a: 2}]"),
        // Keys and other values mixed, in order.
        ("unique([1, [1], null, 1, [1], null])", "[1, [1], null]"),
        (
            r"
            def f = fn x -> x
            len(unique([f, f, id, id]))
            ",
            "2",
        ),
    ]);
}

#[test]
fn unique_uses_an_opaque_types_own_equality() {
    let tag = |key, id| Value::opaque(Tag { key, id });
    let rendered = Script::new("map(unique([a, b, c, d]), to_string)")
        .capture("a", tag(1, 1))
        .capture("b", tag(2, 2))
        .capture("c", tag(1, 3))
        .capture("d", tag(2, 4))
        .run();
    assert_eq!(rendered, Value::array(["<Tag: 1#1>", "<Tag: 2#2>"]));
}

#[test]
fn unique_checks_its_argument() {
    assert_type_errors(
        "unique",
        &[
            ("unique({})", "Array", "argument 1", "Map"),
            ("unique('aab')", "Array", "argument 1", "String"),
            ("unique(null)", "Array", "argument 1", "Null"),
        ],
    );
    assert_arity("unique", "1", &[0, 2]);
}

#[test]
fn unique_by_keeps_the_first_element_of_each_key() {
    assert_values(&[
        (
            "unique_by(['a', 'bb', 'c', 'dd', 'eee'], len)",
            "['a', 'bb', 'eee']",
        ),
        ("unique_by([], id)", "[]"),
        ("unique_by([1, 2, 3], fn x -> null)", "[1]"),
        (
            "unique_by([{n: 1, s: 'x'}, {n: 2, s: 'y'}, {n: 1, s: 'z'}], index('n'))",
            "[{n: 1, s: 'x'}, {n: 2, s: 'y'}]",
        ),
        // Keys that cannot be Map keys compare by equality too.
        (
            "unique_by([[1, 'x'], [1, 'y'], [2, 'z']], fn p -> [p[0]])",
            "[[1, 'x'], [2, 'z']]",
        ),
        // Keys of equal value but different type are distinct.
        (
            "unique_by([1, 2], fn x -> if x == 1: 1 else: 1.0)",
            "[1, 2]",
        ),
    ]);
}

#[test]
fn unique_by_projects_each_element_once() {
    let source = "unique_by([3, 1, 3], fn x -> { print(x); x })";
    assert_values(&[(source, "[3, 1]")]);
    assert_eq!(printed(source), ["3", "1", "3"]);
}

#[test]
fn unique_by_raises_what_its_projection_raises() {
    let script = script("unique_by([1, 2], fn x -> { print(x); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(
        script.printed(),
        ["1"],
        "the first error ends the projecting"
    );
}

#[test]
fn unique_by_checks_its_arguments() {
    assert_type_errors(
        "unique_by",
        &[
            ("unique_by({}, id)", "Array", "argument 1", "Map"),
            (
                "unique_by([1], 1)",
                "Function",
                "argument 2 (projection)",
                "Int",
            ),
        ],
    );
    assert_arity("unique_by", "2", &[0, 1, 3]);
}

#[test]
fn unique_leaves_the_original_unchanged() {
    let source = r"
        def a = [1, 2, 1]
        [unique(a), unique_by(a, id), a]
    ";
    assert_values(&[(source, "[[1, 2], [1, 2], [1, 2, 1]]")]);
}

// --- group_by, count_by ---

#[test]
fn group_by_groups_elements_by_key() {
    assert_values(&[
        (
            "group_by([1, 2, 3, 4, 5], fn x -> x % 2)",
            "{[0]: [2, 4], [1]: [1, 3, 5]}",
        ),
        ("group_by([], id)", "{}"),
        // Each group keeps its elements' order.
        (
            "group_by(['a', 'bc', 'd', 'ef'], len)",
            "{[1]: ['a', 'd'], [2]: ['bc', 'ef']}",
        ),
        (
            "group_by([1, 2, 3], fn x -> if x > 1: 'big' else: 'small')",
            "{big: [2, 3], small: [1]}",
        ),
    ]);
}

#[test]
fn count_by_counts_elements_by_key() {
    assert_values(&[
        (
            "count_by(['a', 'b', 'a', 'c', 'a'], id)",
            "{a: 3, b: 1, c: 1}",
        ),
        ("count_by([], id)", "{}"),
        (
            "count_by([1, 2, 3, 4], fn x -> x > 2)",
            "{[true]: 2, [false]: 2}",
        ),
    ]);
}

#[test]
fn group_by_and_count_by_require_a_valid_key_from_their_function() {
    for function in ["group_by", "count_by"] {
        for (returned, got) in [("null", "Null"), ("[x]", "Array"), ("{a: x}", "Map")] {
            assert_raises(&[(
                &format!("{function}([1], fn x -> {returned})"),
                &format!(
                    "Function {function} requires its function to return a valid Map key, \
                     got {got}"
                ),
            )]);
        }
    }
}

#[test]
fn group_by_and_count_by_raise_what_their_function_raises() {
    for function in ["group_by", "count_by"] {
        let script = script(&format!(
            "{function}([1, 2], fn x -> {{ print(x); error('boom') }})"
        ));
        assert_eq!(script.raises(), "boom", "{function}");
        assert_eq!(script.printed(), ["1"], "{function} stops at the error");
    }
}

#[test]
fn group_by_and_count_by_check_their_arguments() {
    for function in ["group_by", "count_by"] {
        assert_type_errors(
            function,
            &[
                (
                    &format!("{function}({{}}, id)"),
                    "Array",
                    "argument 1",
                    "Map",
                ),
                (
                    &format!("{function}([1], 1)"),
                    "Function",
                    "argument 2 (projection)",
                    "Int",
                ),
            ],
        );
        assert_arity(function, "2", &[0, 1, 3]);
    }
}

// --- scan, partition, map_into ---

#[test]
fn scan_returns_each_running_result() {
    assert_values(&[
        ("scan([1, 2, 3, 4], plus)", "[1, 3, 6, 10]"),
        ("scan([10, 1, 2], minus)", "[10, 9, 7]"),
        ("scan([5], plus)", "[5]"),
        ("scan([], plus)", "[]"),
    ]);
}

#[test]
fn scan_starts_from_an_initializer() {
    // Each result is what `reduce` gives over the elements so far, so the
    // initializer is not itself a result, and every element gets one.
    assert_values(&[
        ("scan([1, 2, 3], 10, plus)", "[11, 13, 16]"),
        ("scan([1, 2, 3], 10, minus)", "[9, 7, 4]"),
        ("scan(['a', 'b'], '', plus)", "['a', 'ab']"),
        ("scan([5], 1, plus)", "[6]"),
        ("scan([], 0, plus)", "[]"),
        // A Null initializer is an initializer.
        ("scan([1], null, fn acc, x -> [acc, x])", "[[null, 1]]"),
        // A Function initializer is an initializer: with three arguments,
        // the second is always the initializer.
        ("scan([1], plus, fn acc, x -> acc)", "[plus]"),
        ("[1, 2, 3] @ scan(0, plus)", "[1, 3, 6]"),
        // Each result agrees with `reduce` over the elements so far.
        (
            "map(range(1, 4), fn n -> reduce(take([4, 5, 6], n), 1, times))",
            "scan([4, 5, 6], 1, times)",
        ),
    ]);
}

#[test]
fn scan_checks_its_arguments() {
    assert_type_errors(
        "scan",
        &[
            ("scan({}, plus)", "Array", "argument 1", "Map"),
            ("scan({}, 0, plus)", "Array", "argument 1", "Map"),
            // With two arguments, the second is the function, never an initializer.
            ("scan([1], 1)", "Function", "argument 2", "Int"),
            // With three, the function is the third.
            ("scan([1], 0, 1)", "Function", "argument 3", "Int"),
            ("scan([1], plus, 0)", "Function", "argument 3", "Int"),
        ],
    );
    assert_arity("scan", "between 2 and 3", &[0, 1, 4]);
}

#[test]
fn scan_raises_what_its_function_raises() {
    // The first element starts the scan, so the function first sees the second.
    let unseeded = script("scan([1, 2, 3], fn a, b -> { print(b); error('boom') })");
    assert_eq!(unseeded.raises(), "boom");
    assert_eq!(unseeded.printed(), ["2"], "the first error ends the scan");
    // With an initializer, the function first sees the first element.
    let seeded = script("scan([1, 2, 3], 0, fn a, b -> { print(b); error('boom') })");
    assert_eq!(seeded.raises(), "boom");
    assert_eq!(seeded.printed(), ["1"], "the first error ends the scan");
}

#[test]
fn partition_splits_by_a_predicate() {
    assert_values(&[
        (
            "partition([1, 2, 3, 4, 5], fn x -> x % 2 == 1)",
            "{pass: [1, 3, 5], fail: [2, 4]}",
        ),
        ("partition([], id)", "{pass: [], fail: []}"),
        (
            "partition([0, null, false], id)",
            "{pass: [0], fail: [null, false]}",
        ),
    ]);
    assert_type_errors(
        "partition",
        &[
            ("partition({}, id)", "Array", "argument 1", "Map"),
            (
                "partition([1], 1)",
                "Function",
                "argument 2 (predicate)",
                "Int",
            ),
        ],
    );
    assert_arity("partition", "2", &[0, 1, 3]);
}

#[test]
fn partition_raises_what_its_predicate_raises() {
    let script = script("partition([1, 2], fn x -> { print(x); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["1"], "the first error ends the loop");
}

#[test]
fn map_into_merges_the_maps_its_function_returns() {
    assert_values(&[
        (
            "map_into(['a', 'bc'], fn s -> {[s]: len(s)})",
            "{a: 1, bc: 2}",
        ),
        (
            "map_into(range(3), fn n -> {[n]: n * n})",
            "{[0]: 0, [1]: 1, [2]: 4}",
        ),
        ("map_into([1], fn n -> {a: n, b: n})", "{a: 1, b: 1}"),
        ("map_into([], fn n -> {a: n})", "{}"),
        // A later entry replaces an earlier one with the same key.
        ("map_into([1, 2], fn n -> {k: n})", "{k: 2}"),
    ]);
    assert_raises(&[(
        "map_into([1], id)",
        "Function map_into requires its function to return a Map, got Int",
    )]);
    assert_type_errors(
        "map_into",
        &[
            ("map_into({}, id)", "Array", "argument 1", "Map"),
            ("map_into([1], 1)", "Function", "argument 2", "Int"),
        ],
    );
    assert_arity("map_into", "2", &[0, 1, 3]);
}

#[test]
fn map_into_raises_what_its_function_raises() {
    let script = script("map_into([1, 2], fn x -> { print(x); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["1"], "the first error ends the loop");
}

// --- chunk_by ---

#[test]
fn chunk_by_groups_runs_of_adjacent_elements() {
    assert_values(&[
        (
            "chunk_by([1, 1, 2, 2, 1], fn a, b -> a == b)",
            "[[1, 1], [2, 2], [1]]",
        ),
        (
            "chunk_by([1, 2, 3, 2, 3, 1], fn a, b -> a < b)",
            "[[1, 2, 3], [2, 3], [1]]",
        ),
        ("chunk_by([1, 2, 3], fn a, b -> true)", "[[1, 2, 3]]"),
        ("chunk_by([1, 2, 3], fn a, b -> false)", "[[1], [2], [3]]"),
        ("chunk_by([5], fn a, b -> false)", "[[5]]"),
        ("chunk_by([], fn a, b -> true)", "[]"),
    ]);
}

#[test]
fn chunk_by_asks_about_each_adjacent_pair_once() {
    let source = r"chunk_by([1, 2, 3], fn a, b -> { print($'${a},${b}'); true })";
    assert_values(&[(source, "[[1, 2, 3]]")]);
    assert_eq!(printed(source), ["1,2", "2,3"]);
    let source = "chunk_by([1], fn a, b -> { print('called'); true })";
    assert_values(&[(source, "[[1]]")]);
    assert!(
        printed(source).is_empty(),
        "a single element has no pair to ask about"
    );
}

#[test]
fn chunk_by_raises_what_its_function_raises() {
    let script = script("chunk_by([1, 2, 3], fn a, b -> { print(a); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["1"], "the first error ends the loop");
}

#[test]
fn chunk_by_checks_its_arguments() {
    assert_type_errors(
        "chunk_by",
        &[
            ("chunk_by({}, id)", "Array", "argument 1", "Map"),
            (
                "chunk_by([1], 1)",
                "Function",
                "argument 2 (predicate)",
                "Int",
            ),
        ],
    );
    assert_arity("chunk_by", "2", &[0, 1, 3]);
}

// --- zip, zip_with ---

#[test]
fn zip_pairs_up_elements() {
    assert_values(&[
        (
            "zip([1, 2, 3], ['a', 'b', 'c'])",
            "[[1, 'a'], [2, 'b'], [3, 'c']]",
        ),
        ("zip([1, 2], [3, 4], [5, 6])", "[[1, 3, 5], [2, 4, 6]]"),
        // Rows stop at the shortest Array.
        ("zip([1, 2, 3], [4])", "[[1, 4]]"),
        ("zip([1], [2, 3], [4, 5, 6])", "[[1, 2, 4]]"),
        ("zip([], [1])", "[]"),
        // Elements are taken whole.
        ("zip([[1]], [2])", "[[[1], 2]]"),
    ]);
}

#[test]
fn zip_with_combines_elements() {
    assert_values(&[
        (
            "zip_with(fn a, b -> a + b, [1, 2, 3], [10, 20, 30])",
            "[11, 22, 33]",
        ),
        (
            "zip_with(fn a, b, c -> a * b + c, [2, 3, 4], [10, 20, 30], [1, 1])",
            "[21, 61]",
        ),
        ("zip_with(collect, [1, 2], [3, 4])", "zip([1, 2], [3, 4])"),
        ("zip_with(plus, [], [1])", "[]"),
    ]);
    let source = "zip_with(fn a, b -> { print(a + b); a * b }, [1, 2], [10, 20])";
    assert_values(&[(source, "[10, 40]")]);
    assert_eq!(printed(source), ["11", "22"]);
}

#[test]
fn zip_with_raises_what_its_function_raises() {
    assert_raises(&[("zip_with(fn a, b -> error('boom'), [1], [2])", "boom")]);
    assert_raises_as(&[("zip_with(fn a -> a, [1], [2])", "(fn a -> a)(1, 2)")]);
}

// --- xprod, xprod_with ---

#[test]
fn xprod_makes_every_combination() {
    assert_values(&[
        (
            "xprod([1, 2], ['a', 'b'])",
            "[[1, 'a'], [1, 'b'], [2, 'a'], [2, 'b']]",
        ),
        (
            "xprod([1, 2], [3], [4, 5])",
            "[[1, 3, 4], [1, 3, 5], [2, 3, 4], [2, 3, 5]]",
        ),
        ("len(xprod(range(3), range(4), range(5)))", "60"),
        // Any empty Array leaves no combinations.
        ("xprod([1], [])", "[]"),
        ("xprod([], [1])", "[]"),
        ("xprod([1, 2], [3], [])", "[]"),
        // Elements are taken whole.
        ("xprod([[1]], [2])", "[[[1], 2]]"),
    ]);
}

#[test]
fn xprod_with_combines_every_combination() {
    assert_values(&[
        (
            "xprod_with(fn a, b -> a + b, [10, 20], [1, 2])",
            "[11, 12, 21, 22]",
        ),
        (
            "xprod_with(fn a, b -> a + b, ['a', 'b'], ['1', '2'])",
            "['a1', 'a2', 'b1', 'b2']",
        ),
        (
            "xprod_with(collect, [1, 2], [3], [4, 5])",
            "xprod([1, 2], [3], [4, 5])",
        ),
    ]);
    let source = "xprod_with(fn a, b -> { print('called'); a }, [1], [])";
    assert_values(&[(source, "[]")]);
    assert!(printed(source).is_empty(), "no combination, no call");
}

#[test]
fn xprod_with_raises_what_its_function_raises() {
    assert_raises(&[("xprod_with(fn a, b -> error('boom'), [1], [2])", "boom")]);
    assert_raises_as(&[("xprod_with(fn a -> a, [1], [2])", "(fn a -> a)(1, 2)")]);
}

// --- zip, zip_with, xprod, xprod_with: arguments ---

#[test]
fn zip_and_xprod_require_arrays() {
    for function in ["zip", "xprod"] {
        assert_type_errors(
            function,
            &[
                (&format!("{function}(1, [2])"), "Array", "argument 1", "Int"),
                (
                    &format!("{function}([1], {{}})"),
                    "Array",
                    "argument 2",
                    "Map",
                ),
                (
                    &format!("{function}([1], [2], 'x')"),
                    "Array",
                    "argument 3",
                    "String",
                ),
            ],
        );
        assert_arity(function, "at least 2", &[0, 1]);
    }
}

#[test]
fn zip_with_and_xprod_with_require_a_function_and_arrays() {
    for function in ["zip_with", "xprod_with"] {
        assert_type_errors(
            function,
            &[
                (
                    &format!("{function}(1, [1], [2])"),
                    "Function",
                    "argument 1",
                    "Int",
                ),
                (
                    &format!("{function}(id, 1, [2])"),
                    "Array",
                    "argument 2",
                    "Int",
                ),
                (
                    &format!("{function}(id, [1], [2], null)"),
                    "Array",
                    "argument 4",
                    "Null",
                ),
            ],
        );
        assert_arity(function, "at least 3", &[0, 1, 2]);
    }
}

#[test]
fn zip_with_and_xprod_with_stop_at_the_first_error() {
    // Each call prints its first argument, and the first with a 2 raises.
    let probe = "fn a, b -> { print(a); if a == 2: error('boom') else: a }";

    // Of 1000 rows, the third raises.
    let script = Script::new(&format!("zip_with({probe}, range(1000), range(1000))"));
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["0", "1", "2"], "zip_with stops at row 3");

    // Of a million rows, row 2001 raises: the first Array's 0 and 1 each pair
    // with all 1000 of the second's elements before its 2 comes up.
    let script = Script::new(&format!("xprod_with({probe}, range(1000), range(1000))"));
    assert_eq!(script.raises(), "boom");
    let expected: Vec<&str> = ["0"; 1000]
        .into_iter()
        .chain(["1"; 1000])
        .chain(["2"])
        .collect();
    assert_eq!(script.printed(), expected, "xprod_with stops at row 2001");
}

// --- id ---

#[test]
fn id_returns_its_argument() {
    assert_values(&[
        ("id(1)", "1"),
        ("id(null)", "null"),
        ("id([1])", "[1]"),
        ("id({a: [1]})", "{a: [1]}"),
        ("id('s')", "'s'"),
        ("id(x'ff')", "x'ff'"),
        ("is_function(id(id))", "true"),
        ("id(id)(7)", "7"),
        ("map([1, 2], id)", "[1, 2]"),
    ]);
    assert_arity("id", 1, &[0, 2]);
}

// --- map, filter, reduce, each ---

#[test]
fn map_maps_an_array() {
    assert_values(&[
        ("map([1, 2, 3], fn x -> x * 2)", "[2, 4, 6]"),
        ("map([], fn x -> x)", "[]"),
        ("[1, 2] @ map(fn x -> [x])", "[[1], [2]]"),
    ]);
}

#[test]
fn map_maps_a_map_through_the_maps_its_function_returns() {
    assert_values(&[
        ("map({a: 1}, fn k, v -> {[k]: v + 1})", "{a: 2}"),
        (
            "map({a: 1, b: 2}, fn k, v -> {[v]: k})",
            "{[1]: 'a', [2]: 'b'}",
        ),
        ("map({}, fn k, v -> {x: 1})", "{}"),
        // Several entries come from one, and colliding keys leave one entry.
        ("map({a: 1}, fn k, v -> {x: v, y: v})", "{x: 1, y: 1}"),
        ("len(map({a: 1, b: 2}, fn k, v -> {same: v}))", "1"),
        ("map({a: 1}, fn k, v -> {})", "{}"),
    ]);
}

#[test]
fn map_requires_a_map_from_a_function_over_a_map() {
    assert_raises(&[
        (
            "map({a: 1}, fn k, v -> 1)",
            "When mapping a Map, the function must return a Map, got Int",
        ),
        (
            "map({a: 1}, fn k, v -> [k])",
            "When mapping a Map, the function must return a Map, got Array",
        ),
    ]);
}

#[test]
fn map_raises_what_its_function_raises() {
    let script = script("map([1, 2], fn x -> { print(x); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["1"], "the first error ends the loop");
    assert_raises(&[("map({a: 1}, fn k, v -> error('boom'))", "boom")]);
}

#[test]
fn map_checks_its_arguments() {
    assert_type_errors(
        "map",
        &[
            ("map(5, id)", "Structured", "argument 1", "Int"),
            ("map('ab', id)", "Structured", "argument 1", "String"),
            ("map([1], 1)", "Function", "argument 2", "Int"),
        ],
    );
    assert_arity("map", "2", &[0, 1, 3]);
}

#[test]
fn filter_keeps_what_its_predicate_accepts() {
    assert_values(&[
        ("filter([1, 2, 3, 4], fn x -> x % 2 == 0)", "[2, 4]"),
        ("filter([], fn x -> true)", "[]"),
        // Only Null and False are falsy.
        ("filter([0, null, false, ''], id)", "[0, '']"),
        ("filter({a: 1, b: 2}, fn k, v -> v > 1)", "{b: 2}"),
        ("filter({a: 1}, fn k, v -> false)", "{}"),
        ("[1, 2, 3] @ filter(fn x -> x > 1)", "[2, 3]"),
    ]);
}

#[test]
fn filter_raises_what_its_predicate_raises() {
    assert_raises(&[("filter({a: 1}, fn k, v -> error('boom'))", "boom")]);
    let script = script("filter([1, 2], fn x -> { print(x); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["1"], "the first error ends the loop");
}

#[test]
fn filter_checks_its_arguments() {
    assert_type_errors(
        "filter",
        &[
            ("filter(5, id)", "Structured", "argument 1", "Int"),
            (
                "filter([1], 1)",
                "Function",
                "argument 2 (predicate)",
                "Int",
            ),
        ],
    );
    assert_arity("filter", "2", &[0, 1, 3]);
}

#[test]
fn reduce_combines_an_array() {
    assert_values(&[
        ("reduce([1, 2, 3, 4], plus)", "10"),
        // The initial value starts the reduction, and the accumulator comes first.
        ("reduce([1, 2, 3], 10, plus)", "16"),
        ("reduce([1, 2, 3], 10, minus)", "4"),
        ("reduce(['a', 'b'], '', plus)", "'ab'"),
        // Without an initial value, the first element starts the reduction.
        ("reduce([10, 1, 2], minus)", "7"),
        ("reduce([5], minus)", "5"),
        ("reduce([], plus)", "null"),
        ("reduce([], 0, plus)", "0"),
        // A Null initial value is an initial value.
        ("reduce([1], null, fn acc, x -> [acc, x])", "[null, 1]"),
        // A Function initial value is an initial value: with three arguments,
        // the second is always the initial value.
        ("reduce([1], plus, fn acc, x -> acc)", "plus"),
        ("[1, 2, 3] @ reduce(0, plus)", "6"),
    ]);
}

#[test]
fn reduce_combines_a_map_from_an_initial_value() {
    assert_values(&[
        ("reduce({a: 1, b: 2}, 0, fn acc, k, v -> acc + v)", "3"),
        ("reduce({}, 0, fn acc, k, v -> acc + v)", "0"),
        (
            "reduce({a: 1}, null, fn acc, k, v -> [acc, k, v])",
            "[null, 'a', 1]",
        ),
    ]);
    assert_raises(&[
        (
            "reduce({a: 1}, fn acc, k, v -> acc)",
            "Reducing a Map requires an initial value",
        ),
        (
            "reduce({}, fn acc, k, v -> acc)",
            "Reducing a Map requires an initial value",
        ),
    ]);
}

#[test]
fn reduce_raises_what_its_function_raises() {
    let script = script("reduce([1, 2, 3], fn acc, x -> { print(x); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(
        script.printed(),
        ["2"],
        "the first error ends the reduction"
    );
    assert_raises(&[("reduce({a: 1}, 0, fn acc, k, v -> error('boom'))", "boom")]);
    // A single element with no initial value is never combined.
    assert_values(&[("reduce([1], fn acc, x -> error('boom'))", "1")]);
}

#[test]
fn reduce_checks_its_arguments() {
    assert_type_errors(
        "reduce",
        &[
            ("reduce(5, plus)", "Structured", "argument 1", "Int"),
            ("reduce(5, 0, plus)", "Structured", "argument 1", "Int"),
            // With two arguments, the second is the function, never an initial value.
            ("reduce([1], 1)", "Function", "argument 2", "Int"),
            // With three, the function is the third.
            ("reduce([1], 0, 1)", "Function", "argument 3", "Int"),
            ("reduce([1], plus, 0)", "Function", "argument 3", "Int"),
        ],
    );
    assert_arity("reduce", "between 2 and 3", &[0, 1, 4]);
}

// --- The former iteration words are ordinary names ---

#[test]
fn iteration_words_are_ordinary_names() {
    assert_values(&[
        ("(fn map -> map + 1)(1)", "2"),
        ("(fn filter, reduce -> [filter, reduce])(1, 2)", "[1, 2]"),
        (
            "(fn foreach, init, with -> [foreach, init, with])(1, 2, 3)",
            "[1, 2, 3]",
        ),
        (
            r"
            def init = 4
            def with = 5
            [init, with]
            ",
            "[4, 5]",
        ),
        (
            r"
            def foreach = 7
            foreach
            ",
            "7",
        ),
        (
            r"
            def map = 1
            def filter = 2
            def reduce = 3
            [map, filter, reduce]
            ",
            "[1, 2, 3]",
        ),
        (
            "{map: 1, filter: 2, reduce: 3, foreach: 4, init: 5, with: 6}.with",
            "6",
        ),
        ("{map: 1, init: 5}.map", "1"),
        ("{init: 5}.init", "5"),
    ]);
}

#[test]
fn a_binding_named_like_a_global_shadows_it() {
    assert_values(&[
        (
            r"
            def map = fn xs, f -> 'mine'
            map([1], id)
            ",
            "'mine'",
        ),
        ("(fn reduce -> reduce)(10)", "10"),
    ]);
}

// --- Evaluating and applying the calls ---

#[test]
fn collection_function_arguments_are_evaluated_in_the_order_written() {
    assert_raises(&[
        ("map(1 / 0, 1 % 0)", "Division by zero"),
        ("filter(1 / 0, 1 % 0)", "Division by zero"),
        ("each(1 / 0, 1 % 0)", "Division by zero"),
        ("reduce(1 / 0, null + true, 1 % 0)", "Division by zero"),
        (
            "reduce([1], null + true, 1 % 0)",
            "Cannot add incompatible types: Null + Bool",
        ),
    ]);
}

#[test]
fn the_operation_may_be_any_function() {
    assert_values(&[
        ("map([1, 2], $($ + 1))", "[2, 3]"),
        ("map([1, 2], to_string)", r#"["1", "2"]"#),
        ("reduce([1, 2, 3], plus)", "6"),
        ("filter([1, null, 2], is_int)", "[1, 2]"),
        (
            r"
            def k = 3
            map([1, 2], fn x -> x * k)
            ",
            "[3, 6]",
        ),
    ]);
}

#[test]
fn collection_functions_compose() {
    assert_values(&[
        (
            "map(filter([1, 2, 3, 4], fn x -> x > 2), fn x -> x * 10)",
            "[30, 40]",
        ),
        ("reduce(map([1, 2, 3], fn x -> x * x), plus)", "14"),
        (
            "map([[1, 2], [3]], fn row -> reduce(row, 0, plus))",
            "[3, 3]",
        ),
        (
            "[1, 2, 3, 4] @ filter(fn x -> x > 2) @ map(fn x -> x * 10)",
            "[30, 40]",
        ),
    ]);
}

#[test]
fn each_calls_its_function_and_returns_the_structure() {
    assert_values(&[
        ("each([1, 2], fn x -> x)", "[1, 2]"),
        ("each([], fn x -> x)", "[]"),
        ("each({a: 1}, fn k, v -> null)", "{a: 1}"),
        ("[3] @ each(fn x -> x)", "[3]"),
    ]);
    assert_eq!(printed("each([1, 2, 3], print)"), ["1", "2", "3"]);
    assert_eq!(
        printed("each({a: 1, b: 2}, fn k, v -> print($'${k}=${v}'))").len(),
        2,
        "once per entry"
    );
    assert_eq!(
        printed("each({a: 1}, fn k, v -> print($'${k}=${v}'))"),
        ["a=1"]
    );
}

#[test]
fn each_raises_what_its_function_raises() {
    let script = script("each([1, 2], fn x -> { print(x); error('boom') })");
    assert_eq!(script.raises(), "boom");
    assert_eq!(script.printed(), ["1"], "the first error ends the loop");
    assert_raises(&[("each({a: 1}, fn k, v -> error('boom'))", "boom")]);
}

#[test]
fn each_checks_its_arguments() {
    assert_type_errors(
        "each",
        &[
            ("each(5, id)", "Structured", "argument 1", "Int"),
            ("each([1], 1)", "Function", "argument 2", "Int"),
        ],
    );
    assert_arity("each", "2", &[0, 1, 3]);
}

// --- shared inputs ---
//
// A global may edit a uniquely-owned input in place, so one that is shared
// must come out unchanged.

#[test]
fn array_globals_leave_a_shared_input_unchanged() {
    let source = r"
        def a = [3, 1, [2], 4]
        def results = [
            reverse(a),
            take_while(a, fn x -> is_int(x)),
            drop_while(a, fn x -> is_int(x)),
            drop(a, 1),
            tail(a, 2),
            flatten(a),
            flat_map(a, fn x -> [x, x]),
            find(a, is_array),
            map(a, fn x -> [x]),
            filter(a, is_int),
            reject(a, is_int),
            scan([a[0], a[1]], plus),
            each(a, id),
            sorted(filter(a, is_int)),
            reduce(a, 0, fn acc, x -> acc),
        ]
        [a, len(results)]
    ";
    assert_values(&[(source, "[[3, 1, [2], 4], 15]")]);
}

#[test]
fn array_globals_return_the_right_results_over_a_shared_input() {
    let source = r"
        def a = [3, 1, [2], 4]
        [
            reverse(a),
            take_while(a, is_int),
            drop_while(a, is_int),
            drop(a, 1),
            tail(a, 2),
            flatten(a),
            flat_map(a, fn x -> [x, x]),
            find(a, is_array),
            a,
        ]
    ";
    assert_values(&[(
        source,
        r"[
            [4, [2], 1, 3],
            [3, 1],
            [[2], 4],
            [1, [2], 4],
            [[2], 4],
            [3, 1, 2, 4],
            [3, 3, 1, 1, [2], [2], 4, 4],
            [2],
            [3, 1, [2], 4],
        ]",
    )]);
}

#[test]
fn map_globals_leave_a_shared_input_unchanged() {
    let source = r"
        def m = {a: 1, b: 2}
        [
            map_values(m, fn v -> v * 10),
            map_keys(m, to_upper),
            dissoc(m, 'a'),
            filter(m, fn k, v -> v > 1),
            reject(m, fn k, v -> v > 1),
            map(m, fn k, v -> {[k]: v}),
            each(m, fn k, v -> v),
            m,
        ]
    ";
    assert_values(&[(
        source,
        r"[
            {a: 10, b: 20},
            {A: 1, B: 2},
            {b: 2},
            {b: 2},
            {a: 1},
            {a: 1, b: 2},
            {a: 1, b: 2},
            {a: 1, b: 2},
        ]",
    )]);
}

#[test]
fn path_globals_leave_a_shared_input_unchanged() {
    let source = r"
        def s = {a: [{b: 1}, {b: 2}], c: {d: {e: 3}}}
        [
            dig_set(s, 9, 'a', 1, 'b'),
            dig_set(s, 9, 'c', 'd', 'f'),
            dig_update(s, fn x -> x + 1, 'a', 0, 'b'),
            dig_update(s, fn d -> d + {f: 4}, 'c', 'd'),
            deep_merge(s, {a: [], c: {d: {e: 4}}}),
            s,
        ]
    ";
    assert_values(&[(
        source,
        r"[
            {a: [{b: 1}, {b: 9}], c: {d: {e: 3}}},
            {a: [{b: 1}, {b: 2}], c: {d: {e: 3, f: 9}}},
            {a: [{b: 2}, {b: 2}], c: {d: {e: 3}}},
            {a: [{b: 1}, {b: 2}], c: {d: {e: 3, f: 4}}},
            {a: [], c: {d: {e: 4}}},
            {a: [{b: 1}, {b: 2}], c: {d: {e: 3}}},
        ]",
    )]);
}

#[test]
fn a_shared_right_side_of_deep_merge_is_unchanged() {
    let source = r"
        def right = {a: {x: 2}}
        [deep_merge({a: {y: 1}}, right), right]
    ";
    assert_values(&[(source, "[{a: {x: 2, y: 1}}, {a: {x: 2}}]")]);
}

#[test]
fn deep_merge_of_a_map_with_itself_is_that_map() {
    let source = r"
        def m = {a: {x: 1}, b: [2]}
        [deep_merge(m, m), m]
    ";
    assert_values(&[(source, "[{a: {x: 1}, b: [2]}, {a: {x: 1}, b: [2]}]")]);
}

#[test]
fn path_edits_may_take_the_structure_they_edit_as_their_payload() {
    let set = r"
        def s = {a: 1}
        dig_set(s, s, 'a')
    ";
    // The function sees the original whole, never one with a hole where its
    // argument was taken out.
    let update = r"
        def s = {a: [1, 2]}
        dig_update(s, fn x -> [x, s], 'a')
    ";
    assert_values(&[(set, "{a: {a: 1}}"), (update, "{a: [[1, 2], {a: [1, 2]}]}")]);
}

#[test]
fn a_failed_path_edit_leaves_a_shared_input_unchanged() {
    // Each fails partway, once the top level has been copied and a child taken
    // out of the copy.
    let source = r"
        def s = {a: [1, 2], b: {c: 1}}
        def failed = [
            try_call(fn -> dig_update(s, fn x -> error('boom'), 'a', 0)).ok,
            try_call(fn -> dig_set(s, 9, 'b', 'missing', 'x')).ok,
        ]
        [failed, s]
    ";
    assert_values(&[(source, "[[false, false], {a: [1, 2], b: {c: 1}}]")]);
}

// --- Editing in place ---
//
// `unshared` sees whether the value `dig_update` hands its function is shared,
// as it is if any level above it was copied while the original still held it.
// A level copied and the original then dropped is invisible to a script; the
// runtime's white-box tests cover that.

/// `source` run with `unshared` and a host Int `n`, which makes each structure
/// holding it built at run time rather than shared with the constant pool.
fn run_probing_sharing(source: &str) -> Value {
    Script::new(source)
        .captures(&[("n", Value::from(1)), ("unshared", unshared())])
        .run()
}

#[test]
fn dig_update_hands_its_function_an_unshared_value_at_every_depth() {
    let source = r"
        def make = fn -> {a: [{b: [n]}]}
        [
            dig_update(make(), unshared, 'a'),
            dig_update(make(), unshared, 'a', 0),
            dig_update(make(), unshared, 'a', 0, 'b'),
        ]
    ";
    assert_eq!(
        run_probing_sharing(source),
        run("[{a: true}, {a: [true]}, {a: [{b: true}]}]"),
        "a uniquely owned structure should be edited in place at every level"
    );
}

#[test]
fn dig_update_hands_its_function_a_shared_value_when_the_input_is_shared() {
    // The probe can see sharing: the same edits over a structure still held
    // elsewhere must copy, so the value at the end of the path is shared.
    let source = r"
        def s = {a: [{b: [n]}]}
        [
            dig_update(s, unshared, 'a'),
            dig_update(s, unshared, 'a', 0),
            dig_update(s, unshared, 'a', 0, 'b'),
            s,
        ]
    ";
    assert_eq!(
        run_probing_sharing(source),
        run("[{a: false}, {a: [false]}, {a: [{b: false}]}, {a: [{b: [1]}]}]"),
        "a structure held elsewhere should be copied, not edited"
    );
}

#[test]
fn edits_copy_a_level_shared_below_a_unique_top() {
    // Each top level is built for the edit and held by nothing else, but the
    // level below it is also held by `inner`.
    let update = r"
        def inner = [n]
        [dig_update({a: inner}, unshared, 'a'), inner]
    ";
    let set = r"
        def inner = [n]
        [dig_set({a: inner}, 9, 'a', 0), inner]
    ";
    let merge = r"
        def inner = {x: n}
        [deep_merge({a: inner}, {a: {y: 2}}), inner]
    ";
    for (source, expected) in [
        (update, "[{a: false}, [1]]"),
        (set, "[{a: [9]}, [1]]"),
        (merge, "[{a: {x: 1, y: 2}}, {x: 1}]"),
    ] {
        assert_eq!(
            run_probing_sharing(source),
            run(expected),
            "the shared level should be copied, leaving `inner` unchanged: {source}"
        );
    }
}

// --- Deep nesting ---

/// Path edits and `deep_merge` through a structure nested `levels` deep, on a
/// small stack (see [`deep`](crate::deep)). How deep comes from the host as
/// `levels`, so that the structure is built when the script runs rather than by
/// the compiler.
#[test]
fn edits_reach_through_deep_nesting() {
    let source = r"
        defn nest(v, n) -> if n == 0: v else: nest({k: v}, n - 1)
        def path = map(range(levels), fn _ -> 'k')
        [
            call(dig_set, [nest(1, levels), 2] + path) == nest(2, levels),
            call(dig_update, [nest(1, levels), fn x -> x + 1] + path) == nest(2, levels),
            deep_merge(nest({a: 1}, levels), nest({b: 2}, levels)) == nest({a: 1, b: 2}, levels),
        ]
    ";
    let tail = crate::deep::on_a_small_stack(|| {
        Script::new(source)
            .capture("levels", Value::from(10_000))
            .run()
    });
    assert_eq!(tail, Value::from(vec![Value::from(true); 3]), "{source}");
}

// --- Maps past the compact form ---

#[test]
fn large_maps_work_in_every_map_global() {
    let ordered_values = "map(range(40), fn n -> n * 2)";
    for (expression, expected) in [
        ("len(m)", "40"),
        ("sorted(keys(m))", "range(40)"),
        ("sorted(values(m))", ordered_values),
        (
            r"
            def ks = keys(m)
            def vs = values(m)
            all(range(40), fn i -> m[ks[i]] == vs[i])
            ",
            "true",
        ),
        (
            "[has(m, 0), has(m, 39), has(m, 40), has(m, '1')]",
            "[true, true, false, false]",
        ),
        ("index(39)(m)", "78"),
        ("len(dissoc(m, 0, 39))", "38"),
        (
            "[has(dissoc(m, 5), 5), has(dissoc(m, 5), 6)]",
            "[false, true]",
        ),
        ("dissoc(m, 100)", LARGE_MAP),
        (
            "sorted(values(map_values(m, fn v -> v + 1)))",
            "map(range(40), fn n -> n * 2 + 1)",
        ),
        (
            "sorted(keys(map_keys(m, fn k -> k + 100)))",
            "range(100, 140)",
        ),
        ("map_values(m, id)", LARGE_MAP),
        ("map_keys(m, id)", LARGE_MAP),
        ("len(to_entries(m))", "40"),
        ("from_entries(to_entries(m))", LARGE_MAP),
        ("filter(m, fn k, v -> k < 20) @ len()", "20"),
        ("reject(m, fn k, v -> k < 20) @ len()", "20"),
        ("map(m, fn k, v -> {[k]: v})", LARGE_MAP),
        ("reduce(m, 0, fn acc, k, v -> acc + v)", "1560"),
        ("each(m, fn k, v -> v)", LARGE_MAP),
        ("len(group_by(range(40), id))", "40"),
        ("count_by(range(40), id) @ values() @ sum()", "40"),
        ("dig_set(m, 0, 39) @ len()", "40"),
        ("dig_set(m, 0, 39)[39]", "0"),
        ("dig_set(m, 'x', 40) @ len()", "41"),
        ("dig_update(m, fn v -> v + 1, 39)[39]", "79"),
        (
            "dig_update({a: m}, fn v -> [v], 'a', 40)['a'][40]",
            "[null]",
        ),
        ("deep_merge(m, {[0]: 'x'})[0]", "'x'"),
        ("deep_merge({a: m}, {a: {[40]: 1}})['a'] @ len()", "41"),
        ("deep_merge({a: m}, {a: m})", "{a: m}"),
    ] {
        assert_values(&[(&over_large_map(expression), &over_large_map(expected))]);
    }
}

#[test]
fn edits_grow_a_map_past_the_compact_form() {
    // 16 is `COMPACT_LIMIT` in value_map.rs, the most entries the compact form
    // holds; a 17th moves a Map to the larger form.
    let source = r"
        def full = map_into(range(16), fn n -> {[n]: n})
        def grown = map_into(range(17), fn n -> {[n]: n})
        [
            dig_set(full, 16, 16) == grown,
            dig_update(full, fn x -> 16, 16) == grown,
            dig_set({a: full}, 16, 'a', 16) == {a: grown},
            deep_merge({a: full}, {a: {[16]: 16}}) == {a: grown},
        ]
    ";
    assert_values(&[(source, "[true, true, true, true]")]);
}
