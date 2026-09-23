//! `and` / `or` lowering, end to end: compile full source, run it on the VM,
//! and check the result.
//!
//! The semantics are Lua's: only `null` and `false` are falsy, evaluation
//! short-circuits, and the result is the deciding operand itself, not a Bool.
//! `and` binds tighter than `or`, and both are left associative, so Lua's
//! and/or idioms carry over unchanged.
//!
//! An operand's form decides which lowering path it exercises: a literal is
//! foldable, a capture is known only at runtime, and `(1 / 0)` raises if it is
//! ever evaluated, making short-circuiting observable. The chaining tests run
//! every mix of these forms through each chain shape, against a reference
//! evaluator of the Lua semantics.
//!
//! Every case runs under each combination of optimization options, which must
//! all agree: optimization never changes a result.

use std::collections::BTreeMap;

use frost_compile::{CompilerOptions, OptimizationOptions, compile_in_scope};
use frost_runtime::{Bytecode, Value, Vm};

/// Every `(constant_fold, constant_propagate)` combination.
const ALL_OPTIONS: [(bool, bool); 4] = [(false, false), (false, true), (true, false), (true, true)];

/// The operand that raises when evaluated, and the message it raises.
const RAISE: &str = "(1 / 0)";
const RAISE_MESSAGE: &str = "Division by zero";

fn options(constant_fold: bool, constant_propagate: bool) -> CompilerOptions {
    CompilerOptions {
        optimization_options: OptimizationOptions {
            constant_fold,
            constant_propagate,
        },
        implicit_export: false,
    }
}

/// Compile `source` with `captures` as its enclosing scope, close over them, and
/// run it: the tail value, or the runtime error's message.
fn outcome(
    source: &str,
    captures: &[(&str, Value)],
    (fold, propagate): (bool, bool),
) -> Result<Value, String> {
    let names: Vec<&str> = captures.iter().map(|(name, _)| *name).collect();
    let output = compile_in_scope("test.frst", source, options(fold, propagate), &names)
        .unwrap_or_else(|errors| {
            panic!(
                "{source:?} should compile (fold: {fold}, propagate: {propagate}):\n{}",
                errors.render_plain()
            )
        });
    let values: BTreeMap<String, Value> = captures
        .iter()
        .map(|(name, value)| (name.to_string(), value.clone()))
        .collect();
    let closure = output
        .code
        .close(values)
        .expect("every capture is supplied");
    Vm::factory()
        .build(closure)
        .expect("closure builds")
        .run()
        .map(|result| result.tail().clone())
        .map_err(|error| error.into_error().message().into_owned())
}

/// Run `source` under every option combination, requiring them all to agree.
fn evaluate(source: &str, captures: &[(&str, Value)]) -> Result<Value, String> {
    let baseline = outcome(source, captures, ALL_OPTIONS[0]);
    for option_set in &ALL_OPTIONS[1..] {
        assert_eq!(
            outcome(source, captures, *option_set),
            baseline,
            "{source:?} with {captures:?}: (fold, propagate) = {option_set:?} \
             disagrees with no optimization"
        );
    }
    baseline
}

/// The value of `source` with `captures`, which must run without error.
fn run_with(source: &str, captures: &[(&str, Value)]) -> Value {
    evaluate(source, captures).unwrap_or_else(|message| {
        panic!("{source:?} with {captures:?} should run, but raised: {message}")
    })
}

/// The value of `source`, which must run without error.
fn run(source: &str) -> Value {
    run_with(source, &[])
}

/// Assert that `source` raises the [`RAISE`] operand's error.
fn assert_raises(source: &str) {
    match evaluate(source, &[]) {
        Ok(value) => panic!("{source:?} should raise, but produced {value:?}"),
        Err(message) => assert!(
            message.contains(RAISE_MESSAGE),
            "{source:?} raised the wrong error: {message}"
        ),
    }
}

/// The top-level function's emitted bytecode, with `x` as a runtime-only
/// capture and propagation off.
fn code(source: &str, fold: bool) -> Vec<Bytecode> {
    compile_in_scope("test.frst", source, options(fold, false), &["x"])
        .expect("source should compile")
        .code
        .close(BTreeMap::from([("x".to_string(), Value::Null)]))
        .expect("`x` is supplied")
        .inner_fn()
        .code
        .clone()
}

/// How many times `op` appears in `code`.
fn count(code: &[Bytecode], op: &Bytecode) -> usize {
    code.iter().filter(|candidate| *candidate == op).count()
}

/// How many conditional jumps (the short-circuit tests) remain in `code`.
fn jumps(code: &[Bytecode]) -> usize {
    code.iter()
        .filter(|op| {
            matches!(
                op,
                Bytecode::PeekJumpIfTrue(_) | Bytecode::PeekJumpIfFalse(_)
            )
        })
        .count()
}

// --- The result is the deciding operand ---

#[test]
fn and_yields_the_first_falsy_operand_or_else_the_last() {
    assert_eq!(run("true and 5"), Value::Int(5));
    assert_eq!(run("5 and true"), Value::Bool(true));
    assert_eq!(run("null and 5"), Value::Null);
    assert_eq!(run("false and 5"), Value::Bool(false));
    assert_eq!(run("5 and null"), Value::Null);
    assert_eq!(run("5 and false"), Value::Bool(false));
}

#[test]
fn or_yields_the_first_truthy_operand_or_else_the_last() {
    assert_eq!(run("5 or true"), Value::Int(5));
    assert_eq!(run("null or 5"), Value::Int(5));
    assert_eq!(run("false or 5"), Value::Int(5));
    assert_eq!(run("null or false"), Value::Bool(false));
    assert_eq!(run("false or null"), Value::Null);
}

#[test]
fn only_null_and_false_are_falsy() {
    // `0`, `0.0`, `""`, and `x''` are truthy.
    for truthy in ["0", "0.0", r#""""#, "x''", "true"] {
        assert_eq!(
            run(&format!("{truthy} and 1")),
            Value::Int(1),
            "{truthy} is truthy, so `and` moves on"
        );
        assert_eq!(
            run(&format!("{truthy} or 1")),
            run(truthy),
            "{truthy} is truthy, so `or` stops there"
        );
    }
}

#[test]
fn the_result_is_not_coerced_to_bool() {
    assert_eq!(run(r#"1 and "yes""#), Value::from("yes"));
    assert_eq!(run(r#"null or "default""#), Value::from("default"));
    assert_eq!(run("false or 0"), Value::Int(0));
    assert_eq!(run("2.5 or 1"), Value::try_from(2.5).unwrap());
}

// --- Short-circuiting ---

#[test]
fn a_decided_and_skips_its_right_operand() {
    assert_eq!(run(&format!("false and {RAISE}")), Value::Bool(false));
    assert_eq!(run(&format!("null and {RAISE}")), Value::Null);
    assert_raises(&format!("true and {RAISE}"));
    assert_raises(&format!("0 and {RAISE}"));
}

#[test]
fn a_decided_or_skips_its_right_operand() {
    assert_eq!(run(&format!("true or {RAISE}")), Value::Bool(true));
    assert_eq!(run(&format!("0 or {RAISE}")), Value::Int(0));
    assert_raises(&format!("false or {RAISE}"));
    assert_raises(&format!("null or {RAISE}"));
}

#[test]
fn the_left_operand_is_always_evaluated() {
    assert_raises(&format!("{RAISE} and true"));
    assert_raises(&format!("{RAISE} or true"));
}

#[test]
fn a_runtime_operand_short_circuits() {
    // `x` is known only at runtime, so the decision is made by the VM.
    let and_source = format!("x and {RAISE}");
    let or_source = format!("x or {RAISE}");
    assert_eq!(run_with(&and_source, &[("x", Value::Null)]), Value::Null);
    assert_eq!(run_with(&or_source, &[("x", Value::Int(3))]), Value::Int(3));
    for (source, x) in [(and_source, Value::Int(3)), (or_source, Value::Null)] {
        let message = evaluate(&source, &[("x", x.clone())])
            .expect_err("the undecided right operand runs, and raises");
        assert!(
            message.contains(RAISE_MESSAGE),
            "{source:?} with x = {x:?}: {message}"
        );
    }
}

// --- Lua idioms ---

#[test]
fn or_supplies_a_default() {
    let source = r#"name or "anonymous""#;
    for (name, expected) in [
        (Value::Null, Value::from("anonymous")),
        (Value::Bool(false), Value::from("anonymous")),
        (Value::from("ada"), Value::from("ada")),
        (Value::from(""), Value::from("")),
        (Value::Int(0), Value::Int(0)),
    ] {
        assert_eq!(
            run_with(source, &[("name", name.clone())]),
            expected,
            "name = {name:?}"
        );
    }
}

#[test]
fn and_or_is_a_conditional_expression() {
    let source = r#"cond and "yes" or "no""#;
    assert_eq!(
        run_with(source, &[("cond", Value::Bool(true))]),
        Value::from("yes")
    );
    assert_eq!(
        run_with(source, &[("cond", Value::Bool(false))]),
        Value::from("no")
    );
    assert_eq!(
        run_with(source, &[("cond", Value::Null)]),
        Value::from("no")
    );
    assert_eq!(
        run_with(source, &[("cond", Value::Int(0))]),
        Value::from("yes")
    );
}

#[test]
fn and_or_falls_through_on_a_falsy_middle_operand() {
    // The idiom's well-known pitfall, exactly as in Lua: a falsy "then" value
    // is indistinguishable from a false condition.
    assert_eq!(
        run(r#"true and false or "fallback""#),
        Value::from("fallback")
    );
    assert_eq!(
        run(r#"true and null or "fallback""#),
        Value::from("fallback")
    );
}

#[test]
fn and_or_picks_the_maximum() {
    let source = "a > b and a or b";
    for (a, b) in [(3, 5), (5, 3), (4, 4)] {
        assert_eq!(
            run_with(source, &[("a", Value::Int(a)), ("b", Value::Int(b))]),
            Value::Int(a.max(b)),
            "max({a}, {b})"
        );
    }
}

#[test]
fn and_guards_an_operation_on_a_null() {
    // `x > 0` would raise on a Null; the guard stops it being evaluated.
    let source = "x and x > 0 and x < 10";
    for (x, expected) in [
        (Value::Null, Value::Null),
        (Value::Int(5), Value::Bool(true)),
        (Value::Int(0), Value::Bool(false)),
        (Value::Int(50), Value::Bool(false)),
    ] {
        assert_eq!(run_with(source, &[("x", x.clone())]), expected, "x = {x:?}");
    }
}

// TODO: add Lua's nested-field guard `foo and foo.bar and foo.bar.baz` once
// hard indexing lowers.

#[test]
fn or_picks_the_first_truthy_of_many() {
    let source = "a or b or c";
    let falsy_then_seven = [
        ("a", Value::Null),
        ("b", Value::Bool(false)),
        ("c", Value::Int(7)),
    ];
    assert_eq!(run_with(source, &falsy_then_seven), Value::Int(7));
    let all_falsy = [
        ("a", Value::Null),
        ("b", Value::Null),
        ("c", Value::Bool(false)),
    ];
    assert_eq!(
        run_with(source, &all_falsy),
        Value::Bool(false),
        "the last operand"
    );
}

// --- Chaining: every operand form through every shape ---

/// A chain shape: an expression tree over numbered operands.
enum Chain {
    Operand(usize),
    And(Box<Chain>, Box<Chain>),
    Or(Box<Chain>, Box<Chain>),
}

fn operand(index: usize) -> Chain {
    Chain::Operand(index)
}

fn and(left: Chain, right: Chain) -> Chain {
    Chain::And(Box::new(left), Box::new(right))
}

fn or(left: Chain, right: Chain) -> Chain {
    Chain::Or(Box::new(left), Box::new(right))
}

/// How an operand is written, which decides the lowering path it takes.
#[derive(Clone, Debug)]
enum Form {
    /// A literal: compile-time known, so foldable.
    Literal(Value),
    /// A capture: known only at runtime.
    Capture(Value),
    /// [`RAISE`]: raises if evaluated.
    Raise,
}

/// Every operand form: each sample value as a literal and as a capture, and
/// the raising operand. The samples cover both falsy values and truthy ones
/// that are falsy in other languages.
fn forms() -> Vec<Form> {
    let samples = [
        Value::Null,
        Value::Bool(false),
        Value::Bool(true),
        Value::Int(0),
    ];
    let mut forms: Vec<Form> = samples.iter().cloned().map(Form::Literal).collect();
    forms.extend(samples.into_iter().map(Form::Capture));
    forms.push(Form::Raise);
    forms
}

/// The Lua semantics: the deciding operand, or `None` if an evaluated operand
/// raises.
fn reference(chain: &Chain, forms: &[Form]) -> Option<Value> {
    let truthy = |value: &Value| !matches!(value, Value::Null | Value::Bool(false));
    match chain {
        Chain::Operand(index) => match &forms[*index] {
            Form::Literal(value) | Form::Capture(value) => Some(value.clone()),
            Form::Raise => None,
        },
        Chain::And(left, right) => {
            let left = reference(left, forms)?;
            if truthy(&left) {
                reference(right, forms)
            } else {
                Some(left)
            }
        }
        Chain::Or(left, right) => {
            let left = reference(left, forms)?;
            if truthy(&left) {
                Some(left)
            } else {
                reference(right, forms)
            }
        }
    }
}

/// Frost source for a literal value.
fn literal_source(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(true) => "true",
        Value::Bool(false) => "false",
        Value::Int(0) => "0",
        other => unreachable!("no literal source for sample {other:?}"),
    }
}

/// Check `template` (with `{0}`, `{1}`, ... for operands) against `chain`, for
/// every assignment of operand forms, under every option set.
fn check_chain(template: &str, chain: &Chain, arity: usize) {
    let forms = forms();
    // Every assignment of forms to the `arity` operands: counting in base
    // `forms.len()`, one digit per operand.
    for mut assignment in 0..forms.len().pow(arity as u32) {
        let chosen: Vec<Form> = (0..arity)
            .map(|_| {
                let form = forms[assignment % forms.len()].clone();
                assignment /= forms.len();
                form
            })
            .collect();

        let mut source = template.to_string();
        let mut captures = Vec::new();
        for (index, form) in chosen.iter().enumerate() {
            let text = match form {
                Form::Literal(value) => literal_source(value).to_string(),
                Form::Capture(value) => {
                    let name = format!("c{index}");
                    captures.push((name.clone(), value.clone()));
                    name
                }
                Form::Raise => RAISE.to_string(),
            };
            source = source.replace(&format!("{{{index}}}"), &text);
        }
        let captures: Vec<(&str, Value)> = captures
            .iter()
            .map(|(name, value)| (name.as_str(), value.clone()))
            .collect();

        let expected = reference(chain, &chosen);
        for option_set in ALL_OPTIONS {
            let actual = outcome(&source, &captures, option_set);
            match (&expected, &actual) {
                (Some(expected), Ok(actual)) if expected == actual => {}
                (None, Err(message)) if message.contains(RAISE_MESSAGE) => {}
                _ => panic!(
                    "{source:?} with {captures:?}, (fold, propagate) = {option_set:?}: \
                     expected {expected:?} (None: raises), got {actual:?}"
                ),
            }
        }
    }
}

#[test]
fn a_single_and() {
    check_chain("{0} and {1}", &and(operand(0), operand(1)), 2);
}

#[test]
fn a_single_or() {
    check_chain("{0} or {1}", &or(operand(0), operand(1)), 2);
}

#[test]
fn an_and_chain() {
    check_chain(
        "{0} and {1} and {2}",
        &and(and(operand(0), operand(1)), operand(2)),
        3,
    );
}

#[test]
fn an_or_chain() {
    check_chain(
        "{0} or {1} or {2}",
        &or(or(operand(0), operand(1)), operand(2)),
        3,
    );
}

#[test]
fn a_right_nested_and_chain() {
    check_chain(
        "{0} and ({1} and {2})",
        &and(operand(0), and(operand(1), operand(2))),
        3,
    );
}

#[test]
fn a_right_nested_or_chain() {
    check_chain(
        "{0} or ({1} or {2})",
        &or(operand(0), or(operand(1), operand(2))),
        3,
    );
}

#[test]
fn and_binds_tighter_than_or() {
    check_chain(
        "{0} and {1} or {2}",
        &or(and(operand(0), operand(1)), operand(2)),
        3,
    );
    check_chain(
        "{0} or {1} and {2}",
        &or(operand(0), and(operand(1), operand(2))),
        3,
    );
}

#[test]
fn parentheses_override_precedence() {
    check_chain(
        "({0} or {1}) and {2}",
        &and(or(operand(0), operand(1)), operand(2)),
        3,
    );
    check_chain(
        "{0} and ({1} or {2})",
        &and(operand(0), or(operand(1), operand(2))),
        3,
    );
}

#[test]
fn two_conditionals_joined_by_or() {
    check_chain(
        "{0} and {1} or {2} and {3}",
        &or(and(operand(0), operand(1)), and(operand(2), operand(3))),
        4,
    );
}

// --- Constant folding ---

#[test]
fn a_constant_logical_folds_to_its_value() {
    for (source, value) in [
        ("true and false", Bytecode::PushFalse),
        ("null or 5", Bytecode::PushInt(5)),
        ("null or false or 5", Bytecode::PushInt(5)),
        ("1 and 2 and null", Bytecode::PushNull),
    ] {
        let folded = code(source, true);
        assert_eq!(jumps(&folded), 0, "{source:?}: the test is folded away");
        assert_eq!(count(&folded, &value), 1, "{source:?}: its value is pushed");

        let unfolded = code(source, false);
        assert!(
            jumps(&unfolded) > 0,
            "{source:?}: with folding off the test stays"
        );
    }
}

#[test]
fn a_fold_short_circuits_past_a_raising_operand() {
    // The skipped `1 / 0` never runs in the fold VM either, so the whole
    // expression folds.
    let folded = code(&format!("false and {RAISE}"), true);
    assert_eq!(jumps(&folded), 0);
    assert_eq!(count(&folded, &Bytecode::Divide), 0);
    assert_eq!(count(&folded, &Bytecode::PushFalse), 1);
}

#[test]
fn a_fold_that_reaches_a_raising_operand_is_left_for_runtime() {
    let unfolded = code(&format!("true and {RAISE}"), true);
    assert_eq!(jumps(&unfolded), 1, "the test is kept");
    assert_eq!(
        count(&unfolded, &Bytecode::Divide),
        1,
        "the division is kept"
    );
}

#[test]
fn the_constant_side_of_a_runtime_logical_folds() {
    for (source, jump) in [
        ("x and (1 + 2)", "and"),
        ("(1 + 2) or x", "or"),
        ("x or (1 + 2)", "or"),
        ("(1 + 2) and x", "and"),
    ] {
        let folded = code(source, true);
        assert_eq!(jumps(&folded), 1, "{source:?}: the runtime {jump} stays");
        assert_eq!(
            count(&folded, &Bytecode::Add),
            0,
            "{source:?}: the constant side folds"
        );
        assert_eq!(count(&folded, &Bytecode::PushInt(3)), 1, "{source:?}");
    }
}

#[test]
fn constant_operands_fold_throughout_a_runtime_chain() {
    // `x and (1 + 2)` cannot fold, so the outer `and` folds its constant side.
    let folded = code("x and (1 + 2) and (3 + 4)", true);
    assert_eq!(jumps(&folded), 2, "both runtime tests stay");
    assert_eq!(
        count(&folded, &Bytecode::Add),
        0,
        "every constant operand folds"
    );
    assert_eq!(count(&folded, &Bytecode::PushInt(3)), 1);
    assert_eq!(count(&folded, &Bytecode::PushInt(7)), 1);
}

#[test]
fn a_constant_prefix_of_a_runtime_chain_folds_whole() {
    // `null or false` is a constant subtree under the runtime `or x`.
    let folded = code("null or false or x", true);
    assert_eq!(jumps(&folded), 1, "only the test against `x` stays");
    assert_eq!(
        count(&folded, &Bytecode::PushNull),
        0,
        "the constant prefix folds"
    );
    assert_eq!(count(&folded, &Bytecode::PushFalse), 1);
}
