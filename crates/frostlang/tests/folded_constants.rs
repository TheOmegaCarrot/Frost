//! Fold results that are deep, or that hold the same container in many places, as
//! constants: the compiler decides whether each can be one without recursing per level
//! and without looking into a shared container more than once.
//!
//! The counts are literals, so that the compiler builds each value by folding. Each
//! case compiles and runs on a small stack under a deadline (see
//! [`deep::on_a_small_stack_within_deadline`]).

use crate::deep;
use crate::script;

use frostlang::Value;
use frostlang::compile::{Optimization, OptimizationOptions};
use script::{Script, UNOPTIMIZED};

use deep::on_a_small_stack_within_deadline;

/// Folding, with the propagation that lets a call to a `defn` fold.
const FOLDING: OptimizationOptions = UNOPTIMIZED
    .with(Optimization::ConstantFold, true)
    .with(Optimization::ConstantPropagate, true);

/// How deep the deep cases nest: within the fold's fuel, so that the fold succeeds.
const LEVELS: usize = 10_000;

/// How many times the shared case doubles, to 2^DOUBLINGS leaves: far more than a walk
/// visiting every place could finish.
const DOUBLINGS: usize = 40;

/// Each level is a Map holding the level below in an Array, so the deep cases nest
/// through both kinds of container. Nothing is shared, as comparing a shared value
/// with one built apart would visit every place.
const NEST: &str = "defn nest(v, n) -> if n == 0: v else: nest({k: [v]}, n - 1)";
const DOUBLED: &str = "defn doubled(v, n) -> if n == 0: v else: doubled([v, v], n - 1)";

/// The value of `nest(inner, LEVELS)`.
fn nested(inner: Value) -> Value {
    deep::nested(LEVELS, inner, |below| {
        Value::map([("k", Value::array([below]))])
    })
}

/// The constants of `source` compiled with [`FOLDING`].
fn constants(source: String) -> Vec<Value> {
    on_a_small_stack_within_deadline(move || {
        Script::new(&source).code(FOLDING).constants().to_vec()
    })
}

#[test]
fn a_deep_fold_result_becomes_a_constant() {
    let constants = constants(format!("{NEST}\nnest(1, {LEVELS})"));
    let folded =
        on_a_small_stack_within_deadline(move || constants.contains(&nested(Value::Int(1))));
    assert!(folded, "the result is folded into a constant");
}

#[test]
fn a_deep_fold_result_runs_alike_under_every_option() {
    let source = format!("{NEST}\nnest(1, {LEVELS})");
    let alike = on_a_small_stack_within_deadline(move || {
        Script::new(&source).run() == nested(Value::Int(1))
    });
    assert!(alike, "the tail is 1 nested {LEVELS} levels deep");
}

#[test]
fn a_deep_fold_result_holding_a_function_innermost_is_no_constant() {
    // `len` is pure, so the call folds, but its result cannot be a constant. The
    // Function sits under every level, so deciding that walks them all.
    let constants = constants(format!("{NEST}\nnest(len, {LEVELS})"));
    assert!(
        !constants
            .iter()
            .any(|constant| matches!(constant, Value::Map(_))),
        "a fold result holding a Function is not made a constant"
    );
}

#[test]
fn a_deep_fold_result_holding_a_function_innermost_runs_alike_under_every_option() {
    let source = format!("{NEST}\nnest(len, {LEVELS})");
    let tail = on_a_small_stack_within_deadline(move || Script::new(&source).run());
    let mut innermost = &tail;
    for level in 0..LEVELS {
        innermost = match innermost {
            Value::Map(map) => match map.get_str("k") {
                Some(Value::Array(array)) if array.len() == 1 => &array[0],
                _ => panic!("level {level} holds no Array of one element at `k`"),
            },
            _ => panic!("level {level} is not a Map"),
        };
    }
    assert!(innermost.is_function(), "the innermost value is `len`");
}

#[test]
fn a_fold_result_sharing_its_contents_becomes_a_constant() {
    let constants = constants(format!("{DOUBLED}\ndoubled(1, {DOUBLINGS})"));
    let constant = constants
        .into_iter()
        .find(|constant| matches!(constant, Value::Array(_)))
        .expect("the result is folded into a constant");
    // Comparing with a value built apart would visit every place, so each level is
    // checked to hold one value twice instead, which `==` decides at once for the very
    // same container.
    let innermost = on_a_small_stack_within_deadline(move || {
        let mut level = constant;
        for doubling in 0..DOUBLINGS {
            level = match level {
                Value::Array(array) if array.len() == 2 && array[0] == array[1] => array[0].clone(),
                _ => panic!("doubling {doubling} is not an Array of one value twice"),
            };
        }
        level
    });
    assert_eq!(innermost, Value::Int(1), "the innermost value is 1");
}

#[test]
fn a_fold_result_sharing_its_contents_runs_alike_under_every_option() {
    // The tail stays small, since comparing two such values built apart, as the harness
    // does across options, would visit every place.
    let source = format!(
        r"
        {DOUBLED}
        def d = doubled(1, {DOUBLINGS})
        [len(d), d[0] == d[1]]
        "
    );
    let tail = on_a_small_stack_within_deadline(move || Script::new(&source).run());
    assert_eq!(tail, Value::from(vec![Value::Int(2), Value::Bool(true)]));
}
