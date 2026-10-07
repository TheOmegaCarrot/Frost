//! Array and Map literal lowering, end to end: compile full source, run it on
//! the VM, and check the result.
//!
//! Elements, and each Map entry's key then value, are evaluated left to right,
//! then the structure is built. A Map key must be a valid key type (Bool, Int,
//! Float, String, or Bytes); a `name:` key is shorthand for the String key
//! `"name"`. The harness runs every behavioral case under every optimization
//! permutation.

mod script;

use frostlang::Value;
use frostlang::bytecode::Bytecode;
use frostlang::compile::{Optimization, OptimizationOptions};
use script::{Emitted, Script, UNOPTIMIZED, raises, run};

const FOLD: OptimizationOptions = UNOPTIMIZED.with(Optimization::ConstantFold, true);

fn ints(values: &[i64]) -> Value {
    Value::from_iter(values.iter().copied().map(Value::Int))
}

/// The code of `source` under exactly `optimization`, with `x` a runtime-only
/// Int.
fn code(source: &str, optimization: OptimizationOptions) -> Emitted {
    Script::new(source)
        .capture("x", Value::Int(7))
        .code(optimization)
}

fn loads_a_constant(emitted: &Emitted) -> bool {
    emitted
        .code
        .iter()
        .any(|op| matches!(op, Bytecode::LoadConst(_)))
}

fn builds(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
        .filter(|op| matches!(op, Bytecode::MakeArray(_) | Bytecode::MakeMap(_)))
        .count()
}

// --- Arrays ---

#[test]
fn an_array_holds_its_elements_in_order() {
    assert_eq!(run("[]"), Value::from(Vec::<Value>::new()));
    assert_eq!(run("[1]"), ints(&[1]));
    assert_eq!(run("[1, 2, 3]"), ints(&[1, 2, 3]));
    assert_eq!(run("[1, 2, 3,]"), ints(&[1, 2, 3]), "a trailing comma");
}

#[test]
fn array_elements_may_be_any_value() {
    assert_eq!(
        run(r#"[null, true, 1, 2.5, "s", x'00']"#),
        Value::from_iter([
            Value::Null,
            Value::Bool(true),
            Value::Int(1),
            Value::try_from(2.5).unwrap(),
            Value::from("s"),
            Value::from(vec![0u8]),
        ])
    );
    assert_eq!(
        run("[[1], [2, 3]]"),
        Value::from_iter([ints(&[1]), ints(&[2, 3])])
    );
    assert_eq!(run("[plus][0](1, 2)"), Value::Int(3), "a function element");
}

#[test]
fn array_elements_are_expressions() {
    let source = r"
        [x, x + 1, if x > 5: 10 else: 0, do {
            def y = 2
            y
        }]
    ";
    let tail = Script::new(source).capture("x", Value::Int(7)).run();
    assert_eq!(tail, ints(&[7, 8, 10, 2]));
}

#[test]
fn an_array_literal_is_an_operand() {
    assert_eq!(run("[10, 20, 30][1]"), Value::Int(20));
    assert_eq!(run("[1, 2] + [3]"), ints(&[1, 2, 3]));
    assert_eq!(run("[1, 2] == [1, 2]"), Value::Bool(true));
    assert_eq!(run("[1, 2] == [2, 1]"), Value::Bool(false));
}

#[test]
fn array_elements_are_evaluated_left_to_right() {
    let message = raises("[1, 1 / 0, 1 % 0]");
    assert!(message.contains("Division by zero"), "{message}");
}

#[test]
fn a_large_array_literal_builds_correctly() {
    let elements: Vec<i64> = (0..200).collect();
    let source = format!(
        "[{}]",
        elements
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert_eq!(run(&source), ints(&elements));
}

// --- Maps ---

#[test]
fn a_map_holds_its_entries() {
    assert_eq!(run("{}"), Value::map::<&str, 0>([]));
    assert_eq!(
        run(r#"{name: "ada", age: 36}"#),
        Value::map([("name", Value::from("ada")), ("age", Value::Int(36))])
    );
    assert_eq!(
        run("{a: 1,}"),
        Value::map([("a", Value::Int(1))]),
        "a trailing comma"
    );
}

#[test]
fn a_large_map_literal_builds_correctly() {
    let entries: Vec<(i64, i64)> = (0..100).map(|n| (n, n * 2)).collect();
    let source = format!(
        "{{{}}}",
        entries
            .iter()
            .map(|(k, v)| format!("[{k}]: {v}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    for (k, v) in &entries {
        assert_eq!(run(&format!("{source}[{k}]")), Value::Int(*v), "key {k}");
    }
}

#[test]
fn a_shorthand_key_is_a_string_key() {
    assert_eq!(run(r#"{foo: 1}["foo"]"#), Value::Int(1));
    assert_eq!(run("{foo: 1}.foo"), Value::Int(1));
}

#[test]
fn a_computed_key_may_be_any_valid_key_type() {
    let source = r#"def m = {[1]: "int", [true]: "bool", [1.5]: "float", ["s"]: "string", [x'00']: "bytes"}"#;
    for (lookup, expected) in [
        ("m[1]", "int"),
        ("m[true]", "bool"),
        ("m[1.5]", "float"),
        (r#"m["s"]"#, "string"),
        ("m[x'00']", "bytes"),
    ] {
        let program = format!(
            r"
            {source}
            {lookup}
            "
        );
        assert_eq!(run(&program), Value::from(expected), "{lookup}");
    }
}

#[test]
fn an_int_key_and_a_float_key_are_distinct() {
    let source = r#"def m = {[1]: "int", [1.0]: "float"}"#;
    for (lookup, expected) in [("m[1]", "int"), ("m[1.0]", "float")] {
        let program = format!(
            r"
            {source}
            {lookup}
            "
        );
        assert_eq!(run(&program), Value::from(expected), "{lookup}");
    }
}

#[test]
fn keys_and_values_are_expressions() {
    assert_eq!(run(r#"{[1 + 2]: "three"}[3]"#), Value::from("three"));
    let tail = Script::new("{[x]: x * 2}[7]")
        .capture("x", Value::Int(7))
        .run();
    assert_eq!(tail, Value::Int(14));
}

#[test]
fn an_invalid_key_type_raises() {
    for (source, type_name) in [
        ("{[null]: 1}", "Null"),
        ("{[[1]]: 1}", "Array"),
        ("{[{}]: 1}", "Map"),
        ("{[plus]: 1}", "Function"),
    ] {
        let message = raises(source);
        assert!(
            message.contains("not a valid Map key") && message.contains(type_name),
            "{source:?}: {message}"
        );
    }
}

#[test]
fn maps_nest_with_arrays() {
    assert_eq!(run("{outer: {inner: 42}}.outer.inner"), Value::Int(42));
    assert_eq!(run("{items: [1, 2, 3]}.items[-1]"), Value::Int(3));
    assert_eq!(
        run(r#"[{name: "bo"}, {name: "cy"}][1].name"#),
        Value::from("cy")
    );
}

#[test]
fn a_map_literal_is_an_operand() {
    assert_eq!(
        run("{a: 1, b: 2} == {b: 2, a: 1}"),
        Value::Bool(true),
        "entry order does not matter"
    );
    assert_eq!(run("({a: 1} + {b: 2}).b"), Value::Int(2));
    assert_eq!(run("({a: 1} + {a: 2}).a"), Value::Int(2));
}

#[test]
fn a_key_is_evaluated_before_its_value() {
    let message = raises("{[1 / 0]: 1 % 0}");
    assert!(message.contains("Division by zero"), "{message}");
}

#[test]
fn entries_are_evaluated_in_order() {
    let message = raises("{a: 1 % 0, b: 1 / 0}");
    assert!(message.contains("Modulus by zero"), "{message}");
}

#[test]
fn a_repeated_key_keeps_its_last_value() {
    // As in Lua, a later entry overwrites an earlier one
    // with the same key, however either key is written.
    for (source, expected) in [
        ("{a: 42, a: 10}", "{a: 10}"),
        (r#"{a: 42, ["a"]: 10}"#, "{a: 10}"),
        (r#"{["a"]: 42, a: 10}"#, "{a: 10}"),
        ("{a: 42, b: 0, a: 10}", "{a: 10, b: 0}"),
        ("{[1]: 42, [1]: 10}", "{[1]: 10}"),
        ("{[true]: 42, [true]: 10}", "{[true]: 10}"),
        ("{[x'00']: 42, [x'00']: 10}", "{[x'00']: 10}"),
    ] {
        assert_eq!(
            run(&format!("{source} == {expected}")),
            Value::Bool(true),
            "{source:?} is {expected:?}"
        );
    }
    let tail = Script::new("{[x]: 42, [7]: 10}[7]")
        .capture("x", Value::Int(7))
        .run();
    assert_eq!(
        tail,
        Value::Int(10),
        "a runtime key repeated by a literal one"
    );
}

#[test]
fn an_overwritten_value_is_still_evaluated() {
    let message = raises("{a: 1 / 0, a: 10}");
    assert!(message.contains("Division by zero"), "{message}");
}

#[test]
fn a_literal_statement_leaves_the_stack_balanced() {
    let source = r"
        [1, 2]
        {a: 1}
        []
        {}
        5
    ";
    assert_eq!(run(source), Value::Int(5));
}

// --- Constant folding ---

#[test]
fn a_constant_array_folds_to_one_constant() {
    for source in ["[1, 2, 3]", "[1, 2 + 3]", "[[1, 2], [3]]"] {
        let emitted = code(source, FOLD);
        assert_eq!(
            builds(&emitted),
            0,
            "{source:?}: built at compile time: {emitted:?}"
        );
        assert!(loads_a_constant(&emitted), "{source:?}: {emitted:?}");
    }
    let emitted = code("[1, 2, 3]", UNOPTIMIZED);
    assert_eq!(
        builds(&emitted),
        1,
        "unfolded, the Array is built at runtime: {emitted:?}"
    );
}

#[test]
fn a_constant_map_folds_to_one_constant() {
    for source in [
        r#"{name: "ada", age: 36}"#,
        "{a: 1 + 2, [4 * 5]: true}",
        "{opts: {deep: [1, 2]}}",
    ] {
        let emitted = code(source, FOLD);
        assert_eq!(
            builds(&emitted),
            0,
            "{source:?}: built at compile time: {emitted:?}"
        );
        assert!(loads_a_constant(&emitted), "{source:?}: {emitted:?}");
    }
}

#[test]
fn a_runtime_element_keeps_the_build_and_folds_the_rest() {
    let emitted = code("[1, x, 2 + 3]", FOLD);
    assert_eq!(builds(&emitted), 1, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::Add), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");

    let emitted = code("{a: x, b: 2 * 3}", FOLD);
    assert_eq!(builds(&emitted), 1, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");

    let emitted = code("{[x]: 1}", FOLD);
    assert_eq!(builds(&emitted), 1, "a runtime key: {emitted:?}");
}

#[test]
fn a_constant_substructure_folds_inside_a_runtime_one() {
    // Only the outer structure is built at runtime; the inner one is a constant.
    for source in ["[x, [1, 2]]", "{a: x, b: {c: 1}}", "[x, {c: [1]}]"] {
        let emitted = code(source, FOLD);
        assert_eq!(builds(&emitted), 1, "{source:?}: {emitted:?}");
        assert!(loads_a_constant(&emitted), "{source:?}: {emitted:?}");
    }
}

#[test]
fn a_structure_holding_a_function_is_built_at_runtime() {
    // A function cannot be a constant, so neither can a structure holding one.
    for source in ["[plus]", "{f: plus}", "[1, [plus]]"] {
        let emitted = code(source, FOLD);
        assert!(builds(&emitted) >= 1, "{source:?}: {emitted:?}");
    }
}

#[test]
fn a_raising_literal_is_left_for_runtime() {
    let emitted = code("{[null]: 1}", FOLD);
    assert_eq!(builds(&emitted), 1, "{emitted:?}");

    // A raising literal nested inside an outer one aborts the outer fold too:
    // both structures are built at runtime, unfolded.
    let emitted = code("[1, {[null]: 1}]", FOLD);
    assert_eq!(
        builds(&emitted),
        2,
        "the outer Array, unfolded: {emitted:?}"
    );
}

#[test]
fn a_propagated_options_map_folds_through_field_access() {
    // A Map used as keyword arguments: bound once, read by field.
    let fold_and_propagate = FOLD.with(Optimization::ConstantPropagate, true);
    let emitted = code(
        r"
        def opts = {width: 80, height: 24}
        opts.width * opts.height
        ",
        fold_and_propagate,
    );
    assert_eq!(emitted.count(&Bytecode::PushInt(1920)), 1, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
    let field_reads = emitted
        .code
        .iter()
        .filter(|op| matches!(op, Bytecode::HardIndexMap(_)))
        .count();
    assert_eq!(field_reads, 0, "{emitted:?}");
}
