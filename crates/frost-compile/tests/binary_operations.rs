//! Binary operator lowering, end to end: compile full source, run it on the VM,
//! and check the result.
//!
//! The operators' own semantics are the runtime's, and tested there; these tests
//! pin that each operator lowers to the right operation with its operands in the
//! right order, and that optimization never changes a result (the harness runs
//! every behavioral case under every optimization permutation).
//!
//! The folding tests inspect the emitted code, since folding's observable effect
//! is exactly which operations are left for runtime. A runtime-only operand is a
//! capture, which no optimization can see through.

mod common;

use common::{Emitted, Script, raises, run};
use frost_runtime::{Bytecode, MapKey, Value};

fn float(f: f64) -> Value {
    Value::try_from(f).expect("test floats are finite")
}

/// The code of `source`, with `x` a runtime-only capture, under every
/// optimization permutation with constant folding on.
fn folded(source: &str) -> Vec<Emitted> {
    Script::new(source)
        .capture("x", Value::Int(1))
        .code_where(|optimization| optimization.constant_fold)
}

// --- Arithmetic ---

#[test]
fn add() {
    assert_eq!(run("2 + 3"), Value::Int(5));
    assert_eq!(run("1.5 + 2.25"), float(3.75));
    assert_eq!(run("1 + 0.5"), float(1.5), "Int + Float is a Float");
    assert_eq!(run("0.5 + 1"), float(1.5), "Float + Int is a Float");
    assert_eq!(
        run(r#""ab" + "cd""#),
        Value::from("abcd"),
        "operand order kept"
    );
    assert_eq!(
        run("x'01' + x'02'"),
        Value::from(vec![0x01u8, 0x02]),
        "operand order kept"
    );
    assert_eq!(
        run("[1, 2] + [3, 4]"),
        Value::from(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4)
        ]),
        "Array + Array concatenates, left elements first"
    );
    assert_eq!(
        run("{a: 1} + {a: 2}"),
        [(MapKey::from("a"), Value::Int(2))]
            .into_iter()
            .collect::<Value>(),
        "Map + Map merges, and the right side wins a key collision"
    );
}

#[test]
fn subtract() {
    assert_eq!(run("10 - 3"), Value::Int(7), "operand order kept");
    assert_eq!(run("3 - 10"), Value::Int(-7), "operand order kept");
    assert_eq!(run("2.5 - 1"), float(1.5));
    assert_eq!(run("1 - 2.5"), float(-1.5));
}

#[test]
fn multiply() {
    assert_eq!(run("6 * 7"), Value::Int(42));
    assert_eq!(run("1.5 * 2"), float(3.0));
    assert_eq!(run("2 * 1.5"), float(3.0));
}

#[test]
fn divide() {
    assert_eq!(run("10 / 4"), Value::Int(2), "Int division truncates");
    assert_eq!(run("4 / 10"), Value::Int(0), "operand order kept");
    assert_eq!(
        run("(0 - 7) / 2"),
        Value::Int(-3),
        "truncation is toward zero"
    );
    assert_eq!(run("7 / 2.0"), float(3.5));
    assert_eq!(run("7.0 / 2"), float(3.5));
}

#[test]
fn modulus() {
    assert_eq!(run("10 % 4"), Value::Int(2));
    assert_eq!(run("4 % 10"), Value::Int(4), "operand order kept");
}

// --- Equality ---

#[test]
fn equal() {
    assert_eq!(run("3 == 3"), Value::Bool(true));
    assert_eq!(run("3 == 4"), Value::Bool(false));
    assert_eq!(run(r#""a" == "a""#), Value::Bool(true));
    assert_eq!(run("null == null"), Value::Bool(true));
    assert_eq!(
        run("3 == 3.0"),
        Value::Bool(false),
        "no cross-type numeric equality"
    );
    assert_eq!(
        run("1 == true"),
        Value::Bool(false),
        "mixed types are unequal"
    );
}

#[test]
fn not_equal() {
    assert_eq!(run("3 != 4"), Value::Bool(true));
    assert_eq!(run("3 != 3"), Value::Bool(false));
    assert_eq!(
        run("3 != 3.0"),
        Value::Bool(true),
        "no cross-type numeric equality"
    );
}

#[test]
fn a_global_equals_itself() {
    // Equality on functions is identity; a global is one fixed value.
    assert_eq!(run("len == len"), Value::Bool(true));
    assert_eq!(run("len == print"), Value::Bool(false));
}

// --- Ordering ---

#[test]
fn less_than() {
    assert_eq!(run("1 < 2"), Value::Bool(true));
    assert_eq!(run("2 < 1"), Value::Bool(false), "operand order kept");
    assert_eq!(run("1 < 1"), Value::Bool(false));
    assert_eq!(
        run("1 < 1.5"),
        Value::Bool(true),
        "ordering spans Int and Float"
    );
    assert_eq!(run(r#""a" < "b""#), Value::Bool(true));
}

#[test]
fn less_than_or_equal() {
    assert_eq!(run("1 <= 2"), Value::Bool(true));
    assert_eq!(run("1 <= 1"), Value::Bool(true));
    assert_eq!(run("2 <= 1"), Value::Bool(false), "operand order kept");
    assert_eq!(
        run("2 <= 2.0"),
        Value::Bool(true),
        "ordering spans Int and Float"
    );
}

#[test]
fn greater_than() {
    assert_eq!(run("2 > 1"), Value::Bool(true));
    assert_eq!(run("1 > 2"), Value::Bool(false), "operand order kept");
    assert_eq!(run("1 > 1"), Value::Bool(false));
    assert_eq!(
        run("1.5 > 1"),
        Value::Bool(true),
        "ordering spans Int and Float"
    );
}

#[test]
fn greater_than_or_equal() {
    assert_eq!(run("2 >= 1"), Value::Bool(true));
    assert_eq!(run("1 >= 1"), Value::Bool(true));
    assert_eq!(run("1 >= 2"), Value::Bool(false), "operand order kept");
    assert_eq!(
        run("2.0 >= 2"),
        Value::Bool(true),
        "ordering spans Int and Float"
    );
}

// --- Nesting and operands ---

#[test]
fn nested_operations_follow_the_tree() {
    assert_eq!(run("1 + 2 * 3"), Value::Int(7));
    assert_eq!(run("(1 + 2) * 3"), Value::Int(9));
    assert_eq!(run("10 - 3 - 2"), Value::Int(5), "left associative");
    assert_eq!(run("10 - (3 - 2)"), Value::Int(9));
    assert_eq!(run("1 + 2 < 2 * 2"), Value::Bool(true));
}

#[test]
fn operands_may_be_bindings() {
    assert_eq!(run("def x = 4; def y = 3; x - y"), Value::Int(1));
    assert_eq!(run("def x = 4; x * x"), Value::Int(16));
    assert_eq!(
        run("def x = 2 + 3; def y = x * 2; y - x"),
        Value::Int(5),
        "bindings of computed values"
    );
}

// --- Runtime errors ---

#[test]
fn a_type_error_names_the_operand_types_in_order() {
    let message = raises(r#"1 + "a""#);
    assert!(
        message.contains("Int + String"),
        "operand types, left first: {message}"
    );
    let message = raises(r#""a" - 1"#);
    assert!(
        message.contains("String - Int"),
        "operand types, left first: {message}"
    );
    let message = raises("1.5 % 2");
    assert!(
        message.contains("Float % Int"),
        "modulus names its operand types too: {message}"
    );
    let message = raises("[1] + {}");
    assert!(
        message.contains("Array + Map"),
        "structured operands are named like any other: {message}"
    );
}

#[test]
fn division_and_modulus_by_zero_raise() {
    assert!(raises("1 / 0").contains("Division by zero"));
    assert!(raises("1.0 / 0.0").contains("Division by zero"));
    assert!(
        raises("1 / 0.0").contains("Division by zero"),
        "Int / Float(0.0)"
    );
    assert!(raises("1 % 0").contains("Modulus by zero"));
}

#[test]
fn an_unorderable_comparison_raises() {
    let message = raises("true < false");
    assert!(message.contains("not orderable"), "{message}");
    let message = raises("{} < {}");
    assert!(
        message.contains("not orderable"),
        "Map has no ordering either: {message}"
    );
    let message = raises(r#"1 < "a""#);
    assert!(message.contains("incompatible"), "{message}");
}

#[test]
fn ordering_extends_to_bytes_and_arrays() {
    // Comparison is not just Int/Float/String; the compiler routes every
    // operand type through the same opcode.
    assert_eq!(run("x'01' < x'02'"), Value::Bool(true));
    assert_eq!(run("[1, 2] < [1, 3]"), Value::Bool(true));
    assert_eq!(
        run("[1] < [1, 2]"),
        Value::Bool(true),
        "a prefix is less than its extension"
    );
}

#[test]
fn the_left_operand_is_evaluated_first() {
    // Both operands raise, so the error reveals which ran first.
    let message = raises("(1 / 0) + (1 % 0)");
    assert!(
        message.contains("Division by zero"),
        "the left operand's error wins: {message}"
    );
}

// --- Constant folding ---

#[test]
fn a_constant_operation_folds_to_its_value() {
    for emitted in folded("2 + 3") {
        assert_eq!(emitted.count(&Bytecode::Add), 0, "folded away: {emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");
    }
    for emitted in Script::new("2 + 3").code_where(|optimization| !optimization.constant_fold) {
        assert_eq!(
            emitted.count(&Bytecode::Add),
            1,
            "with folding off the Add stays: {emitted:?}"
        );
    }
}

#[test]
fn a_nested_constant_operation_folds_whole() {
    for emitted in folded("(1 + 2) * (3 + 4)") {
        assert_eq!(emitted.count(&Bytecode::Add), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(21)), 1, "{emitted:?}");
    }
}

#[test]
fn a_structured_result_folds_to_a_constant() {
    for source in [r#""ab" + "cd""#, "[1, 2] + [3, 4]", "{a: 1} + {b: 2}"] {
        for emitted in folded(source) {
            assert_eq!(
                emitted.count(&Bytecode::Add),
                0,
                "folded away: {source:?}: {emitted:?}"
            );
            assert!(
                emitted
                    .code
                    .iter()
                    .any(|op| matches!(op, Bytecode::LoadConst(_))),
                "the result is loaded from the pool: {source:?}: {emitted:?}"
            );
        }
    }
}

#[test]
fn the_constant_side_of_a_mixed_operation_folds() {
    // `x` is known only at runtime, so the outer Add stays; its constant
    // operand still folds, whichever side it is on.
    for source in ["x + (2 * 3)", "(2 * 3) + x"] {
        for emitted in folded(source) {
            assert_eq!(
                emitted.count(&Bytecode::Add),
                1,
                "the Add stays: {emitted:?}"
            );
            assert_eq!(
                emitted.count(&Bytecode::Multiply),
                0,
                "the constant operand is folded: {emitted:?}"
            );
            assert_eq!(emitted.count(&Bytecode::PushInt(6)), 1, "{emitted:?}");
        }
    }
}

#[test]
fn the_largest_constant_subtree_folds_as_one() {
    // Every constant operation below the runtime-only `x` folds into one value.
    for emitted in folded("((1 + 2) + 3) + x") {
        assert_eq!(
            emitted.count(&Bytecode::Add),
            1,
            "only the Add over `x` stays: {emitted:?}"
        );
        assert_eq!(emitted.count(&Bytecode::PushInt(6)), 1, "{emitted:?}");
    }
}

#[test]
fn a_propagated_binding_folds() {
    let emitted = Script::new("def x = 2; x * 3")
        .code_where(|optimization| optimization.constant_fold && optimization.constant_propagate);
    for emitted in emitted {
        assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(6)), 1, "{emitted:?}");
    }
}

#[test]
fn a_computed_binding_propagates_its_folded_value() {
    // `x` binds a folded constant, so propagation makes `x + 1` foldable too.
    let emitted = Script::new("def x = 2 * 3; x + 1")
        .code_where(|optimization| optimization.constant_fold && optimization.constant_propagate);
    for emitted in emitted {
        assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::Add), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(7)), 1, "{emitted:?}");
    }
}

#[test]
fn an_operation_over_a_pure_global_folds() {
    for emitted in folded("len == len") {
        assert_eq!(emitted.count(&Bytecode::CompareEqual), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushTrue), 1, "{emitted:?}");
    }
}

#[test]
fn an_operation_over_an_impure_global_does_not_fold() {
    for emitted in folded("print == print") {
        assert_eq!(emitted.count(&Bytecode::CompareEqual), 1, "{emitted:?}");
    }
}

#[test]
fn a_failing_fold_is_left_for_runtime() {
    // The fold raises, so the operation is kept and raises only when run.
    for (source, op) in [
        ("1 / 0", Bytecode::Divide),
        ("1 % 0", Bytecode::Modulus),
        (r#"1 + "a""#, Bytecode::Add),
        ("len + 1", Bytecode::Add),
        ("true < false", Bytecode::CompareLessThan),
    ] {
        for emitted in folded(source) {
            assert_eq!(
                emitted.count(&op),
                1,
                "the failing operation is kept: {emitted:?}"
            );
        }
    }
}

#[test]
fn a_failing_constant_operand_is_left_for_runtime() {
    for emitted in folded("x + (1 / 0)") {
        assert_eq!(emitted.count(&Bytecode::Divide), 1, "{emitted:?}");
    }
    let message = Script::new("x + (1 / 0)")
        .capture("x", Value::Int(1))
        .raises();
    assert!(message.contains("Division by zero"), "{message}");
}
