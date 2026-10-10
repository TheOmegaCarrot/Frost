//! `if` / `elif` / `else` lowering, end to end: compile full source, run it on
//! the VM, and check the result.
//!
//! Only `null` and `false` are falsy. Conditions are evaluated in order until one
//! is truthy, and only the taken branch is evaluated; with no `else`, a false
//! chain yields `null`.
//!
//! An operand's form decides which lowering path it exercises: a literal is
//! foldable, a capture is known only at runtime, and `(1 / 0)` raises if it is
//! ever evaluated, making evaluation order observable; the evaluation-order tests
//! also log each part evaluated to a mutable cell. The chain tests run every
//! mix of these forms through the conditions and branches, against a reference
//! evaluator. The harness runs every behavioral case under every optimization
//! permutation.

use crate::script;

use frostlang::Value;
use frostlang::bytecode::Bytecode;
use frostlang::compile::{Optimization, OptimizationOptions};
use script::{Emitted, Script, UNOPTIMIZED, compile_errors, raises, run};

/// The operand that raises when evaluated, and the message it raises.
const RAISE: &str = "(1 / 0)";
const RAISE_MESSAGE: &str = "Division by zero";

/// The value of `source` with `captures` in scope, which must run.
fn run_with(source: &str, captures: &[(&str, Value)]) -> Value {
    Script::new(source).captures(captures).run()
}

/// Assert that `source` raises the [`RAISE`] operand's error.
fn assert_raises(source: &str) {
    let message = raises(source);
    assert!(
        message.contains(RAISE_MESSAGE),
        "{source:?} raised the wrong error: {message}"
    );
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

/// Neither folding nor branch elimination: every branch is emitted.
fn neither_folding_nor_eliminating(optimization: &OptimizationOptions) -> bool {
    !optimization.constant_fold && !optimization.branch_eliminate
}

/// How many jumps (conditional or not) remain in the code.
fn jumps(emitted: &Emitted) -> usize {
    emitted
        .code
        .iter()
        .filter(|op| {
            matches!(
                op,
                Bytecode::Jump(_) | Bytecode::JumpIfFalse(_) | Bytecode::JumpIfTrue(_)
            )
        })
        .count()
}

// --- Choosing a branch ---

#[test]
fn a_truthy_condition_takes_the_consequent() {
    for truthy in ["true", "0", "0.0", r#""""#, "x''", "len"] {
        assert_eq!(
            run(&format!("if {truthy}: 1 else: 2")),
            Value::Int(1),
            "{truthy} is truthy"
        );
    }
}

#[test]
fn a_falsy_condition_takes_the_alternate() {
    for falsy in ["null", "false"] {
        assert_eq!(
            run(&format!("if {falsy}: 1 else: 2")),
            Value::Int(2),
            "{falsy} is falsy"
        );
    }
}

#[test]
fn a_runtime_condition_chooses_at_runtime() {
    for (x, expected) in [
        (Value::Null, 2),
        (Value::Bool(false), 2),
        (Value::Bool(true), 1),
        (Value::Int(0), 1),
    ] {
        assert_eq!(
            run_with("if x: 1 else: 2", &[("x", x.clone())]),
            Value::Int(expected),
            "x = {x:?}"
        );
    }
}

#[test]
fn without_else_a_false_condition_yields_null() {
    assert_eq!(run("if false: 1"), Value::Null);
    assert_eq!(run("if null: 1"), Value::Null);
    assert_eq!(run("if true: 1"), Value::Int(1));
    assert_eq!(run_with("if x: 1", &[("x", Value::Null)]), Value::Null);
    assert_eq!(
        run_with("if x: 1", &[("x", Value::Bool(true))]),
        Value::Int(1)
    );
}

#[test]
fn the_branch_value_is_not_coerced() {
    assert_eq!(run(r#"if true: "yes" else: 2"#), Value::from("yes"));
    assert_eq!(run("if false: 1 else: null"), Value::Null);
    assert_eq!(run("if true: false else: true"), Value::Bool(false));
}

#[test]
fn elif_takes_the_first_truthy_condition() {
    let source = "if a: 1 elif b: 2 else: 3";
    for (a, b, expected) in [
        (true, true, 1),
        (true, false, 1),
        (false, true, 2),
        (false, false, 3),
    ] {
        assert_eq!(
            run_with(source, &[("a", Value::Bool(a)), ("b", Value::Bool(b))]),
            Value::Int(expected),
            "a = {a}, b = {b}"
        );
    }
}

#[test]
fn an_elif_chain_without_else_yields_null_when_all_fail() {
    let source = "if a: 1 elif b: 2";
    let captures = [("a", Value::Null), ("b", Value::Bool(false))];
    assert_eq!(run_with(source, &captures), Value::Null);
}

#[test]
fn structures_functions_and_negative_numbers_are_truthy_conditions() {
    for truthy in [
        "[]",
        "{}",
        "[null]",
        "{a: false}",
        "-1",
        "(fn -> null)",
        "$(null)",
    ] {
        assert_eq!(
            run(&format!("if {truthy}: 1 else: 2")),
            Value::Int(1),
            "{truthy} is truthy"
        );
    }
}

#[test]
fn a_long_elif_chain_takes_the_first_truthy_condition() {
    let chain = r#"if n == 0: "zero"
        elif n == 1: "one"
        elif n == 2: "two"
        elif n == 3: "three"
        elif n == 4: "four""#;
    let with_else = format!(r#"{chain} else: "many""#);
    for (n, expected) in [
        (0, "zero"),
        (1, "one"),
        (2, "two"),
        (3, "three"),
        (4, "four"),
        (5, "many"),
        (-1, "many"),
    ] {
        assert_eq!(
            run_with(&with_else, &[("n", Value::Int(n))]),
            Value::from(expected),
            "n = {n}"
        );
    }
    assert_eq!(
        run_with(chain, &[("n", Value::Int(5))]),
        Value::Null,
        "without else, no match is null"
    );
}

#[test]
fn every_truthy_elif_after_the_first_is_passed_over() {
    let source = "if a: 1 elif a: 2 elif a: 3 else: 4";
    assert_eq!(run_with(source, &[("a", Value::Int(0))]), Value::Int(1));
    assert_eq!(run_with(source, &[("a", Value::Null)]), Value::Int(4));
}

// --- Evaluation ---

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
fn conditions_are_evaluated_once_each_in_order_until_one_is_truthy() {
    for (statement, expected) in [
        (
            "if note(false): note(1) elif note(null): note(2) elif note(0): note(3) else: note(4)",
            "[false, null, 0, 3]",
        ),
        (
            "if note(false): note(1) elif note(null): note(2) else: note(4)",
            "[false, null, 4]",
        ),
        (
            "if note(false): note(1) elif note(null): note(2)",
            "[false, null]",
        ),
        (
            "if note(true): note(1) elif note(true): note(2)",
            "[true, 1]",
        ),
        // A nested `if` in the condition runs fully before the outer test.
        (
            "if (if note(null): note(1) else: note(2)): note(3) else: note(4)",
            "[null, 2, 3]",
        ),
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
            "{statement:?} evaluates the noted parts in order"
        );
    }
}

#[test]
fn the_taken_branchs_value_follows_its_conditions_evaluation() {
    let source = format!(
        r"
        {NOTE}
        [if note(null): note(1) elif note(2): note(3), log()]
        "
    );
    assert_eq!(run(&source), run("[3, [null, 2, 3]]"));
}

#[test]
fn only_the_taken_branch_is_evaluated() {
    assert_eq!(run(&format!("if true: 1 else: {RAISE}")), Value::Int(1));
    assert_eq!(run(&format!("if false: {RAISE} else: 2")), Value::Int(2));
    assert_eq!(run(&format!("if false: {RAISE}")), Value::Null);
    assert_raises(&format!("if true: {RAISE} else: 2"));
    assert_raises(&format!("if false: 1 else: {RAISE}"));
}

#[test]
fn conditions_after_a_truthy_one_are_not_evaluated() {
    assert_eq!(
        run(&format!("if true: 1 elif {RAISE}: 2 else: 3")),
        Value::Int(1)
    );
    assert_raises(&format!("if false: 1 elif {RAISE}: 2 else: 3"));
}

#[test]
fn a_raising_condition_raises() {
    assert_raises(&format!("if {RAISE}: 1 else: 2"));
}

// --- Composition ---

#[test]
fn an_if_is_an_expression() {
    assert_eq!(run("(if true: 1 else: 2) + 10"), Value::Int(11));
    assert_eq!(run("10 + (if false: 1 else: 2)"), Value::Int(12));
    let source = r"
        def y = if true: 1 else: 2
        y * 3
    ";
    assert_eq!(run(source), Value::Int(3));
}

#[test]
fn ifs_nest() {
    let source = "if a: (if b: 1 else: 2) else: (if b: 3 else: 4)";
    for (a, b, expected) in [
        (true, true, 1),
        (true, false, 2),
        (false, true, 3),
        (false, false, 4),
    ] {
        assert_eq!(
            run_with(source, &[("a", Value::Bool(a)), ("b", Value::Bool(b))]),
            Value::Int(expected),
            "a = {a}, b = {b}"
        );
    }
}

#[test]
fn a_condition_may_be_an_if() {
    let source = "if (if x: false else: true): 1 else: 2";
    assert_eq!(run_with(source, &[("x", Value::Bool(true))]), Value::Int(2));
    assert_eq!(run_with(source, &[("x", Value::Null)]), Value::Int(1));
}

#[test]
fn a_condition_may_be_a_logical() {
    let source = "if a and not b: 1 else: 2";
    for (a, b, expected) in [
        (true, false, 1),
        (true, true, 2),
        (false, false, 2),
        (false, true, 2),
    ] {
        assert_eq!(
            run_with(source, &[("a", Value::Bool(a)), ("b", Value::Bool(b))]),
            Value::Int(expected),
            "a = {a}, b = {b}"
        );
    }
}

#[test]
fn an_if_statement_leaves_the_stack_balanced() {
    // A non-tail `if` is evaluated and dropped, whichever branch it takes; the
    // tail value must be unaffected.
    for condition in [Value::Bool(true), Value::Bool(false)] {
        for source in [
            r"
            if x: 1 else: 2
            5
            ",
            r"
            if x: 1
            5
            ",
            r"
            if x: 1 elif x: 2 else: 3
            if x: 4
            5
            ",
        ] {
            assert_eq!(
                run_with(source, &[("x", condition.clone())]),
                Value::Int(5),
                "{source:?} with x = {condition:?}"
            );
        }
    }
}

#[test]
fn an_if_leaves_exactly_its_value_among_others() {
    // Branches of different code shapes, mid-expression: whichever runs must
    // leave one value on top of those already on the stack.
    for (source, if_false, if_true) in [
        (
            r"
            [0, if x: do {
                def a = 1
                a + 1
            } else: [1, 2], 9]
            ",
            "[0, [1, 2], 9]",
            "[0, 2, 9]",
        ),
        (
            "[if x: 1, if x: 2 elif not x: 3, 4]",
            "[null, 3, 4]",
            "[1, 2, 4]",
        ),
        ("10 + (if x: 1 else: 2) * 3", "16", "13"),
        (
            r#"{[if x: "a" else: "b"]: if x: x or 1 else: x and 2}"#,
            "{b: false}",
            "{a: true}",
        ),
        (r#"$'<${if x: "yes" else: "no"}>'"#, "'<no>'", "'<yes>'"),
    ] {
        assert_eq!(
            run_with(source, &[("x", Value::Bool(false))]),
            run(if_false),
            "{source:?} with x = false"
        );
        assert_eq!(
            run_with(source, &[("x", Value::Bool(true))]),
            run(if_true),
            "{source:?} with x = true"
        );
    }
}

#[test]
fn an_if_statement_with_differently_shaped_branches_leaves_the_stack_balanced() {
    for condition in [Value::Bool(true), Value::Bool(false)] {
        for source in [
            r"
            if x: do {
                def a = 1
                a
            } else: [1, 2]
            5
            ",
            r"
            if x: (x and 1) else: (if x: 2 else: 3)
            5
            ",
            r"
            if x: fn -> 1 else: {a: 2}
            5
            ",
            r"
            if true: x
            if false: x
            if null: x else: x
            5
            ",
        ] {
            assert_eq!(
                run_with(source, &[("x", condition.clone())]),
                Value::Int(5),
                "{source:?} with x = {condition:?}"
            );
        }
    }
}

#[test]
fn a_condition_may_be_a_block() {
    let source = r"
        if do {
            def y = x
            not y
        }: 1 else: 2
    ";
    assert_eq!(run_with(source, &[("x", Value::Bool(true))]), Value::Int(2));
    assert_eq!(run_with(source, &[("x", Value::Null)]), Value::Int(1));
}

#[test]
fn an_if_in_a_lambda_chooses_on_each_call() {
    let sign = r"
        def sign = fn n -> if n < 0: -1 elif n == 0: 0 else: 1
        [sign(-5), sign(0), sign(5)]
    ";
    assert_eq!(run(sign), run("[-1, 0, 1]"));
    let define = format!("def f = fn n -> if n: n else: {RAISE}");
    let call_with = |argument: &str| {
        format!(
            r"
            {define}
            f({argument})
            "
        )
    };
    assert_eq!(run(&call_with("4")), Value::Int(4));
    assert_raises(&call_with("null"));
}

// --- Scope ---

#[test]
fn each_branch_block_is_its_own_scope() {
    let source = r"
        if x: do {
            def y = 1
            y
        } else: do {
            def y = 2
            y + 10
        }
    ";
    assert_eq!(run_with(source, &[("x", Value::Bool(true))]), Value::Int(1));
    assert_eq!(run_with(source, &[("x", Value::Null)]), Value::Int(12));
    // A branch's binding shadows the enclosing one only inside its block.
    let source = r"
        def y = 5
        (if x: do {
            def y = 1
            y
        } else: y) + y
    ";
    assert_eq!(run_with(source, &[("x", Value::Bool(true))]), Value::Int(6));
    assert_eq!(run_with(source, &[("x", Value::Null)]), Value::Int(10));
}

#[test]
fn a_branch_binding_is_not_visible_after_the_if() {
    let source = r"
        if x: do {
            def y = 1
            y
        } else: 2
        y
    ";
    let rendered = Script::new(source)
        .capture("x", Value::Null)
        .compile_errors()
        .render_plain();
    assert!(rendered.contains("`y` is not defined"), "{rendered}");
}

#[test]
fn a_branch_block_rejects_a_duplicate_binding() {
    let source = r"
        if x: do {
            def y = 1
            def y = 2
            y
        } else: 0
    ";
    let rendered = Script::new(source)
        .capture("x", Value::Null)
        .compile_errors()
        .render_plain();
    assert!(rendered.contains("`y` is already bound"), "{rendered}");
}

#[test]
fn an_unbound_name_is_a_compile_error_even_where_never_evaluated() {
    for source in [
        "if false: nope else: 1",
        "if true: 1 else: nope",
        "if true: 1 elif nope: 2",
        "if true: 1 elif false: nope",
        "if nope: 1",
        "if 1 == 1: 1 else: nope",
    ] {
        let rendered = compile_errors(source).render_plain();
        assert!(
            rendered.contains("`nope` is not defined"),
            "{source:?}: {rendered}"
        );
    }
}

#[test]
fn a_runtime_binding_shadowing_a_constant_decides_at_runtime() {
    // The inner `c` is the runtime `x`, not the outer constant `true`.
    let source = r"
        def c = true
        do {
            def c = x
            if c: 1 else: 2
        }
    ";
    assert_eq!(
        run_with(source, &[("x", Value::Bool(false))]),
        Value::Int(2)
    );
    assert_eq!(run_with(source, &[("x", Value::Int(0))]), Value::Int(1));
    // And the reverse: a constant inner `c` shadows the runtime outer one.
    let source = r"
        def c = x
        do {
            def c = false
            if c: 1 else: 2
        }
    ";
    assert_eq!(run_with(source, &[("x", Value::Bool(true))]), Value::Int(2));
    // Once the block ends, the outer constant is back in view.
    let source = r"
        def c = true
        def d = do {
            def c = false
            if c: 1 else: 2
        }
        if c: d else: 3
    ";
    assert_eq!(run(source), Value::Int(2));
}

#[test]
fn a_constant_captured_by_a_lambda_decides_inside_it() {
    let falsy = r"
        def c = false
        def f = fn -> if c: 1 else: 2
        f()
    ";
    assert_eq!(run(falsy), Value::Int(2));
    let truthy = r"
        def c = 0
        def f = fn n -> if c: n elif n: 1 else: 2
        f(7)
    ";
    assert_eq!(run(truthy), Value::Int(7));
}

#[test]
fn an_effectful_branch_is_skipped_when_not_taken() {
    // `print` would be called if its branch ran.
    assert_eq!(run("if false: print(1) else: 2"), Value::Int(2));
    assert_eq!(run("if true: 1 else: print(2)"), Value::Int(1));
    assert_eq!(
        run_with("if x: print(1)", &[("x", Value::Null)]),
        Value::Null
    );
}

// --- Chains: every operand form in every position ---

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

impl Form {
    /// The form's value, or `None` if evaluating it raises.
    fn value(&self) -> Option<Value> {
        match self {
            Form::Literal(value) | Form::Capture(value) => Some(value.clone()),
            Form::Raise => None,
        }
    }
}

/// Condition forms: both truthinesses, each foldable and runtime-only, and the
/// raising operand.
fn condition_forms() -> Vec<Form> {
    vec![
        Form::Literal(Value::Bool(false)),
        Form::Literal(Value::Bool(true)),
        Form::Capture(Value::Null),
        Form::Capture(Value::Int(0)),
        Form::Raise,
    ]
}

/// Branch forms for the branch at `index`, whose value is distinct from every
/// other branch's so the taken branch is identifiable.
fn branch_forms(index: i64) -> Vec<Form> {
    vec![
        Form::Literal(Value::Int(index)),
        Form::Capture(Value::Int(index)),
        Form::Raise,
    ]
}

/// Frost source for a literal value.
fn literal_source(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        other => unreachable!("no literal source for sample {other:?}"),
    }
}

/// Every combination of one form per slot, where `choices[i]` lists slot i's forms.
fn every_combination(choices: &[Vec<Form>]) -> Vec<Vec<Form>> {
    choices
        .iter()
        .fold(vec![Vec::new()], |combinations, forms| {
            combinations
                .iter()
                .flat_map(|prefix| {
                    forms.iter().map(move |form| {
                        let mut combination = prefix.clone();
                        combination.push(form.clone());
                        combination
                    })
                })
                .collect()
        })
}

/// The expected outcome of an if chain: `None` if an evaluated operand raises.
fn reference(conditions: &[Form], branches: &[Form], alternate: Option<&Form>) -> Option<Value> {
    let truthy = |value: &Value| !matches!(value, Value::Null | Value::Bool(false));
    for (condition, branch) in conditions.iter().zip(branches) {
        if truthy(&condition.value()?) {
            return branch.value();
        }
    }
    alternate.map_or(Some(Value::Null), Form::value)
}

/// Check the chain `if c0: b0 elif c1: b1 ... [else: e]` for every assignment of
/// forms to its conditions and branches.
fn check_chain(conditions: usize, with_else: bool) {
    let mut slots: Vec<Vec<Form>> = Vec::new();
    for index in 0..conditions {
        slots.push(condition_forms());
        slots.push(branch_forms(index as i64));
    }
    if with_else {
        slots.push(branch_forms(conditions as i64));
    }

    for forms in every_combination(&slots) {
        let mut source = String::new();
        let mut script_captures = Vec::new();
        for (slot, form) in forms.iter().enumerate() {
            let text = match form {
                Form::Literal(value) => literal_source(value),
                Form::Capture(value) => {
                    let name = format!("s{slot}");
                    script_captures.push((name.clone(), value.clone()));
                    name
                }
                Form::Raise => RAISE.to_string(),
            };
            let keyword = match (slot / 2, slot % 2) {
                (0, 0) => "if ",
                (_, 0) if slot / 2 < conditions => " elif ",
                (_, 0) => " else: ",
                _ => ": ",
            };
            source.push_str(keyword);
            source.push_str(&text);
        }

        let conditions_forms: Vec<Form> =
            forms.iter().step_by(2).take(conditions).cloned().collect();
        let branch_forms: Vec<Form> = forms.iter().skip(1).step_by(2).cloned().collect();
        let alternate = with_else.then(|| &forms[forms.len() - 1]);
        let expected = reference(&conditions_forms, &branch_forms, alternate);

        let script = script_captures
            .iter()
            .fold(Script::new(&source), |script, (name, value)| {
                script.capture(name, value.clone())
            });
        let actual = script.outcome().map(|finished| finished.tail);
        match (&expected, &actual) {
            (Some(expected), Ok(actual)) if expected == actual => {}
            (None, Err(message)) if message.contains(RAISE_MESSAGE) => {}
            _ => panic!(
                "{source:?} with {script_captures:?}: expected {expected:?} (None: raises), \
                 got {actual:?}"
            ),
        }
    }
}

#[test]
fn every_form_through_if_else() {
    check_chain(1, true);
}

#[test]
fn every_form_through_if_without_else() {
    check_chain(1, false);
}

#[test]
fn every_form_through_if_elif_else() {
    check_chain(2, true);
}

#[test]
fn every_form_through_if_elif_without_else() {
    check_chain(2, false);
}

// --- Constant folding ---

#[test]
fn a_constant_if_folds_to_the_taken_value() {
    for (source, value) in [
        ("if true: 1 else: 2", Bytecode::PushInt(1)),
        ("if null: 1 else: 2", Bytecode::PushInt(2)),
        ("if false: 1", Bytecode::PushNull),
        ("if false: 1 elif 0: 2 else: 3", Bytecode::PushInt(2)),
    ] {
        for emitted in emitted(source, folding_only) {
            assert_eq!(jumps(&emitted), 0, "folded away: {emitted:?}");
            assert_eq!(emitted.count(&value), 1, "the taken value: {emitted:?}");
        }
        for emitted in emitted(source, neither_folding_nor_eliminating) {
            assert!(jumps(&emitted) > 0, "unfolded, the test stays: {emitted:?}");
        }
    }
}

#[test]
fn a_fold_skips_an_untaken_raising_branch() {
    for source in [
        format!("if true: 1 else: {RAISE}"),
        format!("if false: {RAISE} else: 2"),
    ] {
        for emitted in emitted(&source, folding_only) {
            assert_eq!(jumps(&emitted), 0, "{emitted:?}");
            assert_eq!(emitted.count(&Bytecode::Divide), 0, "{emitted:?}");
        }
    }
}

#[test]
fn a_fold_that_takes_a_raising_branch_is_left_for_runtime() {
    for emitted in emitted(&format!("if true: {RAISE} else: 2"), folding_only) {
        assert!(jumps(&emitted) > 0, "the test is kept: {emitted:?}");
        assert_eq!(emitted.count(&Bytecode::Divide), 1, "{emitted:?}");
    }
}

#[test]
fn constant_branches_of_a_runtime_condition_fold() {
    for emitted in emitted("if x: 1 + 2 else: 3 * 4", folding_only) {
        assert_eq!(jumps(&emitted), 2, "the runtime test stays: {emitted:?}");
        assert_eq!(emitted.count(&Bytecode::Add), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(3)), 1, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(12)), 1, "{emitted:?}");
    }
}

#[test]
fn a_constant_condition_of_a_runtime_branch_folds() {
    for emitted in emitted("if 1 == 1: x else: 2", folding_only) {
        assert_eq!(emitted.count(&Bytecode::CompareEqual), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushTrue), 1, "{emitted:?}");
    }
}

#[test]
fn a_constant_if_folds_as_a_sibling() {
    for emitted in emitted("(if true: 2 else: 3) * x", folding_only) {
        assert_eq!(jumps(&emitted), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(2)), 1, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::Multiply), 1, "{emitted:?}");
    }
}

// --- Branch elimination ---
//
// Each test pins exactly the options it is about, so no other optimization
// changes the code it inspects.

const ELIMINATE: OptimizationOptions = UNOPTIMIZED.with(Optimization::BranchEliminate, true);

/// The code of `source`, with `x` a runtime-only capture, under exactly
/// `optimization`.
fn code(source: &str, optimization: OptimizationOptions) -> Emitted {
    Script::new(source)
        .capture("x", Value::Null)
        .code(optimization)
}

/// Whether the code reads the runtime-only `x`.
fn loads_x(emitted: &Emitted) -> bool {
    emitted
        .code
        .iter()
        .any(|op| matches!(op, Bytecode::LoadLocal(_)))
}

#[test]
fn a_truthy_constant_condition_leaves_only_the_consequent() {
    for (source, condition) in [
        ("if true: x else: 2", Bytecode::PushTrue),
        ("if 0: x else: 2", Bytecode::PushInt(0)),
    ] {
        let emitted = code(source, ELIMINATE);
        assert_eq!(jumps(&emitted), 0, "the test is eliminated: {emitted:?}");
        assert!(loads_x(&emitted), "the consequent remains: {emitted:?}");
        assert_eq!(
            emitted.count(&condition),
            0,
            "the condition is dropped: {emitted:?}"
        );
        assert_eq!(
            emitted.count(&Bytecode::PushInt(2)),
            0,
            "the alternate is dropped: {emitted:?}"
        );
    }
}

#[test]
fn a_falsy_constant_condition_leaves_only_the_alternate() {
    for (source, condition) in [
        ("if false: 1 else: x", Bytecode::PushFalse),
        ("if null: 1 else: x", Bytecode::PushNull),
    ] {
        let emitted = code(source, ELIMINATE);
        assert_eq!(jumps(&emitted), 0, "the test is eliminated: {emitted:?}");
        assert!(loads_x(&emitted), "the alternate remains: {emitted:?}");
        assert_eq!(
            emitted.count(&condition),
            0,
            "the condition is dropped: {emitted:?}"
        );
        assert_eq!(
            emitted.count(&Bytecode::PushInt(1)),
            0,
            "the consequent is dropped: {emitted:?}"
        );
    }
}

#[test]
fn a_falsy_constant_condition_without_else_leaves_null() {
    let emitted = code("if false: x", ELIMINATE);
    assert_eq!(jumps(&emitted), 0, "{emitted:?}");
    assert!(!loads_x(&emitted), "the consequent is dropped: {emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushFalse), 0, "{emitted:?}");
    assert_eq!(
        emitted.count(&Bytecode::PushNull),
        1,
        "the implicit null: {emitted:?}"
    );
}

#[test]
fn elimination_needs_no_folding() {
    // Both branches are constant, but folding is off: elimination alone picks one.
    let emitted = code("if true: 1 else: 2", ELIMINATE);
    assert_eq!(jumps(&emitted), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(1)), 1, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(2)), 0, "{emitted:?}");
}

#[test]
fn a_discarded_raising_branch_is_gone() {
    for source in [
        format!("if true: x else: {RAISE}"),
        format!("if false: {RAISE} else: x"),
        format!("if false: {RAISE}"),
    ] {
        let emitted = code(&source, ELIMINATE);
        assert_eq!(jumps(&emitted), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::Divide), 0, "{emitted:?}");
    }
}

#[test]
fn a_taken_raising_branch_is_kept_for_runtime() {
    let emitted = code(&format!("if true: {RAISE} else: x"), ELIMINATE);
    assert_eq!(jumps(&emitted), 0, "the test is eliminated: {emitted:?}");
    assert_eq!(
        emitted.count(&Bytecode::Divide),
        1,
        "the division remains, to raise at runtime: {emitted:?}"
    );
    assert!(!loads_x(&emitted), "the alternate is dropped: {emitted:?}");
}

#[test]
fn a_folded_condition_decides() {
    // A computed condition beside a runtime branch folds as a sibling, which
    // makes it a known constant.
    let eliminate_and_fold = ELIMINATE.with(Optimization::ConstantFold, true);
    for (source, keeps_x) in [
        ("if 1 == 1: x else: 2", true),
        ("if 1 == 2: x else: 2", false),
    ] {
        let emitted = code(source, eliminate_and_fold);
        assert_eq!(jumps(&emitted), 0, "the test is eliminated: {emitted:?}");
        assert_eq!(emitted.count(&Bytecode::CompareEqual), 0, "{emitted:?}");
        assert_eq!(loads_x(&emitted), keeps_x, "{emitted:?}");
    }
}

#[test]
fn a_propagated_condition_decides() {
    let eliminate_and_propagate = ELIMINATE.with(Optimization::ConstantPropagate, true);
    for (source, keeps_x) in [
        (
            r"
            def c = true
            if c: x else: 2
            ",
            true,
        ),
        (
            r"
            def c = null
            if c: x else: 2
            ",
            false,
        ),
    ] {
        let emitted = code(source, eliminate_and_propagate);
        assert_eq!(jumps(&emitted), 0, "the test is eliminated: {emitted:?}");
        assert_eq!(loads_x(&emitted), keeps_x, "{emitted:?}");
    }
}

#[test]
fn elimination_cascades_through_elif() {
    // An `elif` is an `if` nested in the alternate, so it is eliminated first and
    // its result is what the outer test chooses between.
    let emitted = code("if false: 1 elif true: x else: 3", ELIMINATE);
    assert_eq!(jumps(&emitted), 0, "every test is eliminated: {emitted:?}");
    assert!(loads_x(&emitted), "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(1)), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(3)), 0, "{emitted:?}");

    let emitted = code("if false: 1 elif false: 2", ELIMINATE);
    assert_eq!(jumps(&emitted), 0, "every test is eliminated: {emitted:?}");
    assert_eq!(
        emitted.count(&Bytecode::PushNull),
        1,
        "the implicit null: {emitted:?}"
    );
}

#[test]
fn a_constant_elif_under_a_runtime_condition_is_eliminated() {
    // The outer test depends on `x` and stays; the inner `elif true` does not.
    let emitted = code("if x: 1 elif true: 2 else: 3", ELIMINATE);
    assert_eq!(
        jumps(&emitted),
        2,
        "only the outer test's jumps stay: {emitted:?}"
    );
    assert_eq!(emitted.count(&Bytecode::PushInt(2)), 1, "{emitted:?}");
    assert_eq!(
        emitted.count(&Bytecode::PushInt(3)),
        0,
        "the inner alternate is dropped: {emitted:?}"
    );
}

#[test]
fn a_nested_runtime_if_survives_its_parent_elimination() {
    let emitted = code("if true: (if x: 1 else: 2) else: 3", ELIMINATE);
    assert_eq!(
        jumps(&emitted),
        2,
        "only the inner test's jumps stay: {emitted:?}"
    );
    assert_eq!(emitted.count(&Bytecode::PushInt(3)), 0, "{emitted:?}");
}

#[test]
fn a_runtime_condition_is_never_eliminated() {
    let emitted = code("if x: 1 else: 2", ELIMINATE);
    assert_eq!(jumps(&emitted), 2, "{emitted:?}");
}

#[test]
fn a_constant_string_condition_decides() {
    // A String is a pooled constant, not an inline push; it is known all the same.
    let emitted = code(r#"if "": x else: 2"#, ELIMINATE);
    assert_eq!(jumps(&emitted), 0, "{emitted:?}");
    assert!(loads_x(&emitted), "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(2)), 0, "{emitted:?}");
}

#[test]
fn an_empty_structure_condition_decides() {
    // An empty literal is known without folding: nothing goes into it.
    for condition in ["[]", "{}"] {
        let emitted = code(&format!("if {condition}: x else: 2"), ELIMINATE);
        assert_eq!(jumps(&emitted), 0, "{condition}: {emitted:?}");
        assert!(loads_x(&emitted), "{condition}: {emitted:?}");
        assert_eq!(
            emitted.count(&Bytecode::PushInt(2)),
            0,
            "{condition}: {emitted:?}"
        );
    }
}

#[test]
fn folding_and_elimination_together_leave_only_the_taken_value() {
    // The condition folds beside the runtime alternate and decides; the
    // consequent folds too.
    let emitted = code(
        "if 1 == 1: 2 + 3 else: x",
        ELIMINATE.with(Optimization::ConstantFold, true),
    );
    assert_eq!(jumps(&emitted), 0, "{emitted:?}");
    assert!(!loads_x(&emitted), "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::Add), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(5)), 1, "{emitted:?}");
}

#[test]
fn a_raising_condition_is_not_a_known_constant() {
    // The condition's fold fails, so it stays for runtime, and so does the test.
    let source = format!("if {RAISE}: x else: 2");
    let emitted = code(&source, ELIMINATE.with(Optimization::ConstantFold, true));
    assert_eq!(emitted.count(&Bytecode::Divide), 1, "{emitted:?}");
    assert_eq!(jumps(&emitted), 2, "{emitted:?}");
}

#[test]
fn a_propagated_condition_decides_inside_a_block() {
    let emitted = code(
        r"
        do {
            def c = false
            if c: x else: 2
        }
        ",
        ELIMINATE.with(Optimization::ConstantPropagate, true),
    );
    assert_eq!(jumps(&emitted), 0, "{emitted:?}");
    assert!(!loads_x(&emitted), "the consequent is dropped: {emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(2)), 1, "{emitted:?}");
}

#[test]
fn a_runtime_shadow_of_a_constant_keeps_the_test() {
    let emitted = code(
        r"
        def c = true
        do {
            def c = x
            if c: 1 else: 2
        }
        ",
        ELIMINATE.with(Optimization::ConstantPropagate, true),
    );
    assert_eq!(jumps(&emitted), 2, "the inner `c` is runtime: {emitted:?}");
}

#[test]
fn a_propagated_elif_condition_decides() {
    let emitted = code(
        r"
        def c = 0
        if x: 1 elif c: 2 else: 3
        ",
        ELIMINATE.with(Optimization::ConstantPropagate, true),
    );
    assert_eq!(
        jumps(&emitted),
        2,
        "only the outer test's jumps stay: {emitted:?}"
    );
    assert_eq!(
        emitted.count(&Bytecode::PushInt(3)),
        0,
        "the inner alternate is dropped: {emitted:?}"
    );
}

#[test]
fn elimination_applies_inside_a_lambda() {
    let body = code("fn -> if true: x else: 2", ELIMINATE).nested(0);
    assert_eq!(jumps(&body), 0, "{body:?}");
    assert!(loads_x(&body), "{body:?}");
    assert_eq!(body.count(&Bytecode::PushInt(2)), 0, "{body:?}");
}

#[test]
fn a_hoisted_constant_decides_inside_a_lambda() {
    let emitted = code(
        r"
        def c = null
        fn -> if c: x else: 2
        ",
        ELIMINATE
            .with(Optimization::ConstantPropagate, true)
            .with(Optimization::CaptureHoist, true),
    );
    let body = emitted.nested(0);
    assert_eq!(jumps(&body), 0, "{body:?}");
    assert!(!loads_x(&body), "the consequent is dropped: {body:?}");
}

#[test]
fn elimination_applies_inside_a_branch_block() {
    // The outer test is runtime and stays; the one inside its block does not.
    let source = r"
        if x: do {
            def y = 1
            if true: y else: 2
        } else: 3
    ";
    let emitted = code(source, ELIMINATE);
    assert_eq!(jumps(&emitted), 2, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(2)), 0, "{emitted:?}");
}

#[test]
fn an_eliminated_if_statement_leaves_no_test() {
    let source = r"
        if false: x
        if true: x else: 1
        5
    ";
    let emitted = code(source, ELIMINATE);
    assert_eq!(jumps(&emitted), 0, "{emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(1)), 0, "{emitted:?}");
}

#[test]
fn a_discarded_effectful_branch_is_gone() {
    let emitted = code("if false: print(x) else: 2", ELIMINATE);
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
fn without_elimination_a_constant_condition_is_still_tested() {
    let emitted = code("if true: x else: 2", UNOPTIMIZED);
    assert_eq!(jumps(&emitted), 2, "{emitted:?}");
}
