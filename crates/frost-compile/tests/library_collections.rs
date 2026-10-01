//! The Collections globals, from Frost source.
//!
//! Most are natives. `index` and `dig` are generated from Frost (see the
//! runtime's `generated` module), so they index exactly as `structure[key]` does.
//!
//! The harness runs every case under every optimization permutation, so a call
//! over literals is checked both folded and at run time.

mod common;

use common::{Script, raises, run};

/// Assert each `source` runs to the value of the Frost expression `expected`.
fn assert_values(cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(run(source), run(expected), "{source:?} is {expected}");
    }
}

/// Assert each `source` raises exactly `message`.
fn assert_raises(cases: &[(&str, &str)]) {
    for (source, message) in cases {
        assert_eq!(raises(source), *message, "{source:?} raises {message:?}");
    }
}

/// Assert each `source` raises exactly what `equivalent` raises.
fn assert_raises_as(cases: &[(&str, &str)]) {
    for (source, equivalent) in cases {
        assert_eq!(
            raises(source),
            raises(equivalent),
            "{source:?} raises as {equivalent:?} does"
        );
    }
}

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

/// Assert `function` raises its arity error when called with each count in
/// `counts`, where `expects` is how the error states its arity, such as `2` or
/// `between 1 and 2`. The arguments are Nulls: arity is checked before types.
fn assert_arity(function: &str, expects: &str, counts: &[usize]) {
    for &argc in counts {
        let source = format!("{function}({})", vec!["null"; argc].join(", "));
        assert_eq!(
            raises(&source),
            format!("Function {function} expects {expects} arguments, but was called with {argc}"),
            "{source:?}"
        );
    }
}

/// The text of each `print` that `source` makes.
fn printed(source: &str) -> Vec<String> {
    Script::new(source).printed()
}

/// The set of types a Map key may have, as type errors name it.
const MAP_KEY: &str = "Bool or Int or Float or String or Bytes";

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
        all(['a', 'b', 3, true, 1.5], fn k -> includes(keys(m), k))
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
        &[("from_entries({})", "Array", "argument 1", "Map")],
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
            ("dissoc({}, null)", MAP_KEY, "argument 2", "Null"),
            ("dissoc({}, 'a', [1])", MAP_KEY, "argument 3", "Array"),
        ],
    );
    assert_arity("dissoc", "at least 1", &[0]);
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
    assert_raises(&[(
        "nulls(-1)",
        "Function nulls requires argument 1 to be at least 0, got -1",
    )]);
    assert_type_errors("nulls", &[("nulls('3')", "Int", "argument 1", "String")]);
    assert_arity("nulls", "1", &[0, 2]);
}

// --- has, includes ---

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

#[test]
fn includes_tests_for_an_equal_element() {
    assert_values(&[
        ("includes([1, 2, 3], 2)", "true"),
        ("includes([1, 2, 3], 4)", "false"),
        ("includes([], null)", "false"),
        ("includes([null], null)", "true"),
        // Equality is `==`: there is no cross-type numeric equality.
        ("includes([1], 1.0)", "false"),
        ("includes([[1]], [1])", "true"),
        ("includes([{a: 1}], {a: 1})", "true"),
    ]);
    assert_type_errors(
        "includes",
        &[("includes('abc', 'a')", "Array", "argument 1", "String")],
    );
    assert_arity("includes", "2", &[0, 1, 3]);
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
        ("map [{n: 1}, {n: 2}] with index('n')", "[1, 2]"),
    ]);
    assert_raises_as(&[("index('a')([1])", "[1]['a']"), ("index(0)(5)", "5[0]")]);
    assert_arity("index", "1", &[0, 2]);
    assert_raises(&[(
        "index('a')()",
        "Function index_fn expects 1 arguments, but was called with 0",
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
    assert_eq!(printed(&format!("any([1, 2, 3], {probe})")), ["1", "2"]);
    assert_eq!(printed(&format!("none([1, 2, 3], {probe})")), ["1", "2"]);
    assert_eq!(printed(&format!("all([2, 1, 2], {probe})")), ["2", "1"]);
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
                    "argument 2",
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
    assert_eq!(
        printed("find([1, 2, 3], fn x -> { print(x); x == 2 })"),
        ["1", "2"]
    );
    assert_type_errors(
        "find",
        &[
            ("find({}, id)", "Array", "argument 1", "Map"),
            ("find([1], 1)", "Function", "argument 2 (predicate)", "Int"),
        ],
    );
    assert_arity("find", "2", &[0, 1, 3]);
}

// --- slide, chunk ---

#[test]
fn slide_makes_overlapping_windows() {
    assert_values(&[
        ("slide([1, 2, 3, 4], 3)", "[[1, 2, 3], [2, 3, 4]]"),
        ("slide([1, 2], 1)", "[[1], [2]]"),
        ("slide([1, 2], 2)", "[[1, 2]]"),
        ("slide([1, 2], 3)", "[]"),
        ("slide([], 1)", "[]"),
    ]);
}

#[test]
fn chunk_makes_consecutive_chunks() {
    assert_values(&[
        ("chunk([1, 2, 3, 4, 5], 2)", "[[1, 2], [3, 4], [5]]"),
        ("chunk([1, 2, 3, 4], 2)", "[[1, 2], [3, 4]]"),
        ("chunk([1, 2], 5)", "[[1, 2]]"),
        ("chunk([], 2)", "[]"),
    ]);
}

#[test]
fn slide_and_chunk_check_their_arguments() {
    for function in ["slide", "chunk"] {
        assert_raises(&[
            (
                &format!("{function}([1], 0)"),
                &format!("Function {function} requires argument 2 to be at least 1, got 0"),
            ),
            (
                &format!("{function}([1], -1)"),
                &format!("Function {function} requires argument 2 to be at least 1, got -1"),
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
                    "argument 2",
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
    for function in ["take_while", "drop_while"] {
        assert_eq!(
            printed(&format!("{function}([1, 2, 3, 1], {probe})")),
            ["1", "2", "3"],
            "{function}"
        );
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
                    "argument 2",
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
    ]);
}

#[test]
fn flatten_checks_its_arguments() {
    assert_raises(&[(
        "flatten([1], -1)",
        "Function flatten requires argument 2 to be at least 0, got -1",
    )]);
    assert_type_errors(
        "flatten",
        &[
            ("flatten({})", "Array", "argument 1", "Map"),
            ("flatten([1], 1.0)", "Int", "argument 2", "Float"),
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
        "select([1, 2, 3, 4, 5], fn x -> x > 2) + reject([1, 2, 3, 4, 5], fn x -> x > 2)",
        "[3, 4, 5, 1, 2]",
    )]);
}

#[test]
fn reject_checks_its_arguments() {
    assert_type_errors(
        "reject",
        &[
            ("reject(5, id)", "Structured", "argument 1", "Int"),
            ("reject([1], 1)", "Function", "argument 2", "Int"),
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
        assert_raises(&[(
            &format!("{function}([1], fn x -> null)"),
            &format!(
                "Function {function} requires its function to return a valid Map key, got Null"
            ),
        )]);
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
                    "argument 2",
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
    assert_type_errors(
        "scan",
        &[
            ("scan({}, plus)", "Array", "argument 1", "Map"),
            ("scan([1], 1)", "Function", "argument 2", "Int"),
        ],
    );
    assert_arity("scan", "2", &[0, 1, 3]);
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
            ("partition([1], 1)", "Function", "argument 2", "Int"),
        ],
    );
    assert_arity("partition", "2", &[0, 1, 3]);
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
