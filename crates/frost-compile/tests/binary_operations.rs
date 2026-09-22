//! Binary operator lowering, end to end: compile full source, run it on the VM,
//! and check the result.
//!
//! The operators' own semantics are the runtime's, and tested there; these tests
//! pin that each operator lowers to the right operation with its operands in the
//! right order, and that optimization never changes a result. Every behavioral
//! case therefore runs under each combination of optimization options and
//! requires them all to agree.
//!
//! The folding tests inspect the emitted code, since folding's observable effect
//! is exactly which operations are left for runtime.

use frost_compile::{CompilerOptions, OptimizationOptions, compile_program};
use frost_runtime::{Bytecode, Value, Vm};

/// Every `(constant_fold, constant_propagate)` combination.
const ALL_OPTIONS: [(bool, bool); 4] = [(false, false), (false, true), (true, false), (true, true)];

fn options(constant_fold: bool, constant_propagate: bool) -> CompilerOptions {
    CompilerOptions {
        optimization_options: OptimizationOptions {
            constant_fold,
            constant_propagate,
        },
        implicit_export: false,
    }
}

/// Compile `source` under the given options and run it: the tail value, or the
/// runtime error's message.
fn outcome(source: &str, (fold, propagate): (bool, bool)) -> Result<Value, String> {
    let output =
        compile_program("test.frst", source, options(fold, propagate)).unwrap_or_else(|errors| {
            panic!(
                "{source:?} should compile (fold: {fold}, propagate: {propagate}):\n{}",
                errors.render_plain()
            )
        });
    let closure = output
        .code
        .into_closure()
        .expect("top level needs no captures");
    Vm::factory()
        .build(closure)
        .expect("closure builds")
        .run()
        .map(|result| result.tail().clone())
        .map_err(|error| error.into_error().message().into_owned())
}

/// Run `source` under every option combination, requiring them all to agree.
fn evaluate(source: &str) -> Result<Value, String> {
    let baseline = outcome(source, ALL_OPTIONS[0]);
    for option_set in &ALL_OPTIONS[1..] {
        assert_eq!(
            outcome(source, *option_set),
            baseline,
            "{source:?}: (fold, propagate) = {option_set:?} disagrees with no optimization"
        );
    }
    baseline
}

/// The value of `source`, which must run without error under every option set.
fn run(source: &str) -> Value {
    evaluate(source)
        .unwrap_or_else(|message| panic!("{source:?} should run, but raised: {message}"))
}

/// The runtime error message of `source`, which must raise under every option set.
fn run_error(source: &str) -> String {
    match evaluate(source) {
        Ok(value) => panic!("{source:?} should raise, but produced {value:?}"),
        Err(message) => message,
    }
}

fn float(f: f64) -> Value {
    Value::try_from(f).expect("test floats are finite")
}

/// The top-level function's emitted bytecode.
fn code(source: &str, fold: bool, propagate: bool) -> Vec<Bytecode> {
    compile_program("test.frst", source, options(fold, propagate))
        .expect("source should compile")
        .code
        .into_closure()
        .expect("top level needs no captures")
        .inner_fn()
        .code
        .clone()
}

/// How many times `op` appears in `code`.
fn count(code: &[Bytecode], op: &Bytecode) -> usize {
    code.iter().filter(|candidate| *candidate == op).count()
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
    assert_eq!(run("def x = 4\ndef y = 3\nx - y"), Value::Int(1));
    assert_eq!(run("def x = 4\nx * x"), Value::Int(16));
    assert_eq!(
        run("def x = 2 + 3\ndef y = x * 2\ny - x"),
        Value::Int(5),
        "bindings of computed values"
    );
}

// --- Runtime errors ---

#[test]
fn a_type_error_names_the_operand_types_in_order() {
    let message = run_error(r#"1 + "a""#);
    assert!(
        message.contains("Int + String"),
        "operand types, left first: {message}"
    );
    let message = run_error(r#""a" - 1"#);
    assert!(
        message.contains("String - Int"),
        "operand types, left first: {message}"
    );
}

#[test]
fn division_and_modulus_by_zero_raise() {
    assert!(run_error("1 / 0").contains("Division by zero"));
    assert!(run_error("1.0 / 0.0").contains("Division by zero"));
    assert!(run_error("1 % 0").contains("Modulus by zero"));
}

#[test]
fn an_unorderable_comparison_raises() {
    let message = run_error("true < false");
    assert!(message.contains("not orderable"), "{message}");
    let message = run_error(r#"1 < "a""#);
    assert!(message.contains("incompatible"), "{message}");
}

#[test]
fn the_left_operand_is_evaluated_first() {
    // Both operands raise, so the error reveals which ran first.
    let message = run_error("(1 / 0) + (1 % 0)");
    assert!(
        message.contains("Division by zero"),
        "the left operand's error wins: {message}"
    );
}

// --- Constant folding ---

#[test]
fn a_constant_operation_folds_to_its_value() {
    let folded = code("2 + 3", true, false);
    assert_eq!(count(&folded, &Bytecode::Add), 0, "the Add is folded away");
    assert_eq!(
        count(&folded, &Bytecode::PushInt(5)),
        1,
        "its value is pushed"
    );

    let unfolded = code("2 + 3", false, false);
    assert_eq!(
        count(&unfolded, &Bytecode::Add),
        1,
        "with folding off the Add stays"
    );
}

#[test]
fn a_nested_constant_operation_folds_whole() {
    let folded = code("(1 + 2) * (3 + 4)", true, false);
    assert_eq!(count(&folded, &Bytecode::Add), 0);
    assert_eq!(count(&folded, &Bytecode::Multiply), 0);
    assert_eq!(count(&folded, &Bytecode::PushInt(21)), 1);
}

#[test]
fn a_structured_result_folds_to_a_constant() {
    let folded = code(r#""ab" + "cd""#, true, false);
    assert_eq!(count(&folded, &Bytecode::Add), 0, "the Add is folded away");
    assert!(
        folded.iter().any(|op| matches!(op, Bytecode::LoadConst(_))),
        "the String result is loaded from the pool: {folded:?}"
    );
}

#[test]
fn the_constant_side_of_a_mixed_operation_folds() {
    // With propagation off, `x` is a runtime load, so the outer Add stays; its
    // constant operand still folds, whichever side it is on.
    for source in ["def x = 1\nx + (2 * 3)", "def x = 1\n(2 * 3) + x"] {
        let folded = code(source, true, false);
        assert_eq!(
            count(&folded, &Bytecode::Add),
            1,
            "{source:?}: the Add stays"
        );
        assert_eq!(
            count(&folded, &Bytecode::Multiply),
            0,
            "{source:?}: the constant operand is folded"
        );
        assert_eq!(count(&folded, &Bytecode::PushInt(6)), 1, "{source:?}");
    }
}

#[test]
fn the_largest_constant_subtree_folds_as_one() {
    // Every constant operation below the non-constant `x` folds into one value.
    let folded = code("def x = 1\n((1 + 2) + 3) + x", true, false);
    assert_eq!(
        count(&folded, &Bytecode::Add),
        1,
        "only the Add over `x` stays"
    );
    assert_eq!(count(&folded, &Bytecode::PushInt(6)), 1);
}

#[test]
fn a_propagated_binding_folds() {
    let folded = code("def x = 2\nx * 3", true, true);
    assert_eq!(count(&folded, &Bytecode::Multiply), 0);
    assert_eq!(count(&folded, &Bytecode::PushInt(6)), 1);
}

#[test]
fn a_computed_binding_propagates_its_folded_value() {
    // `x` binds a folded constant, so propagation makes `x + 1` foldable too.
    let folded = code("def x = 2 * 3\nx + 1", true, true);
    assert_eq!(count(&folded, &Bytecode::Multiply), 0);
    assert_eq!(count(&folded, &Bytecode::Add), 0);
    assert_eq!(count(&folded, &Bytecode::PushInt(7)), 1);
}

#[test]
fn an_operation_over_a_pure_global_folds() {
    let folded = code("len == len", true, false);
    assert_eq!(count(&folded, &Bytecode::CompareEqual), 0);
    assert_eq!(count(&folded, &Bytecode::PushTrue), 1);
}

#[test]
fn an_operation_over_an_impure_global_does_not_fold() {
    let unfolded = code("print == print", true, false);
    assert_eq!(count(&unfolded, &Bytecode::CompareEqual), 1);
}

#[test]
fn a_failing_fold_is_left_for_runtime() {
    // The fold raises, so the operation is kept and raises only when run.
    for (source, op) in [
        ("1 / 0", Bytecode::Divide),
        ("1 % 0", Bytecode::Modulus),
        (r#"1 + "a""#, Bytecode::Add),
        ("len + 1", Bytecode::Add),
    ] {
        assert_eq!(
            count(&code(source, true, false), &op),
            1,
            "{source:?}: the failing operation is kept"
        );
    }
}

#[test]
fn a_failing_constant_operand_is_left_for_runtime() {
    let unfolded = code("def x = 1\nx + (1 / 0)", true, false);
    assert_eq!(count(&unfolded, &Bytecode::Divide), 1);
    assert!(run_error("def x = 1\nx + (1 / 0)").contains("Division by zero"));
}
