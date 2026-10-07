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
//! ever evaluated, making short-circuiting observable; the evaluation-order tests
//! log each operand evaluated to a mutable cell. The chaining tests run
//! every mix of these forms through each chain shape, against a reference
//! evaluator of the Lua semantics.
//!
//! The harness runs every behavioral case under every optimization permutation.

mod script;

use frostlang::Value;
use frostlang::bytecode::Bytecode;
use frostlang::compile::{Optimization, OptimizationOptions};
use script::{Emitted, Script, UNOPTIMIZED, run};

/// The operand that raises when evaluated, and the message it raises.
const RAISE: &str = "(1 / 0)";
const RAISE_MESSAGE: &str = "Division by zero";

/// The value of `source` with `captures` in scope, which must run.
fn run_with(source: &str, captures: &[(&str, Value)]) -> Value {
    Script::new(source).captures(captures).run()
}

/// Assert that `source`, with `captures` in scope, raises the [`RAISE`]
/// operand's error.
fn assert_raises_with(source: &str, captures: &[(&str, Value)]) {
    let message = Script::new(source).captures(captures).raises();
    assert!(
        message.contains(RAISE_MESSAGE),
        "{source:?} with {captures:?} raised the wrong error: {message}"
    );
}

fn assert_raises(source: &str) {
    assert_raises_with(source, &[]);
}

/// The code of `source`, with `x` a runtime-only capture, under each
/// optimization permutation `select` accepts.
fn emitted(source: &str, select: impl Fn(&OptimizationOptions) -> bool) -> Vec<Emitted> {
    Script::new(source)
        .capture("x", Value::Null)
        .code_where(select)
}

/// Folding in isolation from branch elimination.
fn folding_only(optimization: &OptimizationOptions) -> bool {
    optimization.constant_fold && !optimization.branch_eliminate
}

/// Neither folding nor branch elimination: every test is emitted.
fn neither_folding_nor_eliminating(optimization: &OptimizationOptions) -> bool {
    !optimization.constant_fold && !optimization.branch_eliminate
}

fn eliminating(optimization: &OptimizationOptions) -> bool {
    optimization.branch_eliminate
}

/// Branch elimination in isolation from folding.
fn eliminating_only(optimization: &OptimizationOptions) -> bool {
    optimization.branch_eliminate && !optimization.constant_fold
}

/// Whether the code reads the runtime-only `x`.
fn loads_x(emitted: &Emitted) -> bool {
    emitted
        .code
        .iter()
        .any(|op| matches!(op, Bytecode::LoadLocal(_) | Bytecode::ConsumeLocal(_)))
}

/// How many conditional jumps (the short-circuit tests) remain in the code.
fn jumps(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
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
    // Undecided, so the right operand runs, and raises.
    assert_raises_with(&and_source, &[("x", Value::Int(3))]);
    assert_raises_with(&or_source, &[("x", Value::Null)]);
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
    for (cond, expected) in [
        (Value::Bool(true), "yes"),
        (Value::Bool(false), "no"),
        (Value::Null, "no"),
        (Value::Int(0), "yes"),
    ] {
        assert_eq!(
            run_with(source, &[("cond", cond.clone())]),
            Value::from(expected),
            "cond = {cond:?}"
        );
    }
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

#[test]
fn and_guards_a_nested_lookup() {
    // Lua's nested-field guard. With a soft index, a missing key is a Null
    // like any other, so the chain stops there.
    let source = r#"foo and foo["bar"] and foo["bar"]["baz"]"#;
    for (foo, expected) in [
        ("null", "null"),
        ("{}", "null"),
        ("{bar: null}", "null"),
        ("{bar: false}", "false"),
        ("{bar: {}}", "null"),
        ("{bar: {baz: 3}}", "3"),
        ("{bar: {baz: false}}", "false"),
    ] {
        assert_eq!(
            run_with(source, &[("foo", run(foo))]),
            run(expected),
            "foo = {foo}"
        );
    }
}

#[test]
fn and_does_not_guard_a_hard_index_of_a_missing_key() {
    // A hard index raises on a missing key, which no guard on the value
    // before it catches; it guards only a Null or false the key holds.
    let source = "foo and foo.bar and foo.bar.baz";
    for (foo, expected) in [
        ("null", "null"),
        ("{bar: null}", "null"),
        ("{bar: false}", "false"),
        ("{bar: {baz: 3}}", "3"),
    ] {
        assert_eq!(
            run_with(source, &[("foo", run(foo))]),
            run(expected),
            "foo = {foo}"
        );
    }
    for (foo, missing) in [("{}", "bar"), ("{bar: {}}", "baz")] {
        let message = Script::new(source).captures(&[("foo", run(foo))]).raises();
        assert!(
            message.contains("no value at key") && message.contains(missing),
            "foo = {foo}: {message}"
        );
    }
}

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

#[test]
fn a_long_chain_stops_at_its_deciding_operand() {
    // Twenty runtime operands, the deciding one at each position in turn.
    const LENGTH: usize = 20;
    for decider in 0..LENGTH {
        let names: Vec<String> = (0..LENGTH).map(|index| format!("c{index}")).collect();
        let captures = |deciding: Value, other: Value| -> Vec<(&str, Value)> {
            names
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    let value = if index == decider {
                        deciding.clone()
                    } else {
                        other.clone()
                    };
                    (name.as_str(), value)
                })
                .collect()
        };
        assert_eq!(
            run_with(&names.join(" or "), &captures(Value::Int(7), Value::Null)),
            Value::Int(7),
            "an `or` chain decided at operand {decider}"
        );
        assert_eq!(
            run_with(
                &names.join(" and "),
                &captures(Value::Bool(false), Value::Int(1))
            ),
            Value::Bool(false),
            "an `and` chain decided at operand {decider}"
        );
    }
}

// --- Other values as operands ---

#[test]
fn structures_functions_and_negative_numbers_are_truthy() {
    for truthy in ["[]", "{}", "[null]", "{a: false}", "-1", r#""false""#] {
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
    for function in ["len", "(fn -> null)", "$(null)"] {
        assert_eq!(
            run(&format!("{function} and 1")),
            Value::Int(1),
            "{function} is truthy, so `and` moves on"
        );
        assert_eq!(
            run(&format!("is_function({function} or 1)")),
            Value::Bool(true),
            "{function} is truthy, so `or` stops there"
        );
    }
}

#[test]
fn a_block_may_be_an_operand() {
    let source = r"
        do {
            def y = x
            y
        } or do {
            def z = 2
            z * 3
        }
    ";
    assert_eq!(run_with(source, &[("x", Value::Null)]), Value::Int(6));
    assert_eq!(run_with(source, &[("x", Value::Int(1))]), Value::Int(1));
    let source = r"
        do {
            def y = x
            y
        } and do {
            def z = 2
            z * 3
        }
    ";
    assert_eq!(run_with(source, &[("x", Value::Null)]), Value::Null);
    assert_eq!(run_with(source, &[("x", Value::Int(1))]), Value::Int(6));
}

// --- Evaluation order ---

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

#[test]
fn operands_are_evaluated_once_each_left_to_right() {
    for (statement, expected) in [
        ("note(1) and note(2) or note(3)", "[1, 2]"),
        ("note(null) and note(2) or note(3)", "[null, 3]"),
        (
            "note(false) or note(null) or note(0) or note(4)",
            "[false, null, 0]",
        ),
        (
            "note(1) and note(2) and note(null) and note(4)",
            "[1, 2, null]",
        ),
        ("note(null) or (note(1) and note(2))", "[null, 1, 2]"),
    ] {
        let source = format!(
            r"
            {NOTE}
            {statement}
            log()
            "
        );
        assert_eq!(
            run(&source),
            run(expected),
            "{statement:?} evaluates the noted operands in order"
        );
    }
}

// --- In context ---

#[test]
fn a_logical_statement_leaves_the_stack_balanced() {
    // A non-tail logical is evaluated and dropped, whichever way it decides.
    for x in [
        Value::Null,
        Value::Bool(false),
        Value::Bool(true),
        Value::Int(0),
    ] {
        for source in [
            r"
            x and 1
            5
            ",
            r"
            x or 1
            5
            ",
            r"
            x and 1 or 2
            x or x and 3
            5
            ",
            r"
            true and x
            false or x
            null and x
            0 or x
            5
            ",
        ] {
            assert_eq!(
                run_with(source, &[("x", x.clone())]),
                Value::Int(5),
                "{source:?} with x = {x:?}"
            );
        }
    }
}

#[test]
fn a_logical_leaves_exactly_its_value_among_others() {
    // Mid-expression, the short-circuit path and the full path must each leave
    // one value on top of those already on the stack.
    for (source, if_null, if_five) in [
        ("[1, x and 2, x or 3, 4]", "[1, null, 3, 4]", "[1, 2, 5, 4]"),
        ("10 + (x or 1) * 2", "12", "20"),
        (r#"{[x and "a" or "b"]: x or 0}"#, "{b: 0}", "{a: 5}"),
        (r#"$'<${x or "none"}>'"#, "'<none>'", "'<5>'"),
        ("[x and (x or 1), (x or 2) and x]", "[null, null]", "[5, 5]"),
    ] {
        assert_eq!(
            run_with(source, &[("x", Value::Null)]),
            run(if_null),
            "{source:?} with x = null"
        );
        assert_eq!(
            run_with(source, &[("x", Value::Int(5))]),
            run(if_five),
            "{source:?} with x = 5"
        );
    }
}

#[test]
fn a_logical_value_may_be_bound_and_reused() {
    let source = r"
        def y = x or 3
        def z = y and y + 1
        [y, z]
    ";
    assert_eq!(run_with(source, &[("x", Value::Null)]), run("[3, 4]"));
    assert_eq!(run_with(source, &[("x", Value::Int(7))]), run("[7, 8]"));
}

#[test]
fn a_logical_in_a_lambda_decides_on_each_call() {
    let pick = r"
        def pick = fn a, b -> a or b
        [pick(null, 1), pick(2, 1), pick(false, null)]
    ";
    assert_eq!(run(pick), run("[1, 2, null]"));
    let guard = r"
        def guard = fn a -> a and a + 1
        [guard(null), guard(false), guard(1)]
    ";
    assert_eq!(run(guard), run("[null, false, 2]"));
}

#[test]
fn a_skipped_operand_in_a_lambda_never_runs() {
    let define = format!("def f = fn a -> a or {RAISE}");
    let call_with = |argument: &str| {
        format!(
            r"
            {define}
            f({argument})
            "
        )
    };
    assert_eq!(run(&call_with("1")), Value::Int(1));
    assert_raises(&call_with("null"));
}

#[test]
fn a_runtime_binding_shadowing_a_constant_decides_at_runtime() {
    // The inner `t` is the runtime `x`, not the outer constant `true`.
    let source = r"
        def t = true
        do {
            def t = x
            t and 1
        }
    ";
    assert_eq!(run_with(source, &[("x", Value::Null)]), Value::Null);
    assert_eq!(
        run_with(source, &[("x", Value::Bool(false))]),
        Value::Bool(false)
    );
    assert_eq!(run_with(source, &[("x", Value::Int(0))]), Value::Int(1));
    // Once the block ends, the outer constant is back in view.
    let source = r"
        def t = null
        def inner = do {
            def t = x
            t or 2
        }
        [inner, t or 3]
    ";
    assert_eq!(run_with(source, &[("x", Value::Int(1))]), run("[1, 3]"));
}

#[test]
fn an_effectful_right_operand_is_skipped_when_the_left_decides() {
    // `print` would be called if its operand ran.
    assert_eq!(run("false and print(1)"), Value::Bool(false));
    assert_eq!(run("true or print(1)"), Value::Bool(true));
    assert_eq!(
        run_with("x or print(1)", &[("x", Value::Int(3))]),
        Value::Int(3)
    );
}

#[test]
fn an_unbound_name_is_a_compile_error_even_where_never_evaluated() {
    for source in [
        "false and nope",
        "true or nope",
        "x and nope",
        "nope or true",
        "(1 == 1) or nope",
    ] {
        let rendered = Script::new(source)
            .capture("x", Value::Null)
            .compile_errors()
            .render_plain();
        assert!(
            rendered.contains("`nope` is not defined"),
            "{source:?}: {rendered}"
        );
    }
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
/// every assignment of operand forms.
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
        let script = captures
            .iter()
            .fold(Script::new(&source), |script, (name, value)| {
                script.capture(name, value.clone())
            });

        let expected = reference(chain, &chosen);
        let actual = script.outcome().map(|finished| finished.tail);
        match (&expected, &actual) {
            (Some(expected), Ok(actual)) if expected == actual => {}
            (None, Err(message)) if message.contains(RAISE_MESSAGE) => {}
            _ => panic!(
                "{source:?} with {captures:?}: expected {expected:?} (None: raises), \
                 got {actual:?}"
            ),
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
        for emitted in emitted(source, folding_only) {
            assert_eq!(jumps(&emitted), 0, "the test is folded away: {emitted:?}");
            assert_eq!(emitted.count(&value), 1, "its value is pushed: {emitted:?}");
        }
        for emitted in emitted(source, neither_folding_nor_eliminating) {
            assert!(
                jumps(&emitted) > 0,
                "with folding off the test stays: {emitted:?}"
            );
        }
    }
}

#[test]
fn a_fold_short_circuits_past_a_raising_operand() {
    // The skipped `1 / 0` never runs in the fold VM either, so the whole
    // expression folds.
    for emitted in emitted(&format!("false and {RAISE}"), folding_only) {
        assert_eq!(jumps(&emitted), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::Divide), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushFalse), 1, "{emitted:?}");
    }
}

#[test]
fn a_fold_that_reaches_a_raising_operand_is_left_for_runtime() {
    for emitted in emitted(&format!("true and {RAISE}"), folding_only) {
        assert_eq!(jumps(&emitted), 1, "the test is kept: {emitted:?}");
        assert_eq!(
            emitted.count(&Bytecode::Divide),
            1,
            "the division is kept: {emitted:?}"
        );
    }
}

#[test]
fn the_constant_side_of_a_runtime_logical_folds() {
    for source in [
        "x and (1 + 2)",
        "(1 + 2) or x",
        "x or (1 + 2)",
        "(1 + 2) and x",
    ] {
        for emitted in emitted(source, folding_only) {
            assert_eq!(jumps(&emitted), 1, "the runtime test stays: {emitted:?}");
            assert_eq!(
                emitted.count(&Bytecode::Add),
                0,
                "the constant side folds: {emitted:?}"
            );
            assert_eq!(emitted.count(&Bytecode::PushInt(3)), 1, "{emitted:?}");
        }
    }
}

#[test]
fn constant_operands_fold_throughout_a_runtime_chain() {
    // `x and (1 + 2)` cannot fold, so the outer `and` folds its constant side.
    for emitted in emitted("x and (1 + 2) and (3 + 4)", folding_only) {
        assert_eq!(jumps(&emitted), 2, "both runtime tests stay: {emitted:?}");
        assert_eq!(
            emitted.count(&Bytecode::Add),
            0,
            "every constant operand folds: {emitted:?}"
        );
        assert_eq!(emitted.count(&Bytecode::PushInt(3)), 1, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(7)), 1, "{emitted:?}");
    }
}

#[test]
fn a_constant_prefix_of_a_runtime_chain_folds_whole() {
    // `null or false` is a constant subtree under the runtime `or x`.
    for emitted in emitted("null or false or x", folding_only) {
        assert_eq!(
            jumps(&emitted),
            1,
            "only the test against `x` stays: {emitted:?}"
        );
        assert_eq!(
            emitted.count(&Bytecode::PushNull),
            0,
            "the constant prefix folds: {emitted:?}"
        );
        assert_eq!(emitted.count(&Bytecode::PushFalse), 1, "{emitted:?}");
    }
}

// --- Branch elimination ---

#[test]
fn a_constant_left_operand_that_decides_is_all_that_remains() {
    for (source, left) in [
        ("false and x", Bytecode::PushFalse),
        ("null and x", Bytecode::PushNull),
        ("true or x", Bytecode::PushTrue),
        ("0 or x", Bytecode::PushInt(0)),
    ] {
        for emitted in emitted(source, eliminating) {
            assert_eq!(jumps(&emitted), 0, "the test is eliminated: {emitted:?}");
            assert!(
                !loads_x(&emitted),
                "the right operand is dropped: {emitted:?}"
            );
            assert_eq!(
                emitted.count(&left),
                1,
                "the left operand remains: {emitted:?}"
            );
        }
    }
}

#[test]
fn a_constant_left_operand_that_defers_leaves_only_the_right() {
    for (source, left) in [
        ("true and x", Bytecode::PushTrue),
        ("0 and x", Bytecode::PushInt(0)),
        ("false or x", Bytecode::PushFalse),
        ("null or x", Bytecode::PushNull),
    ] {
        for emitted in emitted(source, eliminating) {
            assert_eq!(jumps(&emitted), 0, "the test is eliminated: {emitted:?}");
            assert!(loads_x(&emitted), "the right operand remains: {emitted:?}");
            assert_eq!(
                emitted.count(&left),
                0,
                "the left operand is dropped: {emitted:?}"
            );
        }
    }
}

#[test]
fn without_elimination_a_constant_left_operand_is_still_tested() {
    for emitted in emitted("true and x", |optimization| !optimization.branch_eliminate) {
        assert_eq!(jumps(&emitted), 1, "{emitted:?}");
    }
}

#[test]
fn a_right_operand_that_would_raise_is_dropped_unevaluated() {
    // No folding is needed: the literal `false` decides alone.
    for emitted in emitted(&format!("false and {RAISE}"), eliminating_only) {
        assert_eq!(jumps(&emitted), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::Divide), 0, "{emitted:?}");
    }
}

#[test]
fn a_right_operand_that_raises_is_kept_for_runtime() {
    // The fold of the whole raises, so folding keeps the test; elimination
    // still removes it, leaving just the raising operand.
    for emitted in emitted(&format!("true and {RAISE}"), eliminating) {
        assert_eq!(jumps(&emitted), 0, "the test is eliminated: {emitted:?}");
        assert_eq!(
            emitted.count(&Bytecode::Divide),
            1,
            "the division remains, to raise at runtime: {emitted:?}"
        );
    }
}

#[test]
fn a_kept_right_operand_is_still_folded() {
    for emitted in emitted("true and (1 + 2)", eliminating_only) {
        assert_eq!(jumps(&emitted), 0, "{emitted:?}");
        assert_eq!(
            emitted.count(&Bytecode::Add),
            1,
            "folding is off: {emitted:?}"
        );
    }
    for emitted in emitted("true and (1 + 2)", |optimization| {
        optimization.branch_eliminate && optimization.constant_fold
    }) {
        assert_eq!(jumps(&emitted), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::Add), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(3)), 1, "{emitted:?}");
    }
}

#[test]
fn a_folded_left_operand_decides() {
    // A computed left operand beside a runtime right one folds as a sibling,
    // which makes it a known constant.
    let eliminating_and_folding = |optimization: &OptimizationOptions| {
        optimization.branch_eliminate && optimization.constant_fold
    };
    for source in ["(1 == 1) and x", "(1 == 2) or x"] {
        for emitted in emitted(source, eliminating_and_folding) {
            assert_eq!(jumps(&emitted), 0, "the test is eliminated: {emitted:?}");
            assert!(loads_x(&emitted), "only `x` remains: {emitted:?}");
        }
    }
    for source in ["(1 == 2) and x", "(1 == 1) or x"] {
        for emitted in emitted(source, eliminating_and_folding) {
            assert_eq!(jumps(&emitted), 0, "the test is eliminated: {emitted:?}");
            assert!(!loads_x(&emitted), "`x` is dropped: {emitted:?}");
        }
    }
    // Unfolded, a computed operand is not a known constant.
    for emitted in emitted("(1 == 1) and x", eliminating_only) {
        assert_eq!(jumps(&emitted), 1, "{emitted:?}");
    }
}

#[test]
fn a_propagated_left_operand_decides() {
    let source = r"
        def t = true
        t and x
    ";
    let emitted = Script::new(source)
        .capture("x", Value::Null)
        .code_where(|optimization| {
            optimization.branch_eliminate && optimization.constant_propagate
        });
    for emitted in emitted {
        assert_eq!(jumps(&emitted), 0, "the test is eliminated: {emitted:?}");
    }
}

#[test]
fn elimination_cascades_through_a_chain() {
    // Each eliminated test leaves a constant, which decides the next one.
    for source in [
        "true and true and x",
        "null or false or x",
        "true and 0 and x",
    ] {
        for emitted in emitted(source, eliminating) {
            assert_eq!(jumps(&emitted), 0, "every test is eliminated: {emitted:?}");
            assert!(loads_x(&emitted), "only `x` remains: {emitted:?}");
        }
    }
    for source in ["false and true and x", "1 or null or x"] {
        for emitted in emitted(source, eliminating) {
            assert_eq!(jumps(&emitted), 0, "every test is eliminated: {emitted:?}");
            assert!(!loads_x(&emitted), "`x` is dropped: {emitted:?}");
        }
    }
}

#[test]
fn only_tests_with_a_constant_left_operand_are_eliminated() {
    // The outer test's left operand is the runtime `x`, so it stays; the inner
    // one is eliminated.
    for emitted in emitted("x and (true and x)", eliminating) {
        assert_eq!(jumps(&emitted), 1, "{emitted:?}");
    }
    // A runtime left operand is never eliminated, even when the right is constant.
    for emitted in emitted("x or true", eliminating) {
        assert_eq!(jumps(&emitted), 1, "{emitted:?}");
    }
}

// The remaining code-shape tests pin exactly the options they are about.

const ELIMINATE: OptimizationOptions = UNOPTIMIZED.with(Optimization::BranchEliminate, true);

/// The code of `source`, with `x` a runtime-only capture, under exactly
/// `optimization`.
fn code(source: &str, optimization: OptimizationOptions) -> Emitted {
    Script::new(source)
        .capture("x", Value::Null)
        .code(optimization)
}

#[test]
fn a_constant_string_left_operand_decides() {
    // A String is a pooled constant, not an inline push; it is known all the same.
    let emitted = code(r#""" or x"#, ELIMINATE);
    assert_eq!(jumps(&emitted), 0, "{emitted:?}");
    assert!(!loads_x(&emitted), "`x` is dropped: {emitted:?}");
    let emitted = code(r#""" and x"#, ELIMINATE);
    assert_eq!(jumps(&emitted), 0, "{emitted:?}");
    assert!(loads_x(&emitted), "only `x` remains: {emitted:?}");
}

#[test]
fn an_empty_structure_left_operand_decides() {
    // An empty literal is known without folding: nothing goes into it.
    for left in ["[]", "{}"] {
        let emitted = code(&format!("{left} or x"), ELIMINATE);
        assert_eq!(jumps(&emitted), 0, "{left}: {emitted:?}");
        assert!(!loads_x(&emitted), "`x` is dropped: {emitted:?}");
        let emitted = code(&format!("{left} and x"), ELIMINATE);
        assert_eq!(jumps(&emitted), 0, "{left}: {emitted:?}");
        assert!(loads_x(&emitted), "only `x` remains: {emitted:?}");
    }
}

#[test]
fn a_folded_constant_subtree_decides_the_test_above_it() {
    // `(1 == 1) and 2` is constant but not a single op until folded; beside the
    // runtime `x`, it folds to `2`, which then decides the `or`.
    let emitted = code(
        "((1 == 1) and 2) or x",
        ELIMINATE.with(Optimization::ConstantFold, true),
    );
    assert_eq!(jumps(&emitted), 0, "{emitted:?}");
    assert!(!loads_x(&emitted), "`x` is dropped: {emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(2)), 1, "{emitted:?}");
}

#[test]
fn an_effectful_right_operand_is_dropped_when_the_left_decides() {
    let emitted = code("false and print(x)", ELIMINATE);
    assert_eq!(jumps(&emitted), 0, "{emitted:?}");
    assert!(
        !emitted
            .code
            .iter()
            .any(|op| matches!(op, Bytecode::LoadGlobal(_) | Bytecode::Call(_))),
        "the call to `print` is gone: {emitted:?}"
    );
}

#[test]
fn an_effectful_right_operand_keeps_the_test_under_folding() {
    // Folding cannot evaluate a call to `print`, so the test stays for runtime.
    let emitted = code(
        "false and print(1)",
        UNOPTIMIZED.with(Optimization::ConstantFold, true),
    );
    assert_eq!(jumps(&emitted), 1, "{emitted:?}");
}

#[test]
fn a_propagated_left_operand_decides_inside_a_block() {
    let propagate_and_eliminate = ELIMINATE.with(Optimization::ConstantPropagate, true);
    let null_constant = r"
        do {
            def t = null
            t or x
        }
    ";
    let emitted = code(null_constant, propagate_and_eliminate);
    assert_eq!(jumps(&emitted), 0, "{emitted:?}");
    assert!(loads_x(&emitted), "only `x` remains: {emitted:?}");
    let truthy_constant = r"
        do {
            def t = 0
            t or x
        }
    ";
    let emitted = code(truthy_constant, propagate_and_eliminate);
    assert_eq!(jumps(&emitted), 0, "{emitted:?}");
    assert!(!loads_x(&emitted), "`x` is dropped: {emitted:?}");
}

#[test]
fn a_runtime_shadow_of_a_constant_keeps_the_test() {
    let emitted = code(
        r"
        def t = true
        do {
            def t = x
            t and 1
        }
        ",
        ELIMINATE.with(Optimization::ConstantPropagate, true),
    );
    assert_eq!(jumps(&emitted), 1, "the inner `t` is runtime: {emitted:?}");
}

#[test]
fn elimination_applies_inside_a_lambda() {
    let body = code("fn -> false and x", ELIMINATE).nested(0);
    assert_eq!(jumps(&body), 0, "{body:?}");
    assert!(!loads_x(&body), "`x` is dropped: {body:?}");
    let body = code("fn -> true and x", ELIMINATE).nested(0);
    assert_eq!(jumps(&body), 0, "{body:?}");
    assert!(loads_x(&body), "only `x` remains: {body:?}");
}

#[test]
fn a_hoisted_constant_decides_inside_a_lambda() {
    let emitted = code(
        r"
        def t = false
        fn -> t and x
        ",
        ELIMINATE
            .with(Optimization::ConstantPropagate, true)
            .with(Optimization::CaptureHoist, true),
    );
    let body = emitted.nested(0);
    assert_eq!(jumps(&body), 0, "{body:?}");
    assert!(!loads_x(&body), "`x` is dropped: {body:?}");
}
