//! `match` expressions, end to end: `match target { pattern if: guard => result, ... }`.
//!
//! The target is evaluated once, then tried against each arm in source order:
//! the first arm whose pattern matches, and whose guard (if any) is truthy, gives
//! the result, and no matching arm raises. A pattern is a name (optionally
//! type-constrained), a value to compare with, an Array or Map pattern, or
//! alternatives, which must all bind the same names. Each arm is its own scope.
//!
//! The harness runs every behavioral case under every optimization permutation;
//! code-shape cases pin exactly the options they are about.

mod script;

use frostlang::bytecode::Bytecode;
use frostlang::compile::{Optimization, OptimizationOptions};
use frostlang::{MapKey, Value};
use script::{
    Emitted, Script, UNOPTIMIZED, compile_errors, optimization_permutations, raises, run,
};

const FOLD: OptimizationOptions = UNOPTIMIZED.with(Optimization::ConstantFold, true);

/// Defines `note(v)`, which appends `v` to a log and returns it, and `log()`,
/// the values noted so far, in order. A script appends its own statements.
const NOTE: &str = r"
    def cell = mutable_cell([])
    defn note(v) -> {
        cell.exchange(cell.get() + [v])
        v
    }
    defn log() -> cell.get()
";

/// Assert each `source` runs to the value of the Frost expression `expected`.
fn assert_values(cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(run(source), run(expected), "{source:?} is {expected}");
    }
}

/// Assert each `source`, run after [`NOTE`], runs to the value of `expected`.
fn assert_noted(cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        assert_eq!(
            run(&format!("{NOTE}{source}")),
            run(expected),
            "{source:?} is {expected}"
        );
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

/// How many calls the code makes: `(ordinary, tail)`.
fn calls_by_kind(emitted: &Emitted) -> (usize, usize) {
    let count =
        |matching: fn(&Bytecode) -> bool| emitted.code.iter().filter(|op| matching(op)).count();
    (
        count(|op| matches!(op, Bytecode::Call(_))),
        count(|op| matches!(op, Bytecode::TailCall(_))),
    )
}

// --- Choosing an arm ---

#[test]
fn the_first_matching_arm_is_taken() {
    assert_values(&[
        (r#"match 1 { 1 => "a", 1 => "b" }"#, r#""a""#),
        (r#"match 2 { 1 => "a", 2 => "b", _ => "c" }"#, r#""b""#),
        (r#"match 3 { 1 => "a", 2 => "b", _ => "c" }"#, r#""c""#),
        ("match 3 { x => x, _ => 0 }", "3"),
    ]);
}

#[test]
fn the_target_is_evaluated_once_before_any_arm() {
    assert_noted(&[
        (
            r"
            def r = match note(5) { 1 => 0, 2 => 0, x => x }
            [r, log()]
            ",
            "[5, [5]]",
        ),
        (
            r"
            def r = match note(5) { x if: note(x) == 0 => 0, _ => 1 }
            [r, log()]
            ",
            "[1, [5, 5]]",
        ),
    ]);
}

#[test]
fn arms_are_tried_in_order_and_only_the_taken_result_is_evaluated() {
    assert_noted(&[
        (
            r#"
            match 2 { 1 => note("a"), 2 => note("b"), _ => note("c") }
            log()
            "#,
            r#"["b"]"#,
        ),
        (
            r"
            match 3 {
                x if: note(1) == 0 => 0,
                y if: note(2) == 0 => 0,
                z if: note(3) == 3 => z,
                _ if: note(4) => 0
            }
            log()
            ",
            "[1, 2, 3]",
        ),
    ]);
}

#[test]
fn no_matching_arm_raises_naming_the_value() {
    assert_raises(&[
        ("match 5 { 1 => 0 }", "No match arm matches the value: 5"),
        (
            r#"match "text" { 1 => 0 }"#,
            "No match arm matches the value: text",
        ),
        ("match 1 {}", "No match arm matches the value: 1"),
        (
            "match 1 { x if: false => 0 }",
            "No match arm matches the value: 1",
        ),
        (
            "match [1, 2] { [x] => 0, {} => 0 }",
            "No match arm matches the value",
        ),
    ]);
}

#[test]
fn a_raising_target_raises() {
    assert_raises(&[("match (1 / 0) { _ => 1 }", "Division by zero")]);
}

#[test]
fn a_match_leaves_exactly_its_value_among_others() {
    assert_values(&[
        ("[0, match 1 { 1 => 2 }, 3]", "[0, 2, 3]"),
        ("1 + match [2, 3] { [a, b] => a * b } + 10", "17"),
        ("plus(match {a: 1} { {a} => a }, match 2 { x => x })", "3"),
        (r#"$'<${match 1 { 1 => "one" }}>'"#, r#""<one>""#),
        ("{k: match [1] { [x] | x => x }}", "{k: 1}"),
        ("match match 1 { x => [x, x] } { [a, b] => a + b }", "2"),
        // Arms abandoned partway through their patterns leave nothing behind.
        (
            r"
            [
                match [1, [2, 3]] {
                    [x, [y]] => 0,
                    [x, [y, z, w]] => 0,
                    {a} => 0,
                    [x, [y, z]] => x + y + z
                },
                4
            ]
            ",
            "[6, 4]",
        ),
    ]);
}

#[test]
fn a_match_statement_leaves_the_stack_balanced() {
    assert_values(&[
        (
            r"
            match [1, 2] { [a, ...r] => a }
            match {a: 1} { {a: x} => x }
            7
            ",
            "7",
        ),
        (
            r"
            match [1, [2]] { [x, [0]] => 0, [x, [y]] => y }
            match 3 { 1 | 2 => 0, _ => 1 }
            7
            ",
            "7",
        ),
        // Repeated many times over, any imbalance would accumulate.
        (
            r"
            defn go(n) -> if n == 0: 0 else: do {
                match [n, n] { [a, b] | {a, b} => a }
                match n { 1 | 2 => null, _ => null }
                go(n - 1)
            }
            go(1000)
            ",
            "0",
        ),
    ]);
}

#[test]
fn a_match_in_a_lambda_matches_afresh_on_each_call() {
    assert_values(&[(
        r"
        def f = fn v -> match v { [x] => x, {x} => x, _ => 0 }
        [f([1]), f({x: 2}), f(3), f([4])]
        ",
        "[1, 2, 0, 4]",
    )]);
}

// --- Names ---

#[test]
fn a_name_binds_the_whole_value() {
    assert_values(&[
        ("match [1, 2] { v => v }", "[1, 2]"),
        ("match null { v => [v] }", "[null]"),
    ]);
}

#[test]
fn a_discard_matches_anything() {
    assert_values(&[
        ("match 1 { _ => 0 }", "0"),
        ("match null { _ => 0 }", "0"),
        ("match plus { _ => 0 }", "0"),
    ]);
}

#[test]
fn a_type_constraint_admits_only_its_types() {
    // Each constraint over: null, true, 1, 1.5, "s", x'00', [1], {a: 1}, plus.
    for (constraint, admits) in [
        ("Null", "[1, 0, 0, 0, 0, 0, 0, 0, 0]"),
        ("Bool", "[0, 1, 0, 0, 0, 0, 0, 0, 0]"),
        ("Int", "[0, 0, 1, 0, 0, 0, 0, 0, 0]"),
        ("Float", "[0, 0, 0, 1, 0, 0, 0, 0, 0]"),
        ("String", "[0, 0, 0, 0, 1, 0, 0, 0, 0]"),
        ("Bytes", "[0, 0, 0, 0, 0, 1, 0, 0, 0]"),
        ("Array", "[0, 0, 0, 0, 0, 0, 1, 0, 0]"),
        ("Map", "[0, 0, 0, 0, 0, 0, 0, 1, 0]"),
        ("Function", "[0, 0, 0, 0, 0, 0, 0, 0, 1]"),
        // Frost source cannot make an Opaque; the runtime's tests match a host's.
        ("Opaque", "[0, 0, 0, 0, 0, 0, 0, 0, 0]"),
        ("Primitive", "[1, 1, 1, 1, 1, 1, 0, 0, 0]"),
        ("Numeric", "[0, 0, 1, 1, 0, 0, 0, 0, 0]"),
        ("Structured", "[0, 0, 0, 0, 0, 0, 1, 1, 0]"),
        ("Flat", "[0, 0, 0, 0, 1, 1, 0, 0, 0]"),
        ("Nonnull", "[0, 1, 1, 1, 1, 1, 1, 1, 1]"),
    ] {
        for pattern in [format!("_ is {constraint}"), format!("v is {constraint}")] {
            let source = format!(
                r#"transform([null, true, 1, 1.5, "s", x'00', [1], {{a: 1}}, plus], fn v -> match v {{ {pattern} => 1, _ => 0 }})"#
            );
            assert_eq!(run(&source), run(admits), "{pattern}");
        }
    }
}

#[test]
fn a_constrained_name_binds_the_value() {
    assert_values(&[
        (
            r#"match 1.5 { n is Int => ["Int", n], n is Float => ["Float", n] }"#,
            r#"["Float", 1.5]"#,
        ),
        ("match [1] { xs is Map => 0, xs is Array => xs }", "[1]"),
    ]);
}

#[test]
fn a_name_may_shadow_an_enclosing_name_or_global() {
    assert_values(&[
        (
            r"
            def x = 1
            [match 2 { x => x }, x]
            ",
            "[2, 1]",
        ),
        ("match 5 { plus => plus }", "5"),
        (
            r"
            def x = [1]
            match x { [x] => x }
            ",
            "1",
        ),
        ("(fn v -> match v { [v] | {v} => v })([4])", "4"),
        // A hoisted capture, shadowed by the arm, and read by values and keys.
        (
            r"
            def k = 3
            def f = fn v -> match v { k if: k > 5 => k, _ => k }
            [f(9), f(1)]
            ",
            "[9, 3]",
        ),
        (
            r#"
            def k = 3
            def f = fn v -> match v {
                (k) => "k",
                [(k), k2] => k2,
                {[k]: z} => z,
                _ => 0
            }
            [f(3), f([3, 4]), f({[3]: 5}), f(1)]
            "#,
            r#"["k", 4, 5, 0]"#,
        ),
    ]);
}

// --- Values ---

#[test]
fn a_literal_matches_an_equal_value() {
    for literal in [
        "null", "true", "false", "0", "-1", "1.5", "-2.5", r#""s""#, r#""""#, "x'00'", "x''",
    ] {
        let source = format!(r#"match {literal} {{ {literal} => "yes", _ => "no" }}"#);
        assert_eq!(run(&source), Value::from("yes"), "{source}");
    }
}

#[test]
fn a_literal_matches_only_its_own_type() {
    assert_values(&[
        (r#"match 1 { 1.0 => "Float", 1 => "Int" }"#, r#""Int""#),
        (r#"match 1.0 { 1 => "Int", 1.0 => "Float" }"#, r#""Float""#),
        (
            r#"match 0 { false => "Bool", null => "Null", 0 => "Int" }"#,
            r#""Int""#,
        ),
        (
            r#"match "1" { 1 => "Int", "1" => "String" }"#,
            r#""String""#,
        ),
        (
            r#"match x'61' { "a" => "String", x'61' => "Bytes" }"#,
            r#""Bytes""#,
        ),
        (
            r#"match null { false => "Bool", null => "Null" }"#,
            r#""Null""#,
        ),
    ]);
}

#[test]
fn a_parenthesized_expression_is_compared_by_value() {
    assert_values(&[
        (
            r#"
            def k = 3
            match 3 { (k) => "k", _ => "other" }
            "#,
            r#""k""#,
        ),
        (
            r#"
            def k = 4
            match 3 { (k) => "k", _ => "other" }
            "#,
            r#""other""#,
        ),
        ("match 4 { (2 + 2) => 1, _ => 0 }", "1"),
        ("match [1, {a: 2}] { ([1, {a: 2}]) => 1, _ => 0 }", "1"),
        (
            r"
            def f = fn -> 1
            match f { (f) => 1, _ => 0 }
            ",
            "1",
        ),
        // Two evaluations of a lambda are two distinct closures.
        ("match (fn -> 1) { (fn -> 1) => 1, _ => 0 }", "0"),
    ]);
}

#[test]
fn a_value_may_be_an_earlier_binding_of_the_same_pattern() {
    assert_values(&[
        (
            r#"match [1, 1] { [a, (a)] => "same", _ => "different" }"#,
            r#""same""#,
        ),
        (
            r#"match [1, 2] { [a, (a)] => "same", _ => "different" }"#,
            r#""different""#,
        ),
        ("match {a: 1, b: 1} { {a, b: (a)} => 1, _ => 0 }", "1"),
        ("match [2, [2]] { [n, [(n)]] => n, _ => 0 }", "2"),
    ]);
}

#[test]
fn a_value_is_evaluated_only_when_its_pattern_reaches_it() {
    assert_noted(&[
        (
            r"
            match [1, 2] {
                [(note(0)), _] => 0,
                [_, (note(3))] => 0,
                [(note(1)), (note(2))] => log()
            }
            ",
            "[0, 3, 1, 2]",
        ),
        ("match 1 { 2 => 0, _ => log(), (note(9)) => 0 }", "[]"),
        // The length check comes first.
        (
            "match [1] { [(note(1)), (note(2))] => 0, _ => log() }",
            "[]",
        ),
    ]);
}

#[test]
fn a_raising_value_raises_only_if_reached() {
    assert_raises(&[("match 1 { (1 / 0) => 0, _ => 1 }", "Division by zero")]);
    assert_values(&[("match 1 { _ => 1, (1 / 0) => 0 }", "1")]);
}

// --- Arrays ---

#[test]
fn an_array_pattern_matches_by_length_and_elements() {
    assert_values(&[
        (
            "match [1, 2] { [a] => 1, [a, b] => 2, [a, b, c] => 3 }",
            "2",
        ),
        ("match [] { [a] => 1, [] => 0 }", "0"),
        ("match [1, 2, 3] { [a, ...r] => [a, r] }", "[1, [2, 3]]"),
        ("match [1] { [a, ...r] => [a, r] }", "[1, []]"),
        ("match [] { [a, ...r] => 1, [...r] => r }", "[]"),
        (r#"match [1, 2] { [..._] => "any" }"#, r#""any""#),
        ("match [1, 2] { [1, x] => x, _ => 0 }", "2"),
        ("match [2, 2] { [1, x] => x, _ => 0 }", "0"),
        (
            "match [1, 2, 3] { [a, b] => 0, [a, b, c, ...r] => [a, b, c, r] }",
            "[1, 2, 3, []]",
        ),
    ]);
}

#[test]
fn a_non_array_does_not_match_an_array_pattern() {
    for value in ["5", "null", r#""ab""#, "x'0000'", "{a: 1}", "plus"] {
        let source =
            format!("match {value} {{ [] => 1, [..._] => 2, [x] => 3, [x, y] => 4, _ => 0 }}");
        assert_eq!(run(&source), Value::Int(0), "{source}");
    }
}

#[test]
fn array_patterns_nest() {
    assert_values(&[
        ("match [[1, 2], [3]] { [[a, b], [c]] => a + b + c }", "6"),
        (
            "match [[1], [2, 3]] { [[a], [b]] => 0, [[a], [b, c]] => a + b + c }",
            "6",
        ),
        (
            "match [1, [2, [3, [4]]]] { [a, [b, [c, [d]]]] => [d, c, b, a] }",
            "[4, 3, 2, 1]",
        ),
        ("match [[1, 2, 3]] { [[h, ...t]] => [h, t] }", "[1, [2, 3]]"),
    ]);
}

// --- Maps ---

#[test]
fn a_map_pattern_matches_the_keys_it_names() {
    assert_values(&[
        ("match {a: 1, b: 2} { {a, b} => a + b }", "3"),
        ("match {a: 1} { {a, b} => 0, {a} => a }", "1"),
        ("match {a: 1, b: 2, c: 3} { {c: x} => x }", "3"),
        ("match {a: null} { {a} => [a], _ => 0 }", "[null]"),
        ("match {} { {a} => 1, {} => 0 }", "0"),
        ("match {a: 1} { {} as m => m }", "{a: 1}"),
        (
            "match {a: 1, b: 2} { {a} as m => [a, m] }",
            "[1, {a: 1, b: 2}]",
        ),
        (
            r#"match {a: 1} { {a: 2} => "two", {a: 1} => "one" }"#,
            r#""one""#,
        ),
        ("match {a: 1} { {a: _} => 1 }", "1"),
        ("match {a: 1} { {a: _} as _ => 1 }", "1"),
    ]);
}

#[test]
fn a_computed_key_matches_by_value() {
    assert_values(&[
        (
            r#"
            match {[1]: "i", [true]: "t", [x'00']: "b", [1.5]: "f"} {
                {[1]: i, [true]: t, [x'00']: b, [1.5]: f} => [i, t, b, f]
            }
            "#,
            r#"["i", "t", "b", "f"]"#,
        ),
        ("match {[1.0]: 2} { {[1]: x} => x, _ => 0 }", "0"),
        (
            r#"
            def k = "b"
            match {b: 5} { {[k]: v} => v }
            "#,
            "5",
        ),
        (r#"match {a: "c", c: 3} { {a, [a]: b} => b }"#, "3"),
        (r#"match {a: "z", c: 3} { {a, [a]: b} => b, _ => 0 }"#, "0"),
    ]);
}

#[test]
fn keys_are_evaluated_and_looked_up_in_turn() {
    assert_noted(&[
        (
            r#"match {a: 1} { {[note("a")]: x, [note("b")]: y} => 0, _ => log() }"#,
            r#"["a", "b"]"#,
        ),
        // The type check comes first.
        (r#"match 5 { {[note("a")]: x} => 0, _ => log() }"#, "[]"),
        // A part mismatch stops the entries after it.
        (
            r#"match {a: 1, b: 2} { {[note("a")]: 2, [note("b")]: y} => 0, _ => log() }"#,
            r#"["a"]"#,
        ),
    ]);
}

#[test]
fn a_non_map_does_not_match_a_map_pattern() {
    for value in ["5", "null", r#""ab""#, "x'00'", "[1]", "[]", "plus"] {
        let source = format!("match {value} {{ {{}} => 1, {{}} as m => 2, {{a}} => 3, _ => 0 }}");
        assert_eq!(run(&source), Value::Int(0), "{source}");
    }
}

#[test]
fn an_invalid_computed_key_raises() {
    assert_raises(&[
        (
            "match {a: 1} { {[null]: x} => x, _ => 0 }",
            "not a valid Map key",
        ),
        (
            "match {a: 1} { {[[1]]: x} => x, _ => 0 }",
            "not a valid Map key",
        ),
    ]);
    // Never evaluated against a non-Map.
    assert_values(&[("match 5 { {[null]: x} => x, _ => 0 }", "0")]);
}

#[test]
fn map_and_array_patterns_nest_in_each_other() {
    assert_values(&[
        ("match {a: [1, {b: 2}]} { {a: [x, {b}]} => x + b }", "3"),
        ("match [{a: 1}, {a: 2, b: 3}] { [{a}, {b}] => a + b }", "4"),
        ("match {a: [1]} { {a: [x, y]} => 0, {a: [x]} => x }", "1"),
        ("match {p: {q: {r: 7}}} { {p: {q: {r}}} => r }", "7"),
        (
            "match [{a: 1} , 2] { [{a} as m, n] => [m, n] }",
            "[{a: 1}, 2]",
        ),
    ]);
}

// --- Guards ---

#[test]
fn a_guard_filters_a_matched_arm() {
    assert_values(&[
        (
            r#"match 5 { x if: x > 9 => "big", x => "small" }"#,
            r#""small""#,
        ),
        (
            r#"match 50 { x if: x > 9 => "big", x => "small" }"#,
            r#""big""#,
        ),
        (
            r#"match [1, 2] { [a, b] if: a > b => "desc", [a, b] => "asc" }"#,
            r#""asc""#,
        ),
        ("match 1 { _ if: true => 1 }", "1"),
    ]);
}

#[test]
fn a_guard_is_tested_for_truthiness() {
    assert_values(&[
        ("match 1 { _ if: 0 => 1, _ => 2 }", "1"),
        (r#"match 1 { _ if: "" => 1, _ => 2 }"#, "1"),
        ("match 1 { _ if: [] => 1, _ => 2 }", "1"),
        ("match 1 { _ if: null => 1, _ => 2 }", "2"),
        ("match 1 { _ if: false => 1, _ => 2 }", "2"),
    ]);
}

#[test]
fn a_guard_runs_only_after_its_pattern_matches() {
    assert_noted(&[
        (
            r#"
            match [1] {
                [a, b] if: note("two") => 0,
                [a] if: note("one") => log(),
                _ if: note("any") => 0
            }
            "#,
            r#"["one"]"#,
        ),
        (
            "match 3 { x if: note(x) == 0 => 0, x if: note(x + 1) == 4 => log() }",
            "[3, 4]",
        ),
    ]);
}

#[test]
fn a_failed_guard_falls_through_with_the_stack_restored() {
    assert_values(&[(
        r"
        [
            match [1, [2, 3]] {
                [a, [b, c]] if: false => 0,
                {a} => 0,
                [a, [b, c]] => a + b + c
            },
            9
        ]
        ",
        "[6, 9]",
    )]);
}

#[test]
fn a_raising_guard_raises() {
    assert_raises(&[("match 1 { x if: x / 0 => 0, _ => 1 }", "Division by zero")]);
}

// --- Alternatives ---

#[test]
fn the_first_matching_alternative_is_taken() {
    assert_values(&[
        (r#"match 2 { 1 | 2 | 3 => "low", _ => "high" }"#, r#""low""#),
        (
            r#"match 4 { 1 | 2 | 3 => "low", _ => "high" }"#,
            r#""high""#,
        ),
        (
            r#"match null { null | false => "falsy", _ => "truthy" }"#,
            r#""falsy""#,
        ),
        ("match [1] { [x] | x => x }", "1"),
        ("match 5 { [x] | x => x }", "5"),
        ("match [2, 2] { [x, 2] | [2, x] => x }", "2"),
        ("match 1.5 { x is Int | x is Float => x, _ => 0 }", "1.5"),
        (r#"match "s" { x is Int | x is Float => x, _ => 0 }"#, "0"),
        ("match [1, 2] { [a, b] | {a, b} => [a, b] }", "[1, 2]"),
        ("match {a: 1, b: 2} { [a, b] | {a, b} => [a, b] }", "[1, 2]"),
        ("match {b: 1} { {a: x} | {b: x} => x }", "1"),
        // The branches may bind the names in any order.
        ("match [1, 2] { [x, y] | [y, x] => [x, y] }", "[1, 2]"),
        (
            "match {b: 1, c: 2} { {a: x, b: y} | {c: x, b: y} => [x, y] }",
            "[2, 1]",
        ),
    ]);
}

#[test]
fn a_branch_that_fails_after_binding_leaves_nothing_behind() {
    assert_values(&[
        // The first branch binds `x` to 3, then fails; the second rebinds it.
        ("match [3, 5] { [x, 2] | [3, x] => x }", "5"),
        (
            "[match [1, [2, 3]] { [x, [y, 0]] | [x, [0, y]] | [x, [y, _]] => x + y }, 9]",
            "[3, 9]",
        ),
        (
            "[match {a: [1, 2]} { {a: [x, 0]} | {b: x} | {a: [_, x]} => x }, 9]",
            "[2, 9]",
        ),
        // Every branch fails, then the next arm starts clean.
        (
            "[match [1, [2]] { [x, [0]] | [0, [x]] => 0, [a, [b]] => a + b }, 9]",
            "[3, 9]",
        ),
    ]);
}

#[test]
fn alternatives_nest() {
    assert_values(&[
        ("match [2, 7] { [1 | 2, x] => x, _ => 0 }", "7"),
        ("match [3, 7] { [1 | 2, x] => x, _ => 0 }", "0"),
        ("match [1, [5]] { [1 | 2, [x] | x] => x }", "5"),
        ("match [2, 5] { [1 | 2, [x] | x] => x }", "5"),
        (r#"match {a: 2} { {a: 1 | 2} => "a", _ => "b" }"#, r#""a""#),
        // An alternative nested in a later branch binds into the same locals.
        ("match [9, 0] { [x, 0] | [0, [x] | {x}] => x }", "9"),
        ("match [0, [4]] { [x, 0] | [0, [x] | {x}] => x }", "4"),
        ("match [0, {x: 4}] { [x, 0] | [0, [x] | {x}] => x }", "4"),
        (
            "match [0, 4] { [x, 0] | [0, [x] | {x}] => x, _ => -1 }",
            "-1",
        ),
    ]);
}

#[test]
fn a_branch_sees_only_its_own_bindings() {
    // A value in a later branch reads the enclosing name, not the one an
    // earlier branch bound.
    assert_values(&[
        (
            r"
            def x = 7
            match [5, 7, 3] { [x, 1, _] | [_, (x), x] => x, _ => 0 }
            ",
            "3",
        ),
        (
            r"
            def x = 7
            match [7, 3] { [(x), x] | [x, 0] => x, _ => 0 }
            ",
            "3",
        ),
        // Its own binding, once made, is the one it reads.
        (
            r"
            def x = 7
            match [5, 5] { [x, 1] | [x, (x)] => x, _ => 0 }
            ",
            "5",
        ),
        (
            r"
            def x = 7
            match [5, 6] { [x, 1] | [x, (x)] => x, _ => 0 }
            ",
            "0",
        ),
        // Likewise within an alternative nested in a later branch.
        (
            r"
            def x = 7
            match [0, [7, 2]] { [x, 1] | [0, [(x), x] | {x}] => x, _ => 0 }
            ",
            "2",
        ),
    ]);
    assert_compile_errors(&[
        (
            "match [5, 5] { [1, x] | [(x), x] => x, _ => 0 }",
            "`x` is not defined",
        ),
        (
            "match [5, 5] { [x, 1] | [(x), x] => x, _ => 0 }",
            "`x` is not defined",
        ),
        (
            "match [5, [5]] { [x, 1] | [0, [x] | [(x)]] => x, _ => 0 }",
            "`x` is not defined",
        ),
    ]);
}

#[test]
fn alternatives_combine_with_every_pattern_form() {
    assert_values(&[
        // Two alternatives side by side.
        (
            "match [[1, 5], [2, 6]] { [[x, 1] | [1, x], [y, 2] | [2, y]] => [x, y] }",
            "[5, 6]",
        ),
        // Three branches in a later branch.
        (
            "match [3, [4, 5]] { [x, 0, y] | [x, [y, 0] | [0, y] | [_, y]] => [x, y] }",
            "[3, 5]",
        ),
        // Names bound at different depths.
        (
            "match {p: [1, {q: 2}]} { [a, b] | {p: [b, {q: a}]} => [a, b] }",
            "[2, 1]",
        ),
        // A rest in one branch, a Map part in the other.
        (
            "match {a: 1, r: 2} { [a, ...r] | {a, r} => [a, r] }",
            "[1, 2]",
        ),
        (
            "match [1, 2, 3] { [a, ...r] | {a, r} => [a, r] }",
            "[1, [2, 3]]",
        ),
        // In a Map part, beside `as`, and as the `as` name itself.
        (
            "match {a: [1, 2]} { {a: [x, 1] | [1, x] | [x, 2]} as m => [x, m] }",
            "[2, {a: [1, 2]}]",
        ),
        ("match [5] { {a: 1 | 2} as m | [m] => m }", "5"),
        // Constrained names, in either order.
        (
            r#"match ["a", 1] { [x is Int, y is String] | [y is String, x is Int] => [x, y] }"#,
            r#"[1, "a"]"#,
        ),
        (
            "match {name: 1} { {name is String} | {name is Int} => name, _ => 0 }",
            "1",
        ),
        // Empty patterns, and signed literals.
        ("match [{}] { [[] | {}] => 1, _ => 0 }", "1"),
        ("match [[1]] { [[] | {}] => 1, _ => 0 }", "0"),
        (r#"match 1 { -1 | 1 => "one", _ => "other" }"#, r#""one""#),
    ]);
}

#[test]
fn a_later_branch_captures_the_enclosing_name_an_earlier_branch_shadows() {
    // Only a later branch uses the enclosing name, so only that use can make
    // the lambda capture it.
    assert_values(&[
        (
            r"
            def x = 7
            def f = fn v -> match v { [x, 1, _] | [_, (x), x] => x, _ => 0 }
            f([5, 7, 3])
            ",
            "3",
        ),
        // Were `plus` not captured, the branch would compare with the global.
        (
            r"
            def plus = 5
            def g = fn v -> match v { [plus, 1] | [(plus), plus] => plus, _ => 0 }
            g([5, 9])
            ",
            "9",
        ),
        (
            r"
            def y = 2
            (fn v -> match v { [1, [y, 0] | [(y), y]] => y, _ => -1 })([1, [2, 8]])
            ",
            "8",
        ),
        (
            r"
            def y = 1
            (fn v -> match v { {a: y, b: 0} | {[y]: y} => y, _ => -1 })({[1]: 5})
            ",
            "5",
        ),
    ]);
    // A script compiled within an enclosing scope captures it the same way.
    let tail = Script::new("match [5, 9] { [plus, 1] | [(plus), plus] => plus, _ => 0 }")
        .capture("plus", Value::Int(5))
        .run();
    assert_eq!(tail, Value::Int(9));
}

#[test]
fn a_guard_follows_whichever_alternative_matched() {
    assert_values(&[
        (
            "match [5, 1] { [x, 1] | [1, x] if: x > 3 => x, _ => 0 }",
            "5",
        ),
        (
            "match [1, 5] { [x, 1] | [1, x] if: x > 3 => x, _ => 0 }",
            "5",
        ),
        (
            "match [2, 1] { [x, 1] | [1, x] if: x > 3 => x, _ => 0 }",
            "0",
        ),
        // A failed guard fails the arm; later branches are not tried.
        (
            r#"match [1, 2] { [x, _] | [_, x] if: x == 2 => "yes", _ => "no" }"#,
            r#""no""#,
        ),
    ]);
}

#[test]
fn alternatives_must_bind_the_same_names() {
    for source in [
        "match 1 { x | _ => 0 }",
        "match 1 { _ | x => 0 }",
        "match 1 { 1 | x | 2 => 0 }",
        "match 1 { [x, y] | [x] => 0 }",
        "match 1 { [x] | [x, y] => 0 }",
        "match 1 { [x] | [y] => 0 }",
        "match 1 { {a} | {b} => 0 }",
        "match 1 { [a, 1 | b] => 0 }",
        "match 1 { [x, 0] | [0, [x] | {y}] => 0 }",
        "match 1 { [x, 0] | [0, [x] | 1] => 0 }",
        "match 1 { {a} as m | {a} => 0 }",
        "match 1 { [x, ...r] | [x] => 0 }",
    ] {
        let rendered = compile_errors(source).render_plain();
        assert!(
            rendered.contains("every alternative must bind the same names")
                && rendered.contains("alternative bindings"),
            "{source:?}:\n{rendered}"
        );
    }
}

// --- Scope ---

#[test]
fn an_arms_bindings_are_visible_only_in_that_arm() {
    assert_compile_errors(&[
        (
            r"
            match 1 { x => x }
            x
            ",
            "`x` is not defined",
        ),
        ("match 1 { x if: false => 0, _ => x }", "`x` is not defined"),
        (
            r"
            match {a: 1} { {a} as m => 0 }
            m
            ",
            "`m` is not defined",
        ),
        (
            r"
            match [1] { [x] => fn -> x }
            x
            ",
            "`x` is not defined",
        ),
    ]);
}

#[test]
fn each_arm_may_bind_the_same_names() {
    assert_values(&[
        ("match [1, 2] { [x] => x, [x, y] => x + y }", "3"),
        ("match {x: 1} { [x] => x, {x} => x }", "1"),
    ]);
}

#[test]
fn a_name_bound_twice_in_a_pattern_is_a_compile_error() {
    assert_compile_errors(&[
        ("match 1 { [x, x] => 0 }", "`x` is already bound"),
        ("match 1 { {a} as a => 0 }", "`a` is already bound"),
        ("match 1 { [x, {x}] => 0 }", "`x` is already bound"),
        ("match 1 { [x, ...x] => 0 }", "`x` is already bound"),
        ("match 1 { [x, x | x] => 0 }", "`x` is already bound"),
        // Within a later branch of an alternative too.
        ("match 1 { x | [x, x] => 0 }", "`x` is already bound"),
        ("match 1 { [x, 0] | [x, x] => 0 }", "`x` is already bound"),
    ]);
}

#[test]
fn an_unbound_name_is_a_compile_error_even_in_an_arm_never_taken() {
    assert_compile_errors(&[
        ("match nope { _ => 1 }", "`nope` is not defined"),
        ("match 1 { 1 => 0, _ => nope }", "`nope` is not defined"),
        ("match 1 { (nope) => 0, _ => 1 }", "`nope` is not defined"),
        (
            "match 1 { x if: nope => 0, _ => 1 }",
            "`nope` is not defined",
        ),
        (
            "match 1 { {[nope]: x} => 0, _ => 1 }",
            "`nope` is not defined",
        ),
        ("match 1 { _ => 1, 2 => nope }", "`nope` is not defined"),
    ]);
}

#[test]
fn an_arm_may_capture_its_bindings() {
    assert_values(&[
        (
            r"
            def f = match [1, 2] { [a, b] => fn -> a + b }
            f()
            ",
            "3",
        ),
        (
            "transform([[1, 2], [3, 4]], fn p -> match p { [a, b] => a * b })",
            "[2, 12]",
        ),
        ("match 3 { x if: (fn -> x > 2)() => x, _ => 0 }", "3"),
        (
            "transform([1, [2], {v: 3}], $(match $ { [x] | {v: x} => x, x => x }))",
            "[1, 2, 3]",
        ),
        (
            r"
            def fs = transform([[1, 0], [0, 2]], fn p -> match p { [x, 0] | [0, x] => fn -> x })
            [fs[0](), fs[1]()]
            ",
            "[1, 2]",
        ),
    ]);
}

#[test]
fn a_match_may_appear_in_any_part_of_another() {
    assert_values(&[
        (
            r#"match 1 { x if: match x { 1 => true, _ => false } => "one", _ => "other" }"#,
            r#""one""#,
        ),
        (
            r#"match 2 { (match 1 { y => y + 1 }) => "two", _ => 0 }"#,
            r#""two""#,
        ),
        ("match [1, [2]] { [a, b] => match b { [c] => a + c } }", "3"),
        (
            r"
            do {
                def v = [1]
                match v { [x] => x }
            }
            ",
            "1",
        ),
        (
            r"
            (fn v -> {
                def w = [v]
                match w { [x] => x }
            })(4)
            ",
            "4",
        ),
    ]);
}

#[test]
fn a_matchs_bindings_are_not_implicitly_exported() {
    let source = r"
        def r = match [1] { [a] => a }
        r
    ";
    let finished = Script::new(source).implicit_export().finish();
    assert_eq!(finished.exports.keys().collect::<Vec<_>>(), vec!["r"]);

    // A pattern binding shadowing a top-level name leaves that name's export.
    let source = r"
        def x = 1
        def r = match [2, 3] { [x, 1] | [_, x] => x }
        export def e = match {a: 4} { {a} => a }
        [x, r]
    ";
    let finished = Script::new(source).implicit_export().finish();
    assert_eq!(finished.tail, run("[1, 3]"));
    assert_eq!(
        finished.exports.keys().collect::<Vec<_>>(),
        vec!["e", "r", "x"]
    );
    assert_eq!(finished.exports["x"], Value::Int(1));
}

// --- Errors ---

#[test]
fn an_error_inside_a_match_can_be_caught() {
    // Each error leaves the matching machinery mid-arm; the caught function's
    // frame goes with it, and later matches run normally.
    assert_values(&[
        (
            "[try_call(fn -> match 1 { 2 => 0 }).ok, match 1 { 1 => 9 }]",
            "[false, 9]",
        ),
        (
            r#"
            [
                try_call(fn -> match [1] { [x] if: error("boom") => 0 }).ok,
                match [1] { [x] => x }
            ]
            "#,
            "[false, 1]",
        ),
        (
            r"
            [
                try_call(fn -> match [1, 2] { [x, (1 / 0)] => 0 }).ok,
                match [1] { [x] => x }
            ]
            ",
            "[false, 1]",
        ),
        (
            r"
            [
                try_call(fn -> match [1, 2] { [x, 0] | [x, (1 / 0)] => 0 }).ok,
                match [1] { [x] => x }
            ]
            ",
            "[false, 1]",
        ),
        // Caught inside a guard or a value, while this frame's marks are open.
        (
            r#"match [1] { [x] if: try_call(fn -> error("boom")).ok => 0, [x] => x }"#,
            "1",
        ),
        (
            "match [1, 2] { [x, (try_call(fn -> match x { 5 => 0 }).ok)] | [x, 2] => x }",
            "1",
        ),
        (
            r"
            defn f(v) -> [
                match v { [x] | {x} => x, _ => null },
                try_call(fn -> match v { 5 => 0 }).ok,
                match v { _ => 9 }
            ]
            [f([1]), f({x: 2}), f(5), f(6)]
            ",
            "[[1, false, 9], [2, false, 9], [null, true, 9], [null, false, 9]]",
        ),
    ]);
}

#[test]
fn a_caught_error_leaves_no_arm_of_its_frames_behind() {
    // Both matches hold an arm open when the error is caught: the catcher's arm
    // must go on to fail cleanly and fall through to its next arm.
    assert_values(&[
        // The raising match's own arm is open.
        (
            r#"
            match [1] {
                [x] if: try_call(fn -> match 1 { y if: error("boom") => 0 }).ok => 0,
                [x] => x
            }
            "#,
            "1",
        ),
        // An alternative's branch is open inside the raising arm.
        (
            r#"
            match [1] {
                [x] if: try_call(fn -> match [1, 2] { [a, (error("boom"))] | [a, 0] => 0 }).ok => 0,
                [x] => x
            }
            "#,
            "1",
        ),
        // Arms are open in two abandoned frames.
        (
            r#"
            match [1] {
                [x] if: try_call(fn -> match 2 {
                    y if: (fn -> match 3 { z if: error("boom") => 0 })() => 0
                }).ok => 0,
                [x] => x
            }
            "#,
            "1",
        ),
        // Repeated, as any leftover would pile up.
        (
            r#"
            defn f(n) -> if n == 0: "done" else: match [n] {
                [k] if: try_call(fn -> match k { j if: error("boom") => 0 }).ok => 0,
                [k] => f(k - 1)
            }
            f(50)
            "#,
            r#""done""#,
        ),
    ]);
}

// --- Recursion and tail calls ---

#[test]
fn a_recursive_function_may_match_on_its_argument() {
    assert_values(&[
        (
            r"
            defn sum(xs) -> match xs { [] => 0, [h, ...t] => h + sum(t) }
            sum([1, 2, 3, 4])
            ",
            "10",
        ),
        (
            r"
            defn size(t) -> match t {
                {left, right} => 1 + size(left) + size(right),
                _ => 0
            }
            size({left: {left: null, right: null}, right: null})
            ",
            "2",
        ),
        (
            r"
            defn count(xs, n) -> match xs {
                [] => n,
                [[a, 0] | [0, a] | a, ...rest] => count(rest, n + a)
            }
            count([[1, 0], [0, 2], 3], 0)
            ",
            "6",
        ),
    ]);
}

#[test]
fn tail_recursion_through_a_match_runs_in_bounded_depth() {
    for source in [
        r#"
        defn count(n) -> match n { 0 => "done", k => count(k - 1) }
        count(100000)
        "#,
        r#"
        defn count(n) -> match [n, n] {
            [0, _] | [_, 0] => "done",
            [k, _] if: k > 0 => count(k - 1)
        }
        count(100000)
        "#,
        r#"
        defn count(n) -> match {n: n} { {n: 0} => "done", {n} as m => count(m.n - 1) }
        count(100000)
        "#,
    ] {
        let tail = Script::new(source).max_call_depth(100).run();
        assert_eq!(tail, Value::from("done"), "{source}");
    }
}

#[test]
fn only_an_arms_result_is_in_tail_position() {
    // The target, value, and guard calls are ordinary; each result's is a tail call.
    let source = "fn f(n) -> match f(n) { (f(n)) => f(n), x if: f(x) => f(x), _ => f(n) }";
    for optimization in optimization_permutations() {
        let lambda = Script::new(source).code(optimization).nested(0);
        assert_eq!(
            calls_by_kind(&lambda),
            (3, 3),
            "{optimization:?}: {lambda:?}"
        );
    }
    // Nested in something else, no arm's result is.
    let lambda = Script::new("fn f(n) -> [match n { _ => f(n) }]")
        .code(UNOPTIMIZED)
        .nested(0);
    assert_eq!(calls_by_kind(&lambda), (1, 0), "{lambda:?}");
}

// --- What the compiler emits ---

#[test]
fn a_constant_match_folds_to_its_value() {
    for source in [
        "match 3 { 1 => 10, 3 => 30, _ => 0 }",
        "match [1, 2] { [_, 3] => 0, [_, 2] => 30 }",
        "match {a: 1} { {a: 2} => 0, {a: _} => 30 }",
        "match 2 { 1 | 2 => 30, _ => 0 }",
        "match 5 { _ is String => 0, _ is Int => 30 }",
    ] {
        let emitted = Script::new(source).code(FOLD);
        assert_eq!(
            emitted.count(&Bytecode::MarkStack),
            0,
            "{source:?}: {emitted:?}"
        );
        assert_eq!(
            emitted.count(&Bytecode::PushInt(30)),
            1,
            "{source:?}: {emitted:?}"
        );
    }
}

#[test]
fn a_constant_match_that_raises_is_left_for_runtime() {
    let source = "match 1 { 2 => 0 }";
    let emitted = Script::new(source).code(FOLD);
    assert_eq!(emitted.count(&Bytecode::ProduceError), 1, "{emitted:?}");
    assert_raises(&[(source, "No match arm matches the value: 1")]);
    // It raises only if it runs.
    assert_values(&[(
        r"
        def f = fn -> match 1 { 2 => 0 }
        5
        ",
        "5",
    )]);
}

#[test]
fn with_a_runtime_target_each_part_folds_on_its_own() {
    let source = r"
        match x {
            (2 * 3) => 3 * 4,
            {[1 + 1]: v} => 5 * 6,
            _ if: 1 < 2 => 7 * 8
        }
    ";
    let emitted = Script::new(source).capture("x", Value::Int(6)).code(FOLD);
    for op in [Bytecode::Multiply, Bytecode::Add, Bytecode::CompareLessThan] {
        assert_eq!(emitted.count(&op), 0, "no {op:?} is left: {emitted:?}");
    }
    for folded in [6, 12, 30, 56] {
        assert_eq!(
            emitted.count(&Bytecode::PushInt(folded)),
            1,
            "{folded} is folded: {emitted:?}"
        );
    }
    // The folded key is looked up as a constant.
    assert_eq!(
        emitted.key_constants(),
        [MapKey::Int(2), MapKey::Int(2)],
        "{emitted:?}"
    );
    assert_eq!(
        Script::new(source).capture("x", Value::Int(6)).run(),
        Value::Int(12)
    );
}

#[test]
fn a_constant_target_folds_when_its_arms_cannot() {
    // The arm reads its binding, so the match as a whole is left for runtime.
    let source = "match [2 * 3] { [n] => n }";
    let emitted = Script::new(source).code(FOLD);
    assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::MarkStack), 1, "{emitted:?}");
    assert_values(&[(source, "6")]);
}

#[test]
fn without_folding_nothing_is_folded() {
    let emitted = Script::new("match 3 { 1 => 10, 3 => 2 * 15 }").code(UNOPTIMIZED);
    assert_eq!(emitted.count(&Bytecode::MarkStack), 2, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::Multiply), 1, "{emitted:?}");
}

#[test]
fn a_known_key_is_tested_and_looked_up_as_a_constant() {
    // A literal key is known as written, without folding.
    for source in [
        "match x { {a} => a, _ => 0 }",
        "match x { {a: v} => v, _ => 0 }",
        r#"match x { {["a"]: v} => v, _ => 0 }"#,
    ] {
        let emitted = Script::new(source)
            .capture("x", Value::Null)
            .code(UNOPTIMIZED);
        for op in [Bytecode::TestConstKey(0), Bytecode::ExtractConstKey(1)] {
            assert_eq!(emitted.count(&op), 1, "{source:?}: {op:?} in {emitted:?}");
        }
        for op in [Bytecode::TestKey, Bytecode::ExtractKey] {
            assert_eq!(emitted.count(&op), 0, "{source:?}: {op:?} in {emitted:?}");
        }
        assert_eq!(
            emitted.key_constants(),
            [MapKey::from("a"), MapKey::from("a")],
            "{source:?}"
        );
    }
}

#[test]
fn a_key_not_known_as_a_valid_key_is_looked_up_dynamically() {
    // One known only at runtime, and one whose constant is not a valid key.
    for source in [
        "match x { {[x]: v} => v, _ => 0 }",
        "match x { {[null]: v} => v, _ => 0 }",
    ] {
        let emitted = Script::new(source)
            .capture("x", Value::Null)
            .code(UNOPTIMIZED);
        for op in [Bytecode::TestKey, Bytecode::ExtractKey] {
            assert_eq!(emitted.count(&op), 1, "{source:?}: {op:?} in {emitted:?}");
        }
        assert!(
            emitted.key_constants().is_empty(),
            "{source:?}: {emitted:?}"
        );
    }
    // The invalid key still raises, once reached against a Map.
    assert_raises(&[(
        "match {a: 1} { {[null]: v} => v, _ => 0 }",
        "not a valid Map key",
    )]);
    assert_values(&[("match 5 { {[null]: v} => v, _ => 0 }", "0")]);
}
