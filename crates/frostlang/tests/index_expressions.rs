//! Indexing lowering, end to end: compile full source, run it on the VM, and
//! check the result.
//!
//! A soft index `target[key]` works on an Array (by Int, negative from the end)
//! or a Map (by any valid key), yielding `null` for a missing entry. A hard
//! index `target.name` works only on a Map and raises for a missing key.
//!
//! Structures reach the scripts as captures, which are known only at runtime.
//! The harness runs every behavioral case under every optimization permutation.

mod script;

use frostlang::bytecode::Bytecode;
use frostlang::compile::{Optimization, OptimizationOptions};
use frostlang::{Arity, MapKey, Value};
use script::{Emitted, Script, UNOPTIMIZED, raises};

const FOLD: OptimizationOptions = UNOPTIMIZED.with(Optimization::ConstantFold, true);

/// `[10, 20, 30]`
fn array() -> Value {
    Value::array([Value::Int(10), Value::Int(20), Value::Int(30)])
}

/// A Map with String keys, a nested Map, an Array of Maps, and a function.
fn map() -> Value {
    Value::map([
        ("name", Value::from("ada")),
        ("age", Value::Int(36)),
        ("inner", Value::map([("deep", Value::Int(1))])),
        (
            "people",
            Value::array([
                Value::map([("name", Value::from("bo"))]),
                Value::map([("name", Value::from("cy"))]),
            ]),
        ),
        (
            "double",
            Value::native("double", Arity::Exact(1), |_, args| match args[0] {
                Value::Int(n) => Ok(Value::Int(n * 2)),
                _ => Ok(Value::Null),
            }),
        ),
    ])
}

/// A Map keyed by non-String keys.
fn keyed() -> Value {
    Value::map([
        (MapKey::Int(1), Value::from("int")),
        (MapKey::Bool(true), Value::from("bool")),
        (MapKey::Bytes(vec![0u8].into()), Value::from("bytes")),
    ])
}

/// `source` with `a` (an Array), `m` (a Map), and `k` (a non-String-keyed Map)
/// in scope.
fn script(source: &str) -> Script {
    Script::new(source)
        .capture("a", array())
        .capture("m", map())
        .capture("k", keyed())
}

fn run_with_data(source: &str) -> Value {
    script(source).run()
}

fn raises_with_data(source: &str) -> String {
    script(source).raises()
}

fn code(source: &str, optimization: OptimizationOptions) -> Emitted {
    script(source).code(optimization)
}

// --- Soft index: Arrays ---

#[test]
fn an_array_is_indexed_from_zero() {
    assert_eq!(run_with_data("a[0]"), Value::Int(10));
    assert_eq!(run_with_data("a[1]"), Value::Int(20));
    assert_eq!(run_with_data("a[2]"), Value::Int(30));
}

#[test]
fn a_negative_index_counts_from_the_end() {
    assert_eq!(run_with_data("a[-1]"), Value::Int(30));
    assert_eq!(run_with_data("a[-3]"), Value::Int(10));
}

#[test]
fn an_out_of_bounds_array_index_is_null() {
    for source in [
        "a[3]",
        "a[-4]",
        "a[9223372036854775807]",
        // The most negative Int, computed since its literal overflows as a
        // positive token before negation applies.
        "a[-9223372036854775807 - 1]",
    ] {
        assert_eq!(run_with_data(source), Value::Null, "{source:?}");
    }
}

#[test]
fn indexing_an_empty_array_is_always_null() {
    let tail = Script::new("[][0]").run();
    assert_eq!(tail, Value::Null);
    let tail = Script::new("[][-1]").run();
    assert_eq!(tail, Value::Null);
}

#[test]
fn an_array_index_must_be_an_int() {
    for (source, type_name) in [
        (r#"a["0"]"#, "String"),
        ("a[0.0]", "Float"),
        ("a[null]", "Null"),
    ] {
        let message = raises_with_data(source);
        assert!(
            message.contains("Cannot index Array") && message.contains(type_name),
            "{source:?}: {message}"
        );
    }
}

// --- Soft index: Maps ---

#[test]
fn a_map_is_indexed_by_key() {
    assert_eq!(run_with_data(r#"m["name"]"#), Value::from("ada"));
    assert_eq!(run_with_data(r#"m["age"]"#), Value::Int(36));
}

#[test]
fn a_missing_map_key_is_null() {
    assert_eq!(run_with_data(r#"m["nope"]"#), Value::Null);
    assert_eq!(
        run_with_data("m[1]"),
        Value::Null,
        "a valid key type, absent"
    );
}

#[test]
fn indexing_an_empty_map_is_always_null() {
    assert_eq!(run_with_data(r#"{}["anything"]"#), Value::Null);
}

#[test]
fn a_map_is_indexed_by_any_valid_key_type() {
    assert_eq!(run_with_data("k[1]"), Value::from("int"));
    assert_eq!(run_with_data("k[true]"), Value::from("bool"));
    assert_eq!(run_with_data("k[x'00']"), Value::from("bytes"));
}

#[test]
fn map_keys_do_not_cross_numeric_types() {
    assert_eq!(
        run_with_data("k[1.0]"),
        Value::Null,
        "Float 1.0 is not Int 1"
    );
}

#[test]
fn an_invalid_map_key_type_raises() {
    for (source, type_name) in [("m[null]", "Null"), ("m[m]", "Map"), ("m[a]", "Array")] {
        let message = raises_with_data(source);
        assert!(
            message.contains("not a valid Map key") && message.contains(type_name),
            "{source:?}: {message}"
        );
    }
}

// --- Soft index: other targets ---

#[test]
fn soft_indexing_a_non_structure_raises() {
    for (source, type_name) in [
        ("5[0]", "Int"),
        (r#""abc"[0]"#, "String"),
        ("null[0]", "Null"),
        ("true[0]", "Bool"),
    ] {
        let message = raises(source);
        assert!(
            message.contains("Cannot index value") && message.contains(type_name),
            "{source:?}: {message}"
        );
    }
}

// --- Hard index ---

#[test]
fn a_map_field_is_read_by_name() {
    assert_eq!(run_with_data("m.name"), Value::from("ada"));
    assert_eq!(run_with_data("m.age"), Value::Int(36));
}

#[test]
fn a_hard_index_reads_the_same_entry_as_a_soft_one() {
    assert_eq!(run_with_data(r#"m.name == m["name"]"#), Value::Bool(true));
}

#[test]
fn a_missing_field_raises() {
    let message = raises_with_data("m.nope");
    assert!(
        message.contains("no value at key") && message.contains("nope"),
        "{message}"
    );
}

#[test]
fn a_mistyped_field_suggests_the_intended_one() {
    for (source, suggested) in [
        ("m.nmae", "name"),
        ("m.inner.dep", "deep"),
        ("m.people[0].nam", "name"),
    ] {
        let message = raises_with_data(source);
        assert!(
            message.contains(&format!("did you mean '{suggested}'?")),
            "{source:?}: {message}"
        );
    }
    let message = raises_with_data("m.nope");
    assert!(!message.contains("did you mean"), "{message}");
}

#[test]
fn a_hard_index_only_indexes_a_map() {
    for (source, type_name) in [("a.x", "Array"), ("m.age.x", "Int"), ("null.x", "Null")] {
        let message = raises_with_data(source);
        assert!(
            message.contains("Cannot index value") && message.contains(type_name),
            "{source:?}: {message}"
        );
    }
}

#[test]
fn a_field_may_be_called() {
    assert_eq!(run_with_data("m.double(21)"), Value::Int(42));
}

// --- Composition ---

#[test]
fn indexes_chain() {
    assert_eq!(run_with_data("m.inner.deep"), Value::Int(1));
    assert_eq!(run_with_data(r#"m["inner"]["deep"]"#), Value::Int(1));
    assert_eq!(run_with_data(r#"m.inner["deep"]"#), Value::Int(1));
    assert_eq!(run_with_data("m.people[1].name"), Value::from("cy"));
    assert_eq!(run_with_data("m.people[-1].name"), Value::from("cy"));
}

#[test]
fn a_soft_index_absorbs_a_missing_link_only_at_its_own_step() {
    // The missing `m["nope"]` is null; hard-indexing that null raises.
    assert_eq!(run_with_data(r#"m["nope"]"#), Value::Null);
    let message = raises_with_data(r#"m["nope"].x"#);
    assert!(message.contains("Null"), "{message}");
}

#[test]
fn targets_and_keys_are_expressions() {
    assert_eq!(run_with_data("a[1 + 1]"), Value::Int(30));
    assert_eq!(run_with_data("a[0 - 1]"), Value::Int(30));
    assert_eq!(run_with_data("(if true: a else: m)[0]"), Value::Int(10));
    let block_target = r"
        do {
            def t = m
            t
        }.name
    ";
    assert_eq!(run_with_data(block_target), Value::from("ada"));
    assert_eq!(
        run_with_data(r#"a[if m["age"] > 30: 2 else: 0]"#),
        Value::Int(30)
    );
}

#[test]
fn the_target_may_be_a_call_result() {
    let source = r"
        def get_a = fn -> a
        get_a()[1]
    ";
    let tail = Script::new(source).capture("a", array()).run();
    assert_eq!(tail, Value::Int(20));
}

#[test]
fn an_index_is_an_operand() {
    assert_eq!(run_with_data("a[0] + a[1]"), Value::Int(30));
    assert_eq!(run_with_data("m.age * 2"), Value::Int(72));
    assert_eq!(run_with_data(r#"m["nope"] or 5"#), Value::Int(5));
}

// --- Evaluation ---

#[test]
fn the_target_is_evaluated_before_the_key() {
    let message = raises("(1 / 0)[1 % 0]");
    assert!(message.contains("Division by zero"), "{message}");
}

#[test]
fn both_operands_are_evaluated_before_indexing() {
    // The key raises before the non-structure target is rejected.
    let message = raises("5[1 / 0]");
    assert!(message.contains("Division by zero"), "{message}");
}

#[test]
fn a_hard_index_target_error_surfaces() {
    let message = raises("(1 / 0).x");
    assert!(message.contains("Division by zero"), "{message}");
}

#[test]
fn an_index_statement_leaves_the_stack_balanced() {
    let source = r"
        m.name
        a[0]
        m.inner.deep
        5
    ";
    assert_eq!(run_with_data(source), Value::Int(5));
}

// --- Constant folding ---

#[test]
fn a_constant_key_of_a_runtime_target_folds() {
    let emitted = code("a[1 + 1]", FOLD);
    assert_eq!(emitted.count(&Bytecode::Add), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(2)), 1, "{emitted:?}");
    assert_eq!(
        emitted.count(&Bytecode::SoftIndexStructure),
        1,
        "the runtime index stays: {emitted:?}"
    );

    let emitted = code("a[-1]", FOLD);
    assert_eq!(emitted.count(&Bytecode::Negate), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(-1)), 1, "{emitted:?}");
}

#[test]
fn a_constant_key_of_any_valid_type_folds() {
    // Each key expression folds down to its value, whatever its type; the
    // SoftIndexStructure itself stays, since the target (`m`) is runtime-only.
    for source in [
        r#"m["a" + "ge"]"#,
        "m[true == true]",
        "m[1 + 0]",
        "m[1.5 + 0.0]",
    ] {
        let emitted = code(source, FOLD);
        assert_eq!(
            emitted.count(&Bytecode::SoftIndexStructure),
            1,
            "the runtime index stays: {source:?}: {emitted:?}"
        );
        assert_eq!(emitted.count(&Bytecode::Add), 0, "{source:?}: {emitted:?}");
        assert_eq!(
            emitted.count(&Bytecode::CompareEqual),
            0,
            "{source:?}: {emitted:?}"
        );
    }
}

#[test]
fn a_raising_constant_index_is_left_for_runtime() {
    let emitted = code(r#""abc"[0]"#, FOLD);
    assert_eq!(
        emitted.count(&Bytecode::SoftIndexStructure),
        1,
        "{emitted:?}"
    );

    let emitted = code("5.x", FOLD);
    assert_eq!(hard_indexes(&emitted), 1, "{emitted:?}");

    let emitted = code("{a: 1}.b", FOLD);
    assert_eq!(hard_indexes(&emitted), 1, "a missing field: {emitted:?}");
}

fn hard_indexes(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
        .filter(|op| matches!(op, Bytecode::HardIndexMap(_)))
        .count()
}

#[test]
fn an_index_into_a_constant_structure_folds() {
    for (source, value) in [
        ("[10, 20, 30][1]", Bytecode::PushInt(20)),
        ("[10, 20, 30][-1]", Bytecode::PushInt(30)),
        ("[10][5]", Bytecode::PushNull),
        (r#"{a: 5}["a"]"#, Bytecode::PushInt(5)),
        (r#"{a: 5}["b"]"#, Bytecode::PushNull),
    ] {
        let emitted = code(source, FOLD);
        assert_eq!(
            emitted.count(&Bytecode::SoftIndexStructure),
            0,
            "{source:?}: {emitted:?}"
        );
        assert_eq!(emitted.count(&value), 1, "{source:?}: {emitted:?}");
    }
}

#[test]
fn a_field_of_a_constant_map_folds() {
    let emitted = code("{a: {b: 5}}.a.b", FOLD);
    assert_eq!(hard_indexes(&emitted), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");
}

#[test]
fn a_propagated_array_constant_folds_through_indexing() {
    // An Array bound once and read by constant index: both the binding and the
    // index fold away entirely.
    let fold_and_propagate = FOLD.with(Optimization::ConstantPropagate, true);
    let source = r"
        def xs = [10, 20, 30]
        xs[0] + xs[2]
    ";
    let emitted = code(source, fold_and_propagate);
    assert_eq!(emitted.count(&Bytecode::PushInt(40)), 1, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::Add), 0, "{emitted:?}");
    assert_eq!(
        emitted.count(&Bytecode::SoftIndexStructure),
        0,
        "{emitted:?}"
    );
}
