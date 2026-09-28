//! Map destructuring, end to end: `def {key, key: name, [expr]: name} as whole = value`.
//!
//! The value must be a Map, and must hold every key the pattern names; other keys
//! are allowed. A key is a name (shorthand for the String key and a binding of
//! it), a name mapped to a part, or a computed expression; `as` binds the whole
//! Map. Parts bind in source order and may nest: each entry's key is evaluated,
//! looked up, and its part destructured before the next entry's key, so a
//! computed key may read the bindings before it. `as` binds last. Cases were
//! checked against the C++ implementation; error messages are the bytecode
//! compiler's own.
//!
//! The harness runs every behavioral case under every optimization permutation;
//! code-shape cases pin exactly the options they are about.

mod common;

use common::{Emitted, Script, UNOPTIMIZED, compile_errors, raises, run};
use frost_compile::OptimizationOptions;
use frost_runtime::{Bytecode, MapKey, Value};

const FOLD: OptimizationOptions = OptimizationOptions {
    constant_fold: true,
    ..UNOPTIMIZED
};

/// Assert each `source` runs to the value of the Frost expression `expected`.
fn assert_values(cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(run(source), run(expected), "{source:?} is {expected}");
    }
}

/// Assert each `source` raises an error mentioning `message`.
fn assert_raises(cases: &[(&str, &str)]) {
    for (source, message) in cases {
        let raised = raises(source);
        assert!(
            raised.contains(message),
            "{source:?} raises about {message:?}, but raised: {raised}"
        );
    }
}

/// Assert each `source` fails to compile, with a diagnostic mentioning `message`.
fn assert_compile_errors(cases: &[(&str, &str)]) {
    for (source, message) in cases {
        let rendered = compile_errors(source).render_plain();
        assert!(
            rendered.contains(message),
            "{source:?} is rejected for {message:?}, but the diagnostic is:\n{rendered}"
        );
    }
}

/// `source` after a prelude defining `note(x)`, which appends `x` to the Array
/// in the cell `log` and returns it, so a script can observe the order in which
/// its expressions run.
fn noting(source: &str) -> String {
    format!(
        "def log = mutable_cell([]); defn note(x) -> {{ log.exchange(log.get() + [x]); x }}; {source}"
    )
}

/// Assert `script` completes exporting exactly `expected`, each a name and the
/// Frost expression for its value.
fn assert_exports(script: Script, expected: &[(&str, &str)]) {
    let finished = script.finish();
    let expected = expected
        .iter()
        .map(|(name, value)| (name.to_string(), run(value)))
        .collect();
    assert_eq!(finished.exports, expected, "{finished:?}");
}

fn definitions(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
        .filter(|op| matches!(op, Bytecode::DefLocal(_)))
        .count()
}

// --- Binding the entries ---

#[test]
fn a_shorthand_key_binds_its_own_name() {
    assert_values(&[
        ("def {a, b} = {a: 1, b: 2}; [b, a]", "[2, 1]"),
        ("def {a} = {a: [1, 2]}; a", "[1, 2]"),
    ]);
}

#[test]
fn a_key_may_bind_another_name() {
    assert_values(&[
        ("def {a: x, b: y} = {a: 1, b: 2}; [y, x]", "[2, 1]"),
        ("def {a: x, b} = {a: 1, b: 2}; [x, b]", "[1, 2]"),
    ]);
}

#[test]
fn a_key_holding_null_is_present() {
    assert_values(&[
        ("def {a} = {a: null}; a", "null"),
        ("def {a: [x]} = {a: [null]}; x", "null"),
    ]);
}

#[test]
fn a_trailing_comma_is_allowed() {
    assert_values(&[
        ("def {a, b,} = {a: 1, b: 2}; [a, b]", "[1, 2]"),
        ("def {a: x,} = {a: 1}; x", "1"),
        ("def {[1]: x,} as m = {[1]: 2}; [x, m]", "[2, {[1]: 2}]"),
        ("def {a: {b,},} = {a: {b: 1}}; b", "1"),
    ]);
}

#[test]
fn a_pattern_may_span_lines() {
    assert_values(&[
        (
            r"def {
                x, y} = {x: 1, y: 2}
            [x, y]",
            "[1, 2]",
        ),
        (
            r"def {x,
                y} = {x: 1, y: 2}
            [x, y]",
            "[1, 2]",
        ),
        (
            r"def {x, y
            } = {x: 1, y: 2}
            [x, y]",
            "[1, 2]",
        ),
        (
            r"def {
                x,
                y,
            } = {x: 1, y: 2}
            [x, y]",
            "[1, 2]",
        ),
        (
            r"def {
                ['k']: a
            } = {k: 1}
            a",
            "1",
        ),
        (
            r"def {x:
                [a]} = {x: [1]}
            a",
            "1",
        ),
        (
            r"def {[
            'k'
            ]: a} = {k: 1}
            a",
            "1",
        ),
        (
            r"def {x
            } as whole = {x: 1}
            [x, whole]",
            "[1, {x: 1}]",
        ),
        (
            r"def {
                a: [b,
                    c],
                d: {e},
            } as m = {a: [1, 2], d: {e: 3}}
            [b, c, e, m.d]",
            "[1, 2, 3, {e: 3}]",
        ),
    ]);
}

#[test]
fn a_computed_key_may_be_any_valid_key() {
    assert_values(&[
        (
            r#"def {[1]: one, [true]: t, [x'00']: b, [1.5]: f} = {[1]: "i", [true]: "t", [x'00']: "b", [1.5]: "f"}; [one, t, b, f]"#,
            r#"["i", "t", "b", "f"]"#,
        ),
        (r#"def {["with space"]: v} = {["with space"]: 1}; v"#, "1"),
        ("def k = \"b\"; def {[k]: v} = {b: 5}; v", "5"),
        ("def {[1 + 1]: v} = {[2]: \"two\"}; v", r#""two""#),
    ]);
}

#[test]
fn keys_of_every_type_match_only_their_own_type() {
    assert_values(&[
        (
            r#"def {[-1]: n, [false]: f, [""]: e, [0.5]: h} = {[-1]: 1, [false]: 2, [""]: 3, [0.5]: 4}; [n, f, e, h]"#,
            "[1, 2, 3, 4]",
        ),
        (
            r#"def {[x'61']: b, a} = {[x'61']: "bytes", a: "string"}; [b, a]"#,
            r#"["bytes", "string"]"#,
        ),
    ]);
    assert_raises(&[
        ("def {[0]: x} = {[false]: 1}; x", "no value at key"),
        (r#"def {["1"]: x} = {[1]: 1}; x"#, "no value at key"),
        ("def {[1]: x} = {[\"1\"]: 1}; x", "no value at key"),
        ("def {[x'61']: x} = {a: 1}; x", "no value at key"),
        ("def {a} = {[x'61']: 1}; a", "no value at key 'a'"),
    ]);
}

#[test]
fn a_computed_key_may_be_any_expression() {
    assert_values(&[
        (r#"def {[to_string(1)]: v} = {["1"]: 5}; v"#, "5"),
        (r#"def {[(fn -> "a")()]: v} = {a: 1}; v"#, "1"),
        (r#"def ks = ["a", "b"]; def {[ks[1]]: v} = {b: 2}; v"#, "2"),
        (r#"def {[if false: "a" else: "b"]: v} = {b: 3}; v"#, "3"),
        (r#"def {[null or "a"]: v} = {a: 4}; v"#, "4"),
        (r#"def {[do { def k = "a"; k }]: v} = {a: 5}; v"#, "5"),
        (r#"def {[do { def [k] = ["a"]; k }]: v} = {a: 6}; v"#, "6"),
        (r#"def {[{k: "a"}.k]: v} = {a: 7}; v"#, "7"),
    ]);
}

#[test]
fn a_computed_key_may_be_a_format_string() {
    assert_values(&[
        (r#"def n = "b"; def {[$'k${n}']: v} = {kb: 1}; v"#, "1"),
        (
            r#"def n = "b"; def {[$'k${$'${n}!'}']: v} = {["kb!"]: 2}; v"#,
            "2",
        ),
        ("def {[$'${1 + 1}']: v} = {[\"2\"]: 3}; v", "3"),
        // The key reads a binding made earlier in the same pattern.
        (r#"def {n, [$'k${n}']: v} = {n: 1, k1: 4}; v"#, "4"),
    ]);
}

#[test]
fn a_computed_key_may_read_a_capture() {
    let tail = Script::new("def {[key]: v, [key + \"!\"]: w} = {x: 1, [\"x!\"]: 2}; [v, w]")
        .capture("key", Value::from("x"))
        .run();
    assert_eq!(tail, run("[1, 2]"));
}

#[test]
fn keys_may_be_left_out_or_taken_twice() {
    assert_values(&[
        ("def {a} = {a: 1, b: 2, c: 3}; a", "1"),
        ("def {a, a: b} = {a: 1}; [a, b]", "[1, 1]"),
        ("def {} = {a: 1}; 0", "0"),
        ("def {} = {}; 0", "0"),
    ]);
}

#[test]
fn a_discard_still_requires_its_key() {
    assert_values(&[("def {a: _, b} = {a: 1, b: 2}; b", "2")]);
    assert_raises(&[("def {a: _} = {}; 0", "no value at key 'a'")]);
}

#[test]
fn a_shorthand_underscore_requires_the_key_but_binds_nothing() {
    // `{_}` is the String key "_", discarded.
    assert_values(&[
        (r#"def {_} = {["_"]: 1}; 0"#, "0"),
        (r#"def {_, _} = {["_"]: 1}; 0"#, "0"),
        (r#"def {_: x} = {["_"]: 2}; x"#, "2"),
    ]);
    assert_raises(&[("def {_} = {a: 1}; 0", "no value at key '_'")]);
}

#[test]
fn discards_never_collide() {
    assert_values(&[
        ("def {a: _, b: _} as _ = {a: 1, b: 2}; 0", "0"),
        (
            "def {a: [_, _], b: {c: _}} = {a: [1, 2], b: {c: 3}}; 0",
            "0",
        ),
        ("def {a: _} as _ = {a: 1}; def {a: _} as _ = {a: 2}; 0", "0"),
    ]);
}

#[test]
fn as_may_discard_the_whole_map() {
    assert_values(&[
        ("def {a} as _ = {a: 1}; a", "1"),
        ("def {} as _ = {}; 0", "0"),
    ]);
    assert_raises(&[("def {} as _ = [1]; 0", "expected a Map")]);
}

#[test]
fn as_binds_the_whole_map() {
    assert_values(&[
        ("def {a} as m = {a: 1, b: 2}; [a, m]", "[1, {a: 1, b: 2}]"),
        ("def {} as m = {a: 1}; m", "{a: 1}"),
        ("def {a: _} as m = {a: 1}; m", "{a: 1}"),
    ]);
}

#[test]
fn as_binds_at_every_level() {
    assert_values(&[
        (
            "def {a: {b} as inner} as outer = {a: {b: 1}, c: 2}; [b, inner, outer]",
            "[1, {b: 1}, {a: {b: 1}, c: 2}]",
        ),
        (
            "def {a: [{b} as x, {c} as y]} = {a: [{b: 1}, {c: 2}]}; [x, y]",
            "[{b: 1}, {c: 2}]",
        ),
    ]);
}

// --- Scope ---

#[test]
fn a_computed_key_may_read_an_earlier_binding() {
    // `[a]` is the value just bound to `a`, used as a key.
    assert_values(&[(r#"def {a, [a]: b} = {a: "c", c: 3}; b"#, "3")]);
}

#[test]
fn a_computed_key_may_read_any_part_bound_before_it() {
    assert_values(&[
        // An explicit key's part.
        (r#"def {a: x, [x]: y} = {a: "b", b: 1}; y"#, "1"),
        // A nested Array part.
        (
            r#"def {a: [k, ...r], [k]: v} = {a: ["z", 0], z: 2}; [v, r]"#,
            "[2, [0]]",
        ),
        // A nested Map part.
        (r#"def {o: {k}, [k]: v} = {o: {k: "z"}, z: 3}; v"#, "3"),
        // From inside a nested Map, a part of the pattern around it.
        (
            r#"def {k, inner: {[k]: v}} = {k: "x", inner: {x: 4}}; v"#,
            "4",
        ),
        // An earlier element of an Array pattern around the Map.
        (r#"def [k, {[k]: v}] = ["a", {a: 5}]; v"#, "5"),
        // A chain, each key reading the part before it.
        (
            r#"def {start, [start]: next, [next]: last} = {start: "a", a: "b", b: 6}; last"#,
            "6",
        ),
    ]);
}

#[test]
fn a_computed_key_may_not_read_its_own_part_or_a_later_one() {
    assert_compile_errors(&[
        ("def {[v]: v} = {}", "`v` is not defined"),
        ("def {[b]: a, b} = {}", "`b` is not defined"),
        ("def {[k]: [k]} = {}", "`k` is not defined"),
        // `as` binds the whole Map only after every entry.
        ("def {[m.k]: x} as m = {}", "`m` is not defined"),
    ]);
}

#[test]
fn a_computed_key_reads_the_enclosing_name_its_pattern_later_shadows() {
    assert_values(&[
        (r#"def a = "k"; do { def {[a]: a} = {k: 1}; a }"#, "1"),
        (
            r#"def b = "x"; do { def {[b]: a, b} = {x: 1, b: 2}; [a, b] }"#,
            "[1, 2]",
        ),
        (
            r#"def m = {k: "b"}; do { def {[m.k]: v} as m = {k: "a", a: 1, b: 2}; [v, m.k] }"#,
            r#"[2, "a"]"#,
        ),
    ]);
}

#[test]
fn the_value_is_evaluated_before_the_names_are_bound() {
    assert_values(&[(
        "def x = 9; do { def {x, y} = {x: x + 1, y: x}; [x, y] }",
        "[10, 9]",
    )]);
}

#[test]
fn a_pattern_may_shadow_a_capture_its_value_and_keys_read() {
    let tail = Script::new("def {[k]: v, k} = m; def {m} = m; [v, k, m]")
        .captures(&[
            ("k", Value::from("x")),
            (
                "m",
                Value::map([
                    ("x", Value::Int(1)),
                    ("k", Value::from("y")),
                    ("m", Value::Int(2)),
                ]),
            ),
        ])
        .run();
    assert_eq!(tail, run(r#"[1, "y", 2]"#));
}

#[test]
fn destructuring_works_in_any_scope() {
    assert_values(&[
        (
            "defn area(r) -> { def {w, h} = r; w * h }; area({w: 3, h: 4})",
            "12",
        ),
        ("do { def {a} as m = {a: 1}; [a, m] }", "[1, {a: 1}]"),
    ]);
}

#[test]
fn every_part_is_implicitly_exported() {
    let finished = Script::new("def {a, b: [c]} as m = {a: 1, b: [2]}; 0")
        .implicit_export()
        .finish();
    assert_eq!(
        finished.exports.keys().collect::<Vec<_>>(),
        vec!["a", "c", "m"],
        "{finished:?}"
    );
}

#[test]
fn every_kind_of_part_is_implicitly_exported_and_discards_are_not() {
    assert_exports(
        Script::new(
            r#"def {a, b: x, [1]: y, c: [z, ...r], d: {e} as n, f: _, _} as m = {a: 1, b: 2, [1]: 3, c: [4, 5], d: {e: 6}, f: 7, ["_"]: 8}; 0"#,
        )
        .implicit_export(),
        &[
            ("a", "1"),
            ("x", "2"),
            ("y", "3"),
            ("z", "4"),
            ("r", "[5]"),
            ("e", "6"),
            ("n", "{e: 6}"),
            (
                "m",
                r#"{a: 1, b: 2, [1]: 3, c: [4, 5], d: {e: 6}, f: 7, ["_"]: 8}"#,
            ),
        ],
    );
    assert_exports(
        Script::new("def {a} as _ = {a: 1}; 0").implicit_export(),
        &[("a", "1")],
    );
}

#[test]
fn an_exported_pattern_exports_every_named_part() {
    // No implicit export: only the `export def` parts are exported.
    assert_exports(
        Script::new(
            "export def {a, b: [c, ...d], [1]: e, f: _} as m = {a: 1, b: [2], [1]: 3, f: 4}; def {g} = {g: 5}; 0",
        ),
        &[
            ("a", "1"),
            ("c", "2"),
            ("d", "[]"),
            ("e", "3"),
            ("m", "{a: 1, b: [2], [1]: 3, f: 4}"),
        ],
    );
}

#[test]
fn a_name_bound_twice_in_a_pattern_is_a_compile_error() {
    for source in [
        "def {a, a} = {a: 1}",
        "def {a} as a = {a: 1}",
        "def {a, b: a} = {a: 1, b: 2}",
        "def {a, b: [a]} = {a: 1, b: [2]}",
    ] {
        let rendered = compile_errors(source).render_plain();
        assert!(
            rendered.contains("`a` is already bound"),
            "{source:?}:\n{rendered}"
        );
    }
}

#[test]
fn only_a_shorthand_key_binds_its_name() {
    // `a: [a]` names the key `a`, but binds only its part's `a`.
    assert_values(&[("def {a: [a]} = {a: [1]}; a", "1")]);
}

#[test]
fn a_name_bound_twice_across_nesting_is_a_compile_error() {
    assert_compile_errors(&[
        (
            "def {a: {b} as m} as m = {a: {b: 1}}",
            "`m` is already bound",
        ),
        (
            "def {a: [x], b: {x}} = {a: [1], b: {x: 2}}",
            "`x` is already bound",
        ),
        (
            "def {a: [...r], b: [r]} = {a: [], b: [1]}",
            "`r` is already bound",
        ),
        (
            "def {[1]: v, [2]: v} = {[1]: 1, [2]: 2}",
            "`v` is already bound",
        ),
    ]);
}

// --- Nesting ---

#[test]
fn map_and_array_patterns_nest_in_each_other() {
    assert_values(&[
        (
            "def {a: [x, y], b: {c}} = {a: [1, 2], b: {c: 3}}; [x, y, c]",
            "[1, 2, 3]",
        ),
        ("def [{a}, {b}] = [{a: 1}, {b: 2}]; [a, b]", "[1, 2]"),
        (
            "def {pts: [{x}, ...rest]} as all = {pts: [{x: 1}, {x: 2}]}; [x, rest, all.pts[1].x]",
            "[1, [{x: 2}], 2]",
        ),
        ("def {a: {b: {c}}} = {a: {b: {c: 7}}}; c", "7"),
    ]);
}

#[test]
fn patterns_nest_wide_and_deep() {
    assert_values(&[(
        r"def {
            a: [{b: [c, {d}]}, ...e],
            f: {g: [h, ...i], j: {k: [l]}} as fm,
            m,
        } = {
            a: [{b: [1, {d: 2}]}, 3, 4],
            f: {g: [5, 6, 7], j: {k: [8]}},
            m: 9,
        }
        [c, d, e, h, i, l, m, fm.g]",
        "[1, 2, [3, 4], 5, [6, 7], 8, 9, [5, 6, 7]]",
    )]);
}

#[test]
fn values_may_be_closures_and_natives() {
    assert_values(&[
        (
            "def {f, g: [h]} = {f: fn x -> x + 1, g: [plus]}; h(f(1), 2)",
            "4",
        ),
        (
            "def {inc} = do { def n = 10; {inc: fn x -> x + n} }; inc(1)",
            "11",
        ),
        // The Map a native returns destructures like any other.
        (
            "def {get, exchange} = mutable_cell(5); exchange(6); get()",
            "6",
        ),
    ]);
}

#[test]
fn destructuring_leaves_the_value_intact() {
    assert_values(&[(
        "def m = {a: [1, 2], b: 3}; def {a: [x, ...y], b} = m; def {a} as whole = m; [m, x, y, b, a, whole]",
        "[{a: [1, 2], b: 3}, 1, [2], 3, [1, 2], {a: [1, 2], b: 3}]",
    )]);
}

// --- Mismatches ---

#[test]
fn a_missing_key_raises() {
    assert_raises(&[
        ("def {a} = {b: 1}; a", "no value at key 'a'"),
        ("def {x, y} = {y: 1}; x", "no value at key 'x'"),
        ("def {a, b} = {}; a", "no value at key 'a'"),
        ("def {a: {b}} = {a: {c: 1}}; b", "no value at key 'b'"),
    ]);
}

#[test]
fn keys_do_not_cross_numeric_types() {
    assert_raises(&[
        ("def {[1]: x} = {[1.0]: 2}; x", "no value at key"),
        ("def {[1.0]: x} = {[1]: 2}; x", "no value at key"),
    ]);
}

#[test]
fn entries_are_looked_up_in_source_order() {
    // Both keys are missing; the first is reported.
    assert_raises(&[
        ("def {first, second} = {}; 0", "'first'"),
        ("def {second, first} = {}; 0", "'second'"),
    ]);
}

#[test]
fn a_non_map_raises() {
    for value in ["5", "null", r#""ab""#, "[1]", "x'00'", "plus"] {
        for pattern in ["{a}", "{}", "{} as m", "{[1]: x}"] {
            let message = raises(&format!("def {pattern} = {value}; 0"));
            assert!(message.contains("Map"), "{pattern} = {value}: {message}");
        }
    }
}

#[test]
fn an_invalid_computed_key_raises() {
    assert_raises(&[
        ("def {[null]: x} = {a: 1}; x", "not a valid Map key"),
        ("def {[[1]]: x} = {a: 1}; x", "not a valid Map key"),
        ("def {[{}]: x} = {a: 1}; x", "not a valid Map key"),
        ("def {[plus]: x} = {a: 1}; x", "not a valid Map key"),
        ("def {[fn -> 1]: x} = {a: 1}; x", "not a valid Map key"),
        ("def {a, [a]: x} = {a: null}; x", "not a valid Map key"),
    ]);
}

#[test]
fn a_missing_key_is_named_in_the_error() {
    assert_raises(&[
        ("def {[1]: x} = {}; x", "no value at key '1'"),
        ("def {[true]: x} = {}; x", "no value at key 'true'"),
        (
            r#"def {["two words"]: x} = {}; x"#,
            "no value at key 'two words'",
        ),
    ]);
}

#[test]
fn a_mistyped_key_suggests_the_intended_one() {
    assert_raises(&[
        (
            "def {nmae} = {name: 1}; nmae",
            "no value at key 'nmae'; did you mean 'name'?",
        ),
        (
            "def {a: {widht}} = {a: {width: 1}}; widht",
            "did you mean 'width'?",
        ),
    ]);
}

#[test]
fn the_value_is_checked_before_any_key_is_evaluated() {
    assert_raises(&[
        ("def {[1 / 0]: x} = 5; x", "expected a Map"),
        ("def {[null]: x} = [1]; x", "expected a Map"),
    ]);
    assert_values(&[(
        &noting(r#"def r = try_call(fn -> { def {[note("a")]: x} = 5; x }); [r.ok, log.get()]"#),
        "[false, []]",
    )]);
}

#[test]
fn each_entry_destructures_completely_before_the_next_key() {
    assert_raises(&[
        // A part's own shape is checked before the next key is evaluated.
        ("def {a: [x], [1 / 0]: y} = {a: 5}; x", "exactly 1 element"),
        (
            "def {a: {b}, [1 / 0]: y} = {a: {}}; b",
            "no value at key 'b'",
        ),
        ("def {a: {b}, [1 / 0]: y} = {a: 5}; b", "expected a Map"),
        // An invalid key raises before a later missing key is looked up.
        ("def {[null]: x, missing} = {}; x", "not a valid Map key"),
        (
            "def {missing, [null]: x} = {}; x",
            "no value at key 'missing'",
        ),
    ]);
}

#[test]
fn the_value_then_each_key_and_lookup_run_in_turn() {
    assert_values(&[
        // The value runs first, then each key once, in source order.
        (
            &noting(
                r#"def {[note("a")]: x, [note("b")]: y} = note({a: 1, b: 2}); [x, y, log.get()]"#,
            ),
            r#"[1, 2, [{a: 1, b: 2}, "a", "b"]]"#,
        ),
        // A key nested in a part runs before the next key of the pattern around it.
        (
            &noting(
                r#"def {[note("a")]: {[note("b")]: x}, [note("c")]: y} = {a: {b: 1}, c: 2}; [x, y, log.get()]"#,
            ),
            r#"[1, 2, ["a", "b", "c"]]"#,
        ),
        // The first lookup fails before the second key runs.
        (
            &noting(
                r#"def r = try_call(fn -> { def {[note("a")]: x, [note("b")]: y} = {b: 2}; 0 }); [r.ok, log.get()]"#,
            ),
            r#"[false, ["a"]]"#,
        ),
    ]);
}

#[test]
fn errors_come_from_the_value_first_then_each_key_in_turn() {
    assert_raises(&[
        ("def {a} = (1 / 0); a", "Division by zero"),
        ("def {[1 / 0]: x} = {a: 1}; x", "Division by zero"),
        // The first entry's lookup fails before the second key is evaluated.
        (
            "def {missing, [1 / 0]: x} = {a: 1}; x",
            "no value at key 'missing'",
        ),
    ]);
}

// --- What the compiler emits ---

#[test]
fn a_block_destructuring_a_constant_folds_whole() {
    let emitted = Script::new("do { def {a} as m = {a: 1}; 5 }").code(FOLD);
    assert_eq!(definitions(&emitted), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");
}

#[test]
fn a_computed_key_folds_even_when_the_value_is_runtime() {
    let emitted = Script::new("def {[1 + 1]: v} = m; v")
        .capture("m", Value::map([(MapKey::Int(2), Value::Int(9))]))
        .code(FOLD);
    assert_eq!(emitted.count(&Bytecode::Add), 0, "{emitted:?}");
    assert_eq!(emitted.key_constants(), [MapKey::Int(2)], "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::ExtractKey), 0, "{emitted:?}");
}

#[test]
fn a_nested_computed_key_folds_even_when_the_value_is_runtime() {
    let emitted = Script::new("def {a: [{[1 + 1]: v}]} = m; v")
        .capture(
            "m",
            Value::map([(
                "a",
                Value::from_iter([Value::map([(MapKey::Int(2), Value::Int(9))])]),
            )]),
        )
        .code(FOLD);
    assert_eq!(emitted.count(&Bytecode::Add), 0, "{emitted:?}");
    assert_eq!(
        emitted.key_constants(),
        [MapKey::from("a"), MapKey::Int(2)],
        "{emitted:?}"
    );
    assert_eq!(emitted.count(&Bytecode::ExtractKey), 0, "{emitted:?}");
}

#[test]
fn a_computed_key_that_fails_to_fold_is_left_for_runtime() {
    let script =
        Script::new("def {[1 / 0]: v} = m; v").capture("m", Value::map([("a", Value::Int(1))]));
    let emitted = script.code(FOLD);
    assert_eq!(emitted.count(&Bytecode::Divide), 1, "{emitted:?}");
    let raised = script.raises();
    assert!(raised.contains("Division by zero"), "{raised}");
}

#[test]
fn a_block_whose_computed_key_is_invalid_is_left_for_runtime() {
    let source = "do { def {[null]: x} = {a: 1}; 5 }";
    let emitted = Script::new(source).code(FOLD);
    assert_eq!(
        emitted.count(&Bytecode::ExtractKey),
        1,
        "the lookup is kept: {emitted:?}"
    );
    assert_raises(&[(source, "not a valid Map key")]);
}

#[test]
fn a_block_whose_value_is_not_a_map_is_left_for_runtime() {
    let source = "do { def {} = [1]; 5 }";
    let emitted = Script::new(source).code(FOLD);
    assert_eq!(
        emitted.count(&Bytecode::ProduceError),
        1,
        "the shape check is kept: {emitted:?}"
    );
    assert_raises(&[(source, "expected a Map")]);
}

#[test]
fn a_block_whose_destructuring_fails_is_left_for_runtime() {
    let source = "do { def {a} = {b: 1}; 5 }";
    let emitted = Script::new(source).code(FOLD);
    assert_eq!(
        emitted.count(&Bytecode::ExtractConstKey(0)),
        1,
        "the lookup is kept: {emitted:?}"
    );
    assert_raises(&[(source, "no value at key 'a'")]);
}

#[test]
fn a_known_key_is_looked_up_as_a_constant() {
    // A literal key is known as written, without folding.
    for source in [
        "def {a} = m; a",
        "def {a: v} = m; v",
        r#"def {["a"]: v} = m; v"#,
    ] {
        let emitted = Script::new(source)
            .capture("m", Value::map([("a", Value::Int(1))]))
            .code(UNOPTIMIZED);
        assert_eq!(
            emitted.count(&Bytecode::ExtractConstKey(0)),
            1,
            "{source:?}: {emitted:?}"
        );
        assert_eq!(
            emitted.count(&Bytecode::ExtractKey),
            0,
            "{source:?}: {emitted:?}"
        );
        assert_eq!(emitted.key_constants(), [MapKey::from("a")], "{source:?}");
    }
}

#[test]
fn a_key_known_only_at_runtime_is_looked_up_dynamically() {
    let emitted = Script::new("def {[k]: v} = m; v")
        .capture("m", Value::map([("a", Value::Int(1))]))
        .capture("k", Value::from("a"))
        .code(UNOPTIMIZED);
    assert_eq!(emitted.count(&Bytecode::ExtractKey), 1, "{emitted:?}");
    assert!(emitted.key_constants().is_empty(), "{emitted:?}");
}
