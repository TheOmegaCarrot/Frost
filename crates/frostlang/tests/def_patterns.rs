//! `def` takes every pattern a `match` arm does, end to end: names, `_`, type
//! constraints, literal values, pins, Arrays, Maps, and alternatives `p1 | p2`
//! at any depth. Only a guard is not allowed.
//!
//! A `def` whose whole pattern binds no name is a compile error; a part of a
//! pattern may bind nothing, beside a part that does. A mismatch raises, with
//! a message saying what the failed check expected.
//!
//! The harness runs every behavioral case under every optimization permutation;
//! constant propagation of the bound parts is tested in `constant_propagation`.

use crate::script;

use frostlang::Value;
use script::{Script, compile_errors, raises, run};

/// Assert each `source` runs to the value of the Frost expression `expected`.
fn assert_values(cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(run(source), run(expected), "{source:?} is {expected}");
    }
}

/// Assert each `source` raises exactly `message`.
fn assert_raises_exactly(cases: &[(&str, &str)]) {
    for (source, message) in cases {
        assert_eq!(raises(source), *message, "{source:?}");
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

/// The prefix of every message a failed pattern check raises.
const NO_MATCH: &str = "Pattern does not match: ";

// --- Every form is accepted ---

#[test]
fn a_literal_value_is_checked() {
    assert_values(&[
        (
            r"
            def [a, 1] = [7, 1]
            a
            ",
            "7",
        ),
        (
            r"
            def [a, -1] = [7, -1]
            a
            ",
            "7",
        ),
        (
            r"
            def [a, 1.5] = [7, 1.5]
            a
            ",
            "7",
        ),
        (
            r"
            def [a, -2.5] = [7, -2.5]
            a
            ",
            "7",
        ),
        (
            r"
            def [a, 'ok'] = [7, 'ok']
            a
            ",
            "7",
        ),
        (
            r#"
            def [a, "ok"] = [7, "ok"]
            a
            "#,
            "7",
        ),
        (
            r#"
            def [a, R"(raw\n)"] = [7, R"(raw\n)"]
            a
            "#,
            "7",
        ),
        (
            r"
            def [a, x'0102'] = [7, x'0102']
            a
            ",
            "7",
        ),
        (
            r#"
            def [a, $"q${1}"] = [7, "q1"]
            a
            "#,
            "7",
        ),
        (
            r"
            def [a, null] = [7, null]
            a
            ",
            "7",
        ),
        (
            r"
            def [a, true] = [7, true]
            a
            ",
            "7",
        ),
        (
            r"
            def [a, false] = [7, false]
            a
            ",
            "7",
        ),
    ]);
}

#[test]
fn a_literal_compares_without_cross_type_equality() {
    assert_eq!(
        raises(
            r"
            def [a, 3] = [7, 3.0]
            a
            "
        ),
        format!("{NO_MATCH}expected `3`"),
        "an Int pattern does not match a Float"
    );
}

#[test]
fn a_pin_compares_against_an_expression_or_an_earlier_name() {
    assert_values(&[
        (
            r"
            def [a, (1 + 1)] = [7, 2]
            a
            ",
            "7",
        ),
        (
            r"
            def [a, (a)] = [7, 7]
            a
            ",
            "7",
        ),
        (
            r"
            def {k, other: (k)} = {k: 'v', other: 'v'}
            k
            ",
            "'v'",
        ),
        (
            r"
            def n = 5
            def [a, (n)] = [7, 5]
            a
            ",
            "7",
        ),
        (
            r"
            def [a, [(a)]] = [7, [7]]
            a
            ",
            "7",
        ),
    ]);
}

#[test]
fn a_type_constraint_applies_at_every_position() {
    assert_values(&[
        (
            r"
            def x is Int = 5
            x
            ",
            "5",
        ),
        (
            r"
            def [a, b is String] = [1, 'two']
            [a, b]
            ",
            "[1, 'two']",
        ),
        (
            r"
            def [a, [b is Float, ...c]] = [1, [2.5, 3]]
            [a, b, c]
            ",
            "[1, 2.5, [3]]",
        ),
        (
            r"
            def {host is String, port: p is Int} = {host: 'h', port: 80}
            [host, p]
            ",
            "['h', 80]",
        ),
        (
            r"
            def {a: {b is Bool}} = {a: {b: false}}
            b
            ",
            "false",
        ),
        (
            r"
            def [_ is Int, a] = [1, 2]
            a
            ",
            "2",
        ),
        (
            r"
            def [_ is Null, a] = [null, 2]
            a
            ",
            "2",
        ),
        (
            r"
            def {k: _ is String, v} = {k: 'x', v: 3}
            v
            ",
            "3",
        ),
    ]);
}

#[test]
fn every_type_constraint_accepts_a_matching_value() {
    assert_values(&[
        ("def x is Null = null\nx", "null"),
        ("def x is Int = 1\nx", "1"),
        ("def x is Float = 1.5\nx", "1.5"),
        ("def x is Bool = true\nx", "true"),
        ("def x is String = 's'\nx", "'s'"),
        ("def x is Bytes = x'00'\nx", "x'00'"),
        ("def x is Array = [1]\nx", "[1]"),
        ("def x is Map = {a: 1}\nx", "{a: 1}"),
        ("def x is Function = plus\nx == plus", "true"),
        ("def x is Primitive = 1\nx", "1"),
        ("def x is Numeric = 1.5\nx", "1.5"),
        ("def x is Structured = [1]\nx", "[1]"),
        ("def x is Flat = 's'\nx", "'s'"),
        ("def x is Nonnull = 1\nx", "1"),
    ]);
}

#[test]
fn a_nested_map_may_be_bound_whole() {
    assert_values(&[
        (
            r"
            def {outer: {inner} as whole} = {outer: {inner: 1, z: 2}}
            [inner, whole]
            ",
            "[1, {inner: 1, z: 2}]",
        ),
        (
            r"
            def [{a} as m, ...rest] = [{a: 1}, 2]
            [a, m, rest]
            ",
            "[1, {a: 1}, [2]]",
        ),
    ]);
}

#[test]
fn a_computed_key_takes_a_pattern() {
    assert_values(&[
        (
            r"
            def {[1 + 1]: v is Int, ['k']: 1 | 2, w} = {[2]: 9, k: 2, w: 3}
            [v, w]
            ",
            "[9, 3]",
        ),
        (
            r"
            def {a, [a]: [b] | b} = {a: 'c', c: 5}
            [a, b]
            ",
            "['c', 5]",
        ),
    ]);
}

#[test]
fn a_part_that_binds_nothing_may_sit_beside_one_that_does() {
    assert_values(&[
        (
            r"
            def [a, 1] = [5, 1]
            a
            ",
            "5",
        ),
        (
            r"
            def [_, b] = [5, 6]
            b
            ",
            "6",
        ),
        (
            r"
            def [[], a] = [[], 1]
            a
            ",
            "1",
        ),
        (
            r"
            def {a: _, b} = {a: 1, b: 2}
            b
            ",
            "2",
        ),
        (
            r"
            def {x: 1, y} = {x: 1, y: 2}
            y
            ",
            "2",
        ),
    ]);
}

// --- Alternatives ---

#[test]
fn an_alternative_takes_the_first_branch_that_matches() {
    assert_values(&[
        (
            r"
            def [a] | a = [1]
            a
            ",
            "1",
        ),
        (
            r"
            def [a] | a = 5
            a
            ",
            "5",
        ),
        (
            r"
            def [a, 1] | [1, a] = [2, 1]
            a
            ",
            "2",
        ),
        (
            r"
            def [a, 1] | [1, a] = [1, 2]
            a
            ",
            "2",
        ),
        (
            r"
            def [a, 1] | [1, a] = [1, 1]
            a
            ",
            "1",
        ),
        (
            r"
            def {a} | [a] = [5]
            a
            ",
            "5",
        ),
        (
            r"
            def {a} | {b: a} = {b: 1}
            a
            ",
            "1",
        ),
    ]);
}

#[test]
fn an_alternative_may_bind_the_same_names_in_different_positions() {
    assert_values(&[
        (
            r"
            def [a, b] | [b, a] = [1, 2]
            [a, b]
            ",
            "[1, 2]",
        ),
        (
            r"
            def {x: a, y: b} | [b, a] = [1, 2]
            [a, b]
            ",
            "[2, 1]",
        ),
        (
            r"
            def {x: a, y: b} | [b, a] = {x: 1, y: 2}
            [a, b]
            ",
            "[1, 2]",
        ),
        (
            r"
            def [_, a, ...rest] | {a, rest} = {a: 1, rest: 2}
            [a, rest]
            ",
            "[1, 2]",
        ),
    ]);
}

#[test]
fn an_alternative_may_nest() {
    assert_values(&[
        (
            r"
            def [a, [b] | b] = [1, 2]
            [a, b]
            ",
            "[1, 2]",
        ),
        (
            r"
            def [a, [b] | b] = [1, [2]]
            [a, b]
            ",
            "[1, 2]",
        ),
        (
            r"
            def [a, 1 | 2 | 3] = [7, 3]
            a
            ",
            "7",
        ),
        (
            r"
            def {k: [a] | {a}} = {k: {a: 4}}
            a
            ",
            "4",
        ),
        (
            r"
            def [[a, 0] | [0, a], [b] | b] = [[0, 5], 6]
            [a, b]
            ",
            "[5, 6]",
        ),
        (
            r"
            def [a, [b, 1 | 2] | b] = [1, [3, 2]]
            [a, b]
            ",
            "[1, 3]",
        ),
    ]);
}

#[test]
fn an_alternative_may_hold_type_constraints_and_pins() {
    assert_values(&[
        (
            r"
            def x is Int | x is String = 'a'
            x
            ",
            "'a'",
        ),
        (
            r"
            def x is Int | x is String = 3
            x
            ",
            "3",
        ),
        (
            r"
            def [a, (a) | 2] = [1, 2]
            a
            ",
            "1",
        ),
        (
            r"
            def [a, (a) | (a + 1)] = [1, 2]
            a
            ",
            "1",
        ),
    ]);
}

#[test]
fn a_name_bound_beyond_an_alternative_reads_the_matching_branch() {
    assert_values(&[(
        r"
        defn pick(v) -> {
            def [a, 1] | [1, a] = v
            a
        }
        [pick([8, 1]), pick([1, 9]), pick([1, 1])]
        ",
        "[8, 9, 1]",
    )]);
}

// --- Mismatches ---

#[test]
fn a_wrong_array_shape_raises_its_message() {
    assert_raises_exactly(&[
        (
            "def [a, b] = [1]",
            "Pattern does not match: expected an Array of exactly 2 elements",
        ),
        (
            "def [a] = [1, 2]",
            "Pattern does not match: expected an Array of exactly 1 element",
        ),
        (
            "def [a, ...r] = []",
            "Pattern does not match: expected an Array of at least 1 element",
        ),
        (
            "def [a, b, ...r] = [1]",
            "Pattern does not match: expected an Array of at least 2 elements",
        ),
        (
            "def [a, [b]] = [1, 5]",
            "Pattern does not match: expected an Array of exactly 1 element",
        ),
        (
            "def [a] = {a: 1}",
            "Pattern does not match: expected an Array of exactly 1 element",
        ),
    ]);
}

#[test]
fn a_non_map_raises_its_message() {
    assert_raises_exactly(&[
        ("def {a} = 5", "Pattern does not match: expected a Map"),
        ("def {a} = [1]", "Pattern does not match: expected a Map"),
        (
            "def {a: {b}} = {a: null}",
            "Pattern does not match: expected a Map",
        ),
        (
            "def {} as m = 'x'",
            "Pattern does not match: expected a Map",
        ),
    ]);
}

#[test]
fn each_type_constraint_raises_its_message() {
    assert_raises_exactly(&[
        ("def x is Null = 1", "Pattern does not match: expected Null"),
        (
            "def x is Int = 's'",
            "Pattern does not match: expected an Int",
        ),
        (
            "def x is Float = 1",
            "Pattern does not match: expected a Float",
        ),
        (
            "def x is Bool = 1",
            "Pattern does not match: expected a Bool",
        ),
        (
            "def x is String = 1",
            "Pattern does not match: expected a String",
        ),
        (
            "def x is Bytes = 1",
            "Pattern does not match: expected Bytes",
        ),
        (
            "def x is Array = 1",
            "Pattern does not match: expected an Array",
        ),
        ("def x is Map = 1", "Pattern does not match: expected a Map"),
        (
            "def x is Function = 1",
            "Pattern does not match: expected a Function",
        ),
        (
            "def x is Opaque = 1",
            "Pattern does not match: expected an Opaque",
        ),
        (
            "def x is Primitive = [1]",
            "Pattern does not match: expected a Primitive",
        ),
        (
            "def x is Numeric = 's'",
            "Pattern does not match: expected a Numeric",
        ),
        (
            "def x is Structured = 1",
            "Pattern does not match: expected a Structured",
        ),
        (
            "def x is Flat = [1]",
            "Pattern does not match: expected a Flat",
        ),
        (
            "def x is Nonnull = null",
            "Pattern does not match: expected a Nonnull",
        ),
    ]);
}

#[test]
fn a_type_constraint_raises_at_every_position() {
    assert_raises_exactly(&[
        (
            "def [a, b is String] = [1, 2]",
            "Pattern does not match: expected a String",
        ),
        (
            "def {host is String} = {host: 1}",
            "Pattern does not match: expected a String",
        ),
        (
            "def {a: {b is Bool}} = {a: {b: 1}}",
            "Pattern does not match: expected a Bool",
        ),
        (
            "def [_ is Int, a] = ['s', 2]",
            "Pattern does not match: expected an Int",
        ),
    ]);
}

#[test]
fn a_literal_raises_quoting_its_source() {
    assert_raises_exactly(&[
        (
            "def [a, 1] = [0, 2]",
            "Pattern does not match: expected `1`",
        ),
        (
            "def [a, -1] = [0, 1]",
            "Pattern does not match: expected `-1`",
        ),
        (
            "def [a, 1.5] = [0, 2.5]",
            "Pattern does not match: expected `1.5`",
        ),
        (
            "def [a, 'ok'] = [0, 'no']",
            "Pattern does not match: expected `'ok'`",
        ),
        (
            r#"def [a, "ok"] = [0, "no"]"#,
            r#"Pattern does not match: expected `"ok"`"#,
        ),
        (
            r#"def [a, R"(raw)"] = [0, "no"]"#,
            r#"Pattern does not match: expected `R"(raw)"`"#,
        ),
        (
            "def [a, x'01'] = [0, x'02']",
            "Pattern does not match: expected `x'01'`",
        ),
        (
            r#"def [a, $"q${1}"] = [0, "z"]"#,
            r#"Pattern does not match: expected `$"q${1}"`"#,
        ),
        (
            "def [a, null] = [0, 1]",
            "Pattern does not match: expected `null`",
        ),
        (
            "def [a, true] = [0, false]",
            "Pattern does not match: expected `true`",
        ),
        (
            "def [a, false] = [0, null]",
            "Pattern does not match: expected `false`",
        ),
    ]);
}

#[test]
fn a_pin_raises_quoting_its_source() {
    assert_raises_exactly(&[
        (
            "def [a, (1 + 1)] = [0, 3]",
            "Pattern does not match: expected the value of `(1 + 1)`",
        ),
        (
            "def [a, (a)] = [0, 1]",
            "Pattern does not match: expected the value of `(a)`",
        ),
    ]);
}

#[test]
fn an_alternative_nothing_matches_raises_at_any_depth() {
    assert_raises_exactly(&[
        (
            "def [a, 1] | [1, a] = [3, 4]",
            "Pattern does not match: the value matches no alternative",
        ),
        (
            "def [a] | a is Int = 's'",
            "Pattern does not match: the value matches no alternative",
        ),
        (
            "def [a, 1 | 2] = [0, 3]",
            "Pattern does not match: the value matches no alternative",
        ),
        (
            "def {k: 1 | 2, v} = {k: 3, v: 0}",
            "Pattern does not match: the value matches no alternative",
        ),
        (
            "def [a, [b] | [b, 0]] = [1, [2, 3]]",
            "Pattern does not match: the value matches no alternative",
        ),
        (
            "def [a, [b, 1 | 2] | b is Int] = [1, [3, 4]]",
            "Pattern does not match: the value matches no alternative",
        ),
        (
            "def x is Int | x is String = null",
            "Pattern does not match: the value matches no alternative",
        ),
    ]);
}

#[test]
fn a_missing_key_raises_its_lookup_error_outside_an_alternative() {
    assert_raises_exactly(&[
        (
            "def {a} = {b: 1}",
            r#"Map has no value at key "a"; did you mean "b"?"#,
        ),
        (
            "def [{a}] = [{b: 1}]",
            r#"Map has no value at key "a"; did you mean "b"?"#,
        ),
        (
            "def {k: [a] | a, z} = {k: 1}",
            r#"Map has no value at key "z"; did you mean "k"?"#,
        ),
    ]);
}

#[test]
fn a_missing_key_inside_an_alternative_is_a_plain_mismatch() {
    assert_raises_exactly(&[
        (
            "def {a} | [a] = {b: 1}",
            "Pattern does not match: the value matches no alternative",
        ),
        (
            "def [{a} | {b: a}] = [{c: 1}]",
            "Pattern does not match: the value matches no alternative",
        ),
        (
            "def {k: {a} | {b: a}} = {k: {c: 1}}",
            "Pattern does not match: the value matches no alternative",
        ),
    ]);
    // A later branch still gets its turn after an earlier one lacks the key.
    assert_values(&[
        (
            r"
            def {a} | {b: a} = {b: 1}
            a
            ",
            "1",
        ),
        (
            r"
            def [{a, c} | {b: a, c}] = [{b: 1, c: 2}]
            [a, c]
            ",
            "[1, 2]",
        ),
    ]);
}

#[test]
fn a_mismatch_ends_the_program() {
    let script = Script::new(
        r"
        print('before')
        def [a, 1] = [0, 2]
        print('after')
        a
        ",
    );
    assert_eq!(
        script.raises(),
        format!("{NO_MATCH}expected `1`"),
        "the literal check raises"
    );
    assert_eq!(script.printed(), vec!["before"], "nothing after it runs");
}

#[test]
fn a_mismatch_raised_in_a_function_is_catchable() {
    assert_values(&[(
        r"
        defn first_is_one(v) -> {
            def [1, rest] = v
            rest
        }
        [try_call(fn -> first_is_one([2, 3])).ok, try_call(fn -> first_is_one([1, 3])).ok]
        ",
        "[false, true]",
    )]);
}

// --- A def that binds no name ---

/// The names the `binds nothing` examples refer to.
fn nameless(source: &str) -> Script {
    ["f", "p", "m", "xs", "x"]
        .into_iter()
        .fold(Script::new(source), Script::in_scope)
}

#[test]
fn a_def_that_binds_no_name_is_a_compile_error() {
    for source in [
        "def _ = f()",
        "def [_, _] = p",
        "def {a: _} = m",
        "def [] = x",
        "def {} = m",
        "def 1 = x",
        "def [..._] = xs",
        "def {a: _} as _ = m",
        "def _ is Int = x",
        "def null = x",
        "def (1 + 1) = x",
        "def [[], {}] = x",
        "def 1 | 2 = x",
        "def _ | [] = x",
        "def -1 = x",
        "def 'ok' = x",
        "def true | false = x",
    ] {
        assert_binds_nothing(nameless(source));
    }
}

/// Assert `script` fails to compile with exactly one diagnostic, the
/// `binds nothing` one, and that its code, message, label, and help are each
/// present as an exact line of the rendering.
fn assert_binds_nothing(script: Script) {
    let errors = script.compile_errors();
    assert_eq!(errors.len(), 1, "exactly one diagnostic");
    let rendered = errors.render_plain();
    let lines: Vec<&str> = rendered.lines().collect();
    assert_eq!(
        lines.first().copied(),
        Some("binds nothing"),
        "the first line is the code:\n{rendered}"
    );
    assert!(
        lines.contains(&"  x this `def` binds no name"),
        "the message is exact:\n{rendered}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.trim_start().ends_with("`-- no name bound here")),
        "the label is exact:\n{rendered}"
    );
    assert!(
        lines.contains(&"  help: to check a value without binding it, use `assert` or `match`"),
        "the help is exact:\n{rendered}"
    );
}

#[test]
fn binds_nothing_names_its_code_message_label_and_help() {
    let rendered = nameless("def [_, _] = p").compile_errors().render_plain();
    assert!(
        rendered.contains("def [_, _] = p"),
        "the diagnostic quotes the source:\n{rendered}"
    );
    assert!(
        !rendered.contains("`-- binds nothing"),
        "the label no longer repeats the code:\n{rendered}"
    );
    assert_binds_nothing(nameless("def [_, _] = p"));
}

#[test]
fn a_literal_of_any_kind_binds_no_name() {
    for source in [
        "def x'00ff' = x",
        r"def $'plain' = x",
        r"def R'(raw\n)' = x",
        "def [x'00', 1.5] = x",
    ] {
        assert_binds_nothing(nameless(source));
    }
}

#[test]
fn a_pin_binds_no_name() {
    assert_binds_nothing(Script::new(
        r"
        def n = 1
        def (n) = 1
        ",
    ));
    assert_binds_nothing(Script::new(
        r"
        def n = 1
        def [(n)] = [1]
        ",
    ));
}

#[test]
fn a_pin_of_an_unbound_name_reports_binds_nothing() {
    // The check runs on the pattern before it compiles, so the unbound `x`
    // is not what is reported.
    assert_binds_nothing(Script::new("def (x) = 5"));
}

#[test]
fn an_exported_def_that_binds_no_name_is_a_compile_error() {
    let rendered = nameless("export def [_, 1] = xs")
        .compile_errors()
        .render_plain();
    assert!(rendered.contains("this `def` binds no name"), "{rendered}");
}

#[test]
fn a_def_that_binds_no_name_is_an_error_in_any_scope() {
    for source in [
        r"
        defn g(v) -> {
            def [_, _] = v
            0
        }
        ",
        r"
        do {
            def {} = {}
            0
        }
        ",
    ] {
        let rendered = compile_errors(source).render_plain();
        assert!(
            rendered.contains("this `def` binds no name"),
            "{source:?}:\n{rendered}"
        );
    }
}

#[test]
fn the_binds_nothing_check_looks_at_the_whole_pattern() {
    // Each of these binds a name somewhere, so none is rejected.
    assert_values(&[
        (
            r"
            def [a, 1] = [2, 1]
            a
            ",
            "2",
        ),
        (
            r"
            def [_, b] = [1, 2]
            b
            ",
            "2",
        ),
        (
            r"
            def {} as m = {}
            m
            ",
            "{}",
        ),
    ]);
}

// --- Binding-set errors ---

#[test]
fn alternatives_must_bind_the_same_names() {
    for source in [
        "def x is Int | _ = 1",
        "def _ is Int | x = 1",
        "def 1 | x is Int | 2 = 1",
        "def [x, y] | [x] = [1]",
        "def [x] | [y] = [1]",
        "def {a} | {b} = {a: 1}",
        "def [a, 1 | b] = [1, 1]",
        "def [x, 0] | [0, [x] | {y}] = [1, 0]",
        "def {a} as m | {a} = {a: 1}",
        "def [x, ...r] | [x] = [1]",
    ] {
        let rendered = compile_errors(source).render_plain();
        assert!(
            rendered.contains("every alternative must bind the same names")
                && rendered.contains("alternative bindings"),
            "{source:?}:\n{rendered}"
        );
    }
}

#[test]
fn a_name_bound_twice_in_a_def_pattern_is_a_compile_error() {
    assert_compile_errors(&[
        ("def [x, x] = [1, 2]", "`x` is already bound"),
        ("def {a} as a = {a: 1}", "`a` is already bound"),
        ("def [x, {x}] = [1, {x: 2}]", "`x` is already bound"),
        ("def [x, ...x] = [1, 2]", "`x` is already bound"),
        ("def [x, x is Int] = [1, 2]", "`x` is already bound"),
        ("def [x, [x] | [x]] = [1, [2]]", "`x` is already bound"),
        ("def [x, x is Int | x] = [1, 2]", "`x` is already bound"),
        ("def x is Int | [x, x] = 1", "`x` is already bound"),
        ("def [x, 0] | [x, x] = [1, 0]", "`x` is already bound"),
        (
            r"
            def x = 1
            def [x, 1] | [1, x] = [1, 1]
            ",
            "`x` is already bound",
        ),
    ]);
}

#[test]
fn a_pin_may_not_name_a_part_not_yet_bound() {
    assert_compile_errors(&[
        ("def [(b), b] = [1, 1]", "`b` is not defined"),
        ("def [a, (b)] = [1, 1]", "`b` is not defined"),
    ]);
}

// --- Export ---

#[test]
fn an_exported_alternative_exports_its_name_once() {
    for (source, expected) in [
        (
            r"
            export def {a} | [a] = [5]
            0
            ",
            [("a", "5")],
        ),
        (
            r"
            export def {a} | [a] = {a: 6}
            0
            ",
            [("a", "6")],
        ),
        (
            r"
            export def [a, 1] | [1, a] = [1, 2]
            0
            ",
            [("a", "2")],
        ),
    ] {
        let finished = Script::new(source).finish();
        let wanted: Vec<_> = expected
            .into_iter()
            .map(|(name, value)| (name.to_string(), run(value)))
            .collect();
        assert_eq!(
            finished.exports.into_iter().collect::<Vec<_>>(),
            wanted,
            "{source:?}"
        );
        assert_eq!(finished.tail, Value::Int(0), "{source:?}");
    }
}

#[test]
fn an_exported_pattern_exports_every_name_it_binds() {
    let finished = Script::new(
        r"
        export def {k: x is Int, y} as whole = {k: 1, y: [2]}
        export def [a, 1, ..._] | [_, a] = [3, 1, 4]
        def [hidden, 1] = [9, 1]
        0
        ",
    )
    .finish();
    assert_eq!(
        finished.exports.keys().collect::<Vec<_>>(),
        vec!["a", "whole", "x", "y"],
        "{finished:?}"
    );
    assert_eq!(finished.exports["x"], run("1"));
    assert_eq!(finished.exports["a"], run("3"));
}

#[test]
fn an_implicit_export_covers_every_form() {
    for (source, expected) in [
        (
            r"
            def {a} | [a] = [5]
            0
            ",
            vec![("a", "5")],
        ),
        (
            r"
            def [a, 1] | [1, a] = [1, 2]
            0
            ",
            vec![("a", "2")],
        ),
        (
            r"
            def x is Int = 4
            0
            ",
            vec![("x", "4")],
        ),
        (
            r"
            def [a, 1, (a)] = [7, 1, 7]
            0
            ",
            vec![("a", "7")],
        ),
        (
            r"
            def {h is String, p: [q, ..._]} = {h: 'x', p: [1, 2]}
            0
            ",
            vec![("h", "'x'"), ("q", "1")],
        ),
    ] {
        let finished = Script::new(source).implicit_export().finish();
        let wanted: Vec<_> = expected
            .into_iter()
            .map(|(name, value)| (name.to_string(), run(value)))
            .collect();
        assert_eq!(
            finished.exports.into_iter().collect::<Vec<_>>(),
            wanted,
            "{source:?}"
        );
    }
}

#[test]
fn a_def_without_export_exports_nothing() {
    let finished = Script::new(
        r"
        def {a} | [a] = [5]
        a
        ",
    )
    .finish();
    assert!(finished.exports.is_empty(), "{finished:?}");
    assert_eq!(finished.tail, Value::Int(5));
}

// --- Scope ---

#[test]
fn the_value_is_evaluated_before_the_pattern_binds() {
    // The pin `(x)` and the value `x` are different bindings of `x`.
    assert_values(&[(
        r"
        def x = 9
        do {
            def [x, (x)] = [x, 9]
            x
        }
        ",
        "9",
    )]);
}

#[test]
fn every_form_works_in_any_scope() {
    assert_values(&[
        (
            r"
            defn f(p) -> {
                def [a is Int, 1] | [1, a is Int] = p
                a
            }
            [f([4, 1]), f([1, 5])]
            ",
            "[4, 5]",
        ),
        (
            r"
            do {
                def {k: 'v', z} = {k: 'v', z: 1}
                z
            }
            ",
            "1",
        ),
        (
            r"
            def g = fn p -> {
                def {a} | [a] = p
                a
            }
            [g({a: 1}), g([2])]
            ",
            "[1, 2]",
        ),
    ]);
}
