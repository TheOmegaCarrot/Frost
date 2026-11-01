//! Tests of [`OptimizationOptions::compile_patterns`].
//!
//! Behavioral cases run under every optimization permutation, so the two ways
//! of testing a pattern must agree. Their targets are captured, unknown at
//! compile time, so that no fold decides the match and the code runs as
//! compiled. Code-shape cases pin exactly the options they are about.

use std::collections::BTreeMap;

use crate::common::global_slot;
use crate::common::pattern::{
    at_least, bind, discard, either, exact, input_key, int, key, map, pin, typed, value,
};
use crate::script::{Emitted, Script, UNOPTIMIZED, raises, run};
use crate::unshared::unshared;

use frostlang::bytecode::{Bytecode, CompiledPattern, PatternComparand, PatternInput};
use frostlang::compile::{Optimization, OptimizationOptions};
use frostlang::{FrostFloat, FrostType, MapKey, Value};

const COMPILE_PATTERNS: OptimizationOptions = UNOPTIMIZED.with(Optimization::CompilePatterns, true);

/// Defines `f`, which returns its argument, for a pin or key that calls it.
const DEFINE_F: &str = "defn f(n) -> n";

/// The pattern a case expects, given the code it compiled to.
type ExpectedPattern = fn(&Emitted) -> CompiledPattern;

/// `source`'s top level, compiled under `options`, with `x` captured.
fn code_with_x(source: &str, options: OptimizationOptions) -> Emitted {
    Script::new(source).capture("x", Value::Null).code(options)
}

/// A match of the captured `x`, with the arm `pattern => 0` and a catch-all,
/// after the definitions in `before`.
fn one_arm(before: &str, pattern: &str) -> String {
    format!(
        r"
{before}
match x {{
    {pattern} => 0,
    _ => 1
}}
"
    )
}

/// The local slot of the binding `name`, as a pattern holds it.
fn slot(emitted: &Emitted, name: &str) -> u32 {
    u32::try_from(emitted.slot_named(name)).expect("a slot fits a u32")
}

/// The one index in `pool` holding `entry`, as a pattern holds it.
fn only_index<T: PartialEq + std::fmt::Debug>(pool: &[T], entry: &T) -> u32 {
    let indices: Vec<usize> = pool
        .iter()
        .enumerate()
        .filter(|(_, held)| *held == entry)
        .map(|(index, _)| index)
        .collect();
    let [index] = indices[..] else {
        panic!("the pool holds {entry:?} at {indices:?}, not once: {pool:?}");
    };
    u32::try_from(index).expect("a pool index fits a u32")
}

/// The index of `constant`, which the constant pool holds once.
fn constant(emitted: &Emitted, constant: &Value) -> u32 {
    only_index(emitted.constants(), constant)
}

/// The index of `key`, which the key pool holds once.
fn key_constant(emitted: &Emitted, key: &MapKey) -> u32 {
    only_index(emitted.key_constants(), key)
}

/// The global `name`, as a pattern reads it.
fn global(name: &str) -> PatternInput {
    PatternInput::Global(u32::try_from(global_slot(name)).expect("a global fits a u32"))
}

/// Assert `emitted` tests its one tried arm with `MatchPattern` against
/// `expected`, and lays no mark for it.
fn assert_compiled(emitted: &Emitted, expected: CompiledPattern, source: &str) {
    assert_eq!(
        emitted.patterns(),
        [expected],
        "{source:?} compiles to the pattern"
    );
    assert_eq!(
        emitted.count(&Bytecode::MatchPattern(0)),
        1,
        "{source:?} tests the pattern once: {emitted:?}"
    );
    assert_eq!(
        emitted.count(&Bytecode::MarkStack),
        0,
        "{source:?} needs no mark: {emitted:?}"
    );
}

/// Assert each arm pattern in `cases`, after the definitions in `before`,
/// compiles under `options` to the pattern paired with it.
fn assert_each_compiles(
    before: &str,
    options: OptimizationOptions,
    cases: &[(&str, ExpectedPattern)],
) {
    for (pattern, expected) in cases {
        let source = one_arm(before, pattern);
        let emitted = code_with_x(&source, options);
        assert_compiled(&emitted, expected(&emitted), &source);
    }
}

/// Assert `emitted` tests its one tried arm's pattern neither with
/// `MatchPattern` nor under a mark.
fn assert_untested(emitted: &Emitted, source: &str) {
    assert!(
        emitted.patterns().is_empty(),
        "{source:?} compiles no pattern under {:?}: {:?}",
        emitted.optimization,
        emitted.patterns()
    );
    assert_eq!(
        emitted.count(&Bytecode::MarkStack),
        0,
        "{source:?} lays no mark under {:?}: {emitted:?}",
        emitted.optimization
    );
}

/// Assert `emitted` keeps its one tried arm's instructions.
fn assert_not_compiled(emitted: &Emitted, source: &str) {
    assert!(
        emitted.patterns().is_empty(),
        "{source:?} compiles no pattern under {:?}: {:?}",
        emitted.optimization,
        emitted.patterns()
    );
    assert!(
        emitted.count(&Bytecode::MarkStack) > 0,
        "{source:?} tries its arm under a mark under {:?}: {emitted:?}",
        emitted.optimization
    );
}

/// The value `source` runs to, with each name in `captures` captured as the
/// value of its Frost expression.
fn run_with(source: &str, captures: &[(&str, &str)]) -> Value {
    let captures: Vec<(&str, Value)> = captures
        .iter()
        .map(|(name, expression)| (*name, run(expression)))
        .collect();
    Script::new(source).captures(&captures).run()
}

/// Assert `source` runs, with `x` captured as each input and the names in
/// `captures` as theirs, to the value of the Frost expression paired with
/// the input.
fn assert_outcomes_with(source: &str, captures: &[(&str, &str)], cases: &[(&str, &str)]) {
    for (input, expected) in cases {
        let all: Vec<(&str, &str)> = [("x", *input)]
            .into_iter()
            .chain(captures.to_vec())
            .collect();
        assert_eq!(
            run_with(source, &all),
            run(expected),
            "{source:?} with `x` = {input} and {captures:?} is {expected}"
        );
    }
}

/// [`assert_outcomes_with`] with no other captures.
fn assert_outcomes(source: &str, cases: &[(&str, &str)]) {
    assert_outcomes_with(source, &[], cases);
}

// --- Which patterns compile ---

#[test]
fn each_kind_of_pattern_compiles() {
    assert_each_compiles(
        "",
        COMPILE_PATTERNS,
        &[
            ("42", |_| int(42)),
            ("null", |_| value(PatternComparand::Null)),
            ("true", |_| value(PatternComparand::Bool(true))),
            ("1.5", |_| {
                value(PatternComparand::Float(
                    FrostFloat::new(1.5).expect("a finite Float"),
                ))
            }),
            ("'text'", |e| {
                value(PatternComparand::Constant(constant(
                    e,
                    &Value::from("text"),
                )))
            }),
            ("x'61'", |e| {
                value(PatternComparand::Constant(constant(e, &run("x'61'"))))
            }),
            ("[1, [2]]", |_| exact([int(1), exact([int(2)])])),
            ("n is Int", |e| typed(FrostType::INT, Some(slot(e, "n")))),
            ("_ is Numeric", |_| typed(FrostType::NUMERIC, None)),
            ("[]", |_| exact([])),
            ("[a, _, ...rest]", |e| {
                at_least([bind(slot(e, "a")), discard()], Some(slot(e, "rest")))
            }),
            ("[...rest]", |e| at_least([], Some(slot(e, "rest")))),
            ("{}", |_| map([], None)),
            ("{name: n} as m", |e| {
                map(
                    [(
                        key(key_constant(e, &MapKey::from("name"))),
                        bind(slot(e, "n")),
                    )],
                    Some(slot(e, "m")),
                )
            }),
            ("{[1]: one}", |e| {
                map(
                    [(key(key_constant(e, &MapKey::Int(1))), bind(slot(e, "one")))],
                    None,
                )
            }),
            ("1 | [_] | 3", |_| {
                either([int(1), exact([discard()]), int(3)])
            }),
        ],
    );
}

#[test]
fn a_pin_or_key_reading_a_name_compiles_to_an_input() {
    assert_each_compiles(
        "def k = 'name'",
        COMPILE_PATTERNS,
        &[
            ("(k)", |e| pin(PatternInput::Local(slot(e, "k")))),
            ("(print)", |_| pin(global("print"))),
            ("{[k]: v}", |e| {
                map(
                    [(
                        input_key(PatternInput::Local(slot(e, "k"))),
                        bind(slot(e, "v")),
                    )],
                    None,
                )
            }),
            // A name the pattern itself bound is read as bound earlier in the
            // same test, since its slot is written only once the whole pattern
            // matches.
            ("[a, (a)]", |e| {
                let a = slot(e, "a");
                exact([bind(a), pin(PatternInput::EarlierBinding(a))])
            }),
            ("[a, {[a]: b}]", |e| {
                let a = slot(e, "a");
                exact([
                    bind(a),
                    map(
                        [(
                            input_key(PatternInput::EarlierBinding(a)),
                            bind(slot(e, "b")),
                        )],
                        None,
                    ),
                ])
            }),
            // Also in a later branch, which binds anew only its own names.
            ("[a, 0 | (a)]", |e| {
                let a = slot(e, "a");
                exact([
                    bind(a),
                    either([int(0), pin(PatternInput::EarlierBinding(a))]),
                ])
            }),
            ("[a, [b, (a)] | [(a), b]]", |e| {
                let (a, b) = (slot(e, "a"), slot(e, "b"));
                exact([
                    bind(a),
                    either([
                        exact([bind(b), pin(PatternInput::EarlierBinding(a))]),
                        exact([pin(PatternInput::EarlierBinding(a)), bind(b)]),
                    ]),
                ])
            }),
        ],
    );
}

#[test]
fn a_pin_of_a_capture_reads_its_slot() {
    // A name from the enclosing scope, as in a REPL.
    let source = one_arm("", "(k)");
    let emitted = Script::new(&source)
        .captures(&[("x", Value::Null), ("k", Value::Null)])
        .code(COMPILE_PATTERNS);
    assert_compiled(
        &emitted,
        pin(PatternInput::Local(slot(&emitted, "k"))),
        &source,
    );

    // A lambda's capture. The outer match gives the top level a pattern of its
    // own, so the lambda's is seen to be the lambda's.
    let source = r"
def k = 'name'
def g = fn m -> match m {
    (k) => 0,
    _ => 1
}
match x {
    [_] => g,
    _ => g
}
";
    let emitted = code_with_x(source, COMPILE_PATTERNS);
    assert_eq!(
        emitted.patterns(),
        [exact([discard()])],
        "the top level's own pattern"
    );
    let lambda = emitted.nested(0);
    assert_compiled(
        &lambda,
        pin(PatternInput::Local(slot(&lambda, "k"))),
        source,
    );
}

#[test]
fn a_pin_or_key_that_runs_code_keeps_the_whole_pattern_as_instructions() {
    for pattern in [
        "(f(1))",
        "{[f('k')]: v}",
        // Nested anywhere, it keeps the whole pattern.
        "[a, (f(a))]",
        "[_, {[f(1)]: v}]",
        "1 | (f(1))",
        // Unfolded, an operation runs code.
        "(1 + 1)",
        // A constant that is not a valid key raises only once looked up.
        "{[null]: v}",
        "{[[1]]: v}",
    ] {
        let source = one_arm(DEFINE_F, pattern);
        assert_not_compiled(&code_with_x(&source, COMPILE_PATTERNS), &source);
    }
}

#[test]
fn a_folded_pin_compiles() {
    assert_each_compiles(
        "",
        COMPILE_PATTERNS.with(Optimization::ConstantFold, true),
        &[("(1 + 1)", |_| int(2))],
    );
}

#[test]
fn a_known_pin_or_key_compiles_as_its_value() {
    let propagated = COMPILE_PATTERNS.with(Optimization::ConstantPropagate, true);

    let source = one_arm("def k = 'name'", "(k)");
    let emitted = code_with_x(&source, propagated);
    // Found by value, since the pool may hold other copies.
    let [CompiledPattern::Value(PatternComparand::Constant(index))] = emitted.patterns() else {
        panic!(
            "{source:?} compiles to a constant pin: {:?}",
            emitted.patterns()
        );
    };
    assert_eq!(
        emitted.constants()[*index as usize],
        Value::from("name"),
        "{source:?} compares against its constant"
    );

    assert_each_compiles(
        "def k = 'name'",
        propagated,
        &[("{[k]: v}", |e| {
            map(
                [(
                    key(key_constant(e, &MapKey::from("name"))),
                    bind(slot(e, "v")),
                )],
                None,
            )
        })],
    );
    // A scalar is held inline, so one value always has one comparand.
    assert_each_compiles("def k = 1", propagated, &[("(k)", |_| int(1))]);
    assert_each_compiles(
        "def k = null",
        propagated,
        &[("(k)", |_| value(PatternComparand::Null))],
    );
    // A Function cannot be a constant, so its pin still reads its slot.
    assert_each_compiles(
        "defn g() -> 1",
        propagated,
        &[("(g)", |e| pin(PatternInput::Local(slot(e, "g"))))],
    );
    // A folded key is a constant key.
    assert_each_compiles(
        "",
        COMPILE_PATTERNS.with(Optimization::ConstantFold, true),
        &[("{['a' + 'b']: v}", |e| {
            map(
                [(
                    key(key_constant(e, &MapKey::from("ab"))),
                    bind(slot(e, "v")),
                )],
                None,
            )
        })],
    );

    // A known value that is not a valid key raises only once looked up.
    let source = one_arm("def k = null", "{[k]: v}");
    assert_not_compiled(&code_with_x(&source, propagated), &source);
}

#[test]
fn every_permutation_compiles_patterns_exactly_when_the_optimization_is_on() {
    for (source, captures) in [
        (
            r"
match x {
    [a, (a)] => a,
    _ => 1
}
",
            &["x"][..],
        ),
        (
            r"
match x {
    {[k]: v} => v,
    _ => 1
}
",
            &["x", "k"][..],
        ),
    ] {
        let captures: Vec<(&str, Value)> =
            captures.iter().map(|name| (*name, Value::Null)).collect();
        let script = Script::new(source).captures(&captures);
        for emitted in script.code_where(|options| options.compile_patterns) {
            assert_eq!(
                (
                    emitted.count(&Bytecode::MatchPattern(0)),
                    emitted.count(&Bytecode::MarkStack)
                ),
                (1, 0),
                "{source:?} compiles its pattern under {:?}: {emitted:?}",
                emitted.optimization
            );
        }
        for emitted in script.code_where(|options| !options.compile_patterns) {
            assert_not_compiled(&emitted, source);
        }
    }
}

#[test]
fn a_guarded_bare_binding_is_not_tested() {
    // The arm is tried only for its guard, and its pattern's own store takes a
    // copy of the target.
    let source = one_arm("", "n if: n > 0");
    let emitted = code_with_x(&source, COMPILE_PATTERNS);
    let n = emitted.slot_named("n");
    assert_untested(&emitted, &source);
    assert!(
        emitted
            .code
            .windows(2)
            .any(|ops| ops == [Bytecode::Dup, Bytecode::DefLocal(n)]),
        "{source:?} stores a copy of the target to `n`: {emitted:?}"
    );

    // `_` stores nothing: its copy is popped, and with discard elimination
    // never made.
    let source = one_arm("", "_ if: x > 0");
    let emitted = code_with_x(&source, COMPILE_PATTERNS);
    assert_untested(&emitted, &source);
    let emitted = code_with_x(
        &source,
        COMPILE_PATTERNS.with(Optimization::DiscardEliminate, true),
    );
    assert_eq!(
        emitted.count(&Bytecode::Dup),
        0,
        "{source:?} makes no copy of the target: {emitted:?}"
    );
}

#[test]
fn a_typed_binding_or_an_alternative_is_still_tested() {
    // A typed binding can fail. An alternative can match every value, but
    // which branch matches decides its bindings.
    assert_each_compiles(
        "",
        COMPILE_PATTERNS,
        &[
            ("n is Int if: n > 0", |e| {
                typed(FrostType::INT, Some(slot(e, "n")))
            }),
            ("[n] | n if: n != 0", |e| {
                let n = slot(e, "n");
                either([exact([bind(n)]), bind(n)])
            }),
        ],
    );
}

#[test]
fn every_permutation_tests_a_guarded_bare_binding_exactly_when_the_optimization_is_off() {
    for source in [
        r"
match x {
    n if: n > 0 => n,
    _ => 0
}
",
        r"
match x {
    _ if: x > 0 => 1,
    _ => 0
}
",
    ] {
        let script = Script::new(source).capture("x", Value::Null);
        for emitted in script.code_where(|options| options.compile_patterns) {
            assert_untested(&emitted, source);
        }
        for emitted in script.code_where(|options| !options.compile_patterns) {
            assert_not_compiled(&emitted, source);
        }
    }
}

#[test]
fn a_catch_all_and_a_def_keep_their_instructions() {
    // A catch-all cannot fail, so it tests nothing.
    let source = r"
match x {
    [a] => a,
    b => b
}
";
    let emitted = code_with_x(source, COMPILE_PATTERNS);
    assert_eq!(
        emitted.patterns(),
        [exact([bind(slot(&emitted, "a"))])],
        "only the tried arm compiles"
    );

    // A def raises on a mismatch, which `MatchPattern` does not.
    let source = r"
def [a, b] = x
a + b
";
    let emitted = code_with_x(source, COMPILE_PATTERNS);
    assert!(
        emitted.patterns().is_empty(),
        "a def compiles no pattern: {emitted:?}"
    );
}

#[test]
fn each_tried_arm_has_its_own_pattern() {
    let source = r"
match x {
    [1] => 'one',
    [a, 2] => a,
    _ => 'other'
}
";
    let emitted = code_with_x(source, COMPILE_PATTERNS);
    assert_eq!(
        emitted.patterns(),
        [exact([int(1)]), exact([bind(slot(&emitted, "a")), int(2)])],
        "one pattern per tried arm, in order"
    );
    for index in 0..2 {
        assert_eq!(
            emitted.count(&Bytecode::MatchPattern(index)),
            1,
            "pattern {index} is tested once: {emitted:?}"
        );
    }
}

#[test]
fn an_arm_that_cannot_compile_keeps_its_mark_beside_ones_that_do() {
    let source = format!(
        r"
{DEFINE_F}
match x {{
    1 => 'one',
    (f(2)) => 'two',
    [y] => y,
    _ => 'other'
}}
"
    );
    let emitted = code_with_x(&source, COMPILE_PATTERNS);
    assert_eq!(
        emitted.patterns(),
        [int(1), exact([bind(slot(&emitted, "y"))])],
        "the first and third arms compile"
    );
    assert_eq!(
        emitted.count(&Bytecode::MarkStack),
        1,
        "only the second arm lays a mark: {emitted:?}"
    );
    assert_outcomes(
        &source,
        &[
            ("1", "'one'"),
            ("2", "'two'"),
            ("[3]", "3"),
            ("4", "'other'"),
        ],
    );
}

#[test]
fn a_match_in_a_pin_that_cannot_compile_reads_the_outer_binding_from_its_slot() {
    // The outer pin runs code, so its arm keeps its instructions, and `a` is
    // in its slot by the time the inner match tests it.
    let source = r"
match x {
    [a, (match 1 { (a) => 1, _ => 2 })] => 'yes',
    _ => 'no'
}
";
    let emitted = code_with_x(source, COMPILE_PATTERNS);
    assert_eq!(
        emitted.patterns(),
        [pin(PatternInput::Local(slot(&emitted, "a")))],
        "only the inner pattern compiles"
    );
    assert_eq!(
        emitted.count(&Bytecode::MarkStack),
        1,
        "the outer arm keeps its mark: {emitted:?}"
    );
    assert_outcomes(
        source,
        &[("[1, 1]", "'yes'"), ("[2, 2]", "'yes'"), ("[2, 1]", "'no'")],
    );
}

// --- With the other optimizations ---

#[test]
fn a_binding_nothing_reads_is_not_kept() {
    let dead_stores = COMPILE_PATTERNS.with(Optimization::DeadStoreEliminate, true);
    assert_each_compiles(
        "",
        dead_stores,
        &[
            ("[a, b] if: b > 0", |e| {
                exact([discard(), bind(slot(e, "b"))])
            }),
            // The type test stays, without the binding.
            ("n is Int", |_| typed(FrostType::INT, None)),
            ("[a, ...rest]", |_| at_least([discard()], None)),
            ("{a: v} as m", |e| {
                map(
                    [(key(key_constant(e, &MapKey::from("a"))), discard())],
                    None,
                )
            }),
            // The pattern's own read keeps a binding.
            ("[a, (a)]", |e| {
                let a = slot(e, "a");
                exact([bind(a), pin(PatternInput::EarlierBinding(a))])
            }),
            ("[a, {[a]: _}]", |e| {
                let a = slot(e, "a");
                exact([
                    bind(a),
                    map(
                        [(input_key(PatternInput::EarlierBinding(a)), discard())],
                        None,
                    ),
                ])
            }),
            ("[a, 0 | (a)]", |e| {
                let a = slot(e, "a");
                exact([
                    bind(a),
                    either([int(0), pin(PatternInput::EarlierBinding(a))]),
                ])
            }),
        ],
    );

    // A closure's capture of a binding also keeps it.
    let source = r"
match x {
    [a] => fn -> a,
    _ => null
}
";
    let emitted = code_with_x(source, dead_stores);
    assert_compiled(&emitted, exact([bind(slot(&emitted, "a"))]), source);
}

#[test]
fn an_unread_bare_name_is_not_stored() {
    let dead_stores = COMPILE_PATTERNS
        .with(Optimization::DeadStoreEliminate, true)
        .with(Optimization::DiscardEliminate, true);
    let source = one_arm("", "n if: x > 0");
    let emitted = code_with_x(&source, dead_stores);
    assert_eq!(
        emitted.count(&Bytecode::Dup),
        0,
        "{source:?} makes no copy of the target: {emitted:?}"
    );
    assert!(
        !emitted
            .code
            .iter()
            .any(|op| matches!(op, Bytecode::DefLocal(_))),
        "{source:?} stores nothing: {emitted:?}"
    );

    // Read only by the result, it is still stored.
    let source = r"
match x {
    n if: x > 0 => [n],
    _ => 0
}
";
    let emitted = code_with_x(source, dead_stores);
    assert_eq!(
        emitted.count(&Bytecode::DefLocal(emitted.slot_named("n"))),
        1,
        "{source:?} stores `n`, which its result reads: {emitted:?}"
    );
}

#[test]
fn a_pins_read_is_never_consumed_and_keeps_earlier_reads() {
    let consume = COMPILE_PATTERNS.with(Optimization::ConsumeLocals, true);
    // The target's read of `k` comes before the pin's, so is not the last.
    let source = r"
def k = [1]
match k {
    (k) => 'same',
    _ => 'other'
}
";
    let emitted = code_with_x(source, consume);
    let k = slot(&emitted, "k");
    assert_compiled(&emitted, pin(PatternInput::Local(k)), source);
    assert_eq!(
        (
            emitted.count(&Bytecode::LoadLocal(u64::from(k))),
            emitted.count(&Bytecode::ConsumeLocal(u64::from(k)))
        ),
        (1, 0),
        "the target copies `k`, which the pin then reads: {emitted:?}"
    );
}

#[test]
fn a_matched_part_is_left_unshared() {
    // The Array is built at runtime, so only the target holds it, and the arm
    // drops the target once it has matched.
    for source in [
        r"
defn probe(v) -> match v {
    [a] => unshared(a),
    _ => null
}
def make = fn n -> [n]
probe([make(1)])
",
        r"
defn probe(v) -> match v {
    [a] if: len(a) > 0 => unshared(a),
    _ => null
}
def make = fn n -> [n]
probe([make(1)])
",
        // A bare name binds a copy of the whole target; the arm drops the
        // target once it has matched, leaving the copy unshared.
        r"
defn probe(v) -> match v {
    a if: len(a) > 0 => unshared(a),
    _ => null
}
def make = fn n -> [n]
probe(make(1))
",
    ] {
        for optimization in [
            UNOPTIMIZED.with(Optimization::ConsumeLocals, true),
            COMPILE_PATTERNS.with(Optimization::ConsumeLocals, true),
        ] {
            let tail = Script::new(source)
                .capture("unshared", unshared())
                .run_under(optimization);
            assert_eq!(
                tail,
                Value::Bool(true),
                "{source:?} hands the part over whole under {optimization:?}"
            );
        }
    }
}

#[test]
fn an_arm_clears_its_locals_as_its_instructions_do() {
    let clears = |source: &str, optimization| {
        Script::new(source)
            .code(optimization)
            .nested(0)
            .clears()
            .into_iter()
            .map(|names| names.join(" "))
            .collect::<Vec<_>>()
    };
    // After the result, and on failure, as when the guard fails; in tail
    // position, the frame's release stands in for the clear after the result.
    for (source, expected) in [
        (
            r"
fn v -> [match v {
    [a] if: a > 0 => a,
    [b] => b,
    _ => 0
}]
",
            &["a", "a", "b", "b"][..],
        ),
        (
            r"
fn v -> [match v {
    n if: n > 0 => n,
    _ => 0
}]
",
            &["n", "n"][..],
        ),
        (
            r"
fn v -> [match v {
    _ if: v > 0 => 1,
    _ => 0
}]
",
            &[][..],
        ),
        (
            r"
fn v -> match v {
    n if: n > 0 => n,
    _ => 0
}
",
            &["n"][..],
        ),
    ] {
        let compiled = clears(source, COMPILE_PATTERNS);
        assert_eq!(compiled, expected, "{source:?} clears {expected:?}");
        assert_eq!(
            compiled,
            clears(source, UNOPTIMIZED),
            "{source:?} clears as its instructions do"
        );
    }

    // A dropped binding has no slot to clear.
    let dead_stores = COMPILE_PATTERNS
        .with(Optimization::DeadStoreEliminate, true)
        .with(Optimization::DiscardEliminate, true);
    for (source, expected) in [
        (
            r"
fn v -> [match v {
    [a, b] if: b > 0 => b,
    _ => 0
}]
",
            &["b", "b"][..],
        ),
        (
            r"
fn v -> [match v {
    n if: v > 0 => 0,
    _ => 1
}]
",
            &[][..],
        ),
    ] {
        assert_eq!(
            clears(source, dead_stores),
            expected,
            "{source:?} clears only the bindings it keeps"
        );
    }
}

#[test]
fn a_foldable_match_folds_through_its_pattern() {
    let fold = COMPILE_PATTERNS
        .with(Optimization::ConstantFold, true)
        .with(Optimization::ConstantPropagate, true);
    for source in [
        // The fold seats the known Function the pin reads.
        r"
defn g() -> 1
match g {
    (g) => 'same',
    _ => 'other'
}
",
        // The fold gives the pattern's binding a slot.
        r"
match [1, 1] {
    [a, 1] => 'same',
    _ => 'other'
}
",
    ] {
        let emitted = Script::new(source).code(fold);
        assert!(
            emitted.patterns().is_empty(),
            "{source:?} folds away its pattern: {emitted:?}"
        );
        assert_eq!(
            emitted.constants(),
            [Value::from("same")],
            "{source:?} folds to its taken arm's result"
        );
    }
}

#[test]
fn a_patterns_constants_share_the_functions_pools() {
    let source = format!(
        r"
{DEFINE_F}
match x {{
    'text' => 'text',
    ['text'] => 0,
    {{name: n}} => n,
    {{name: (f(1))}} => 1,
    _ => 2
}}
"
    );
    let deduplicate = COMPILE_PATTERNS.with(Optimization::DeduplicateConstants, true);
    let emitted = code_with_x(&source, deduplicate);
    let text = constant(&emitted, &Value::from("text"));
    let name = key_constant(&emitted, &MapKey::from("name"));
    assert_eq!(
        emitted.patterns(),
        [
            value(PatternComparand::Constant(text)),
            exact([value(PatternComparand::Constant(text))]),
            map([(key(name), bind(slot(&emitted, "n")))], None),
        ],
        "every arm's `'text'` is the one entry, as is `name`"
    );
    assert_eq!(
        emitted.count(&Bytecode::LoadConst(u64::from(text))),
        1,
        "the result loads the same entry: {emitted:?}"
    );
    assert_eq!(
        emitted.key_constants(),
        [MapKey::from("name")],
        "the uncompiled arm's key is the same entry"
    );

    let emitted = code_with_x(&source, COMPILE_PATTERNS);
    let [
        CompiledPattern::Value(PatternComparand::Constant(first)),
        CompiledPattern::Array { elements, .. },
        ..,
    ] = emitted.patterns()
    else {
        panic!("unexpected patterns: {:?}", emitted.patterns());
    };
    let [CompiledPattern::Value(PatternComparand::Constant(second))] = elements[..] else {
        panic!("unexpected Array pattern: {elements:?}");
    };
    assert_ne!(
        *first, second,
        "without deduplication, each use of `'text'` has its own entry"
    );
}

// --- Behavior, under every permutation ---

#[test]
fn compiled_patterns_match_as_their_instructions_do() {
    let cases: &[(&str, &[(&str, &str)])] = &[
        (
            r"
match x {
    41 => 'no',
    42 => 'yes',
    _ => 'other'
}
",
            &[("42", "'yes'"), ("41.0", "'other'")],
        ),
        (
            r"
match x {
    1 => 'Int',
    1.0 => 'Float',
    -0.0 => 'negative zero',
    _ => 'other'
}
",
            &[
                ("1", "'Int'"),
                ("1.0", "'Float'"),
                ("0.0", "'other'"),
                ("-0.0", "'negative zero'"),
            ],
        ),
        (
            r"
match x {
    null => 'null',
    false => 'false',
    'a' => 'a',
    _ => 'other'
}
",
            &[
                ("null", "'null'"),
                ("false", "'false'"),
                ("'a'", "'a'"),
                ("x'61'", "'other'"),
            ],
        ),
        (
            r"
match x {
    n is Int => n,
    _ is Flat => 'flat',
    _ => 'other'
}
",
            &[
                ("3", "3"),
                ("'s'", "'flat'"),
                ("x'73'", "'flat'"),
                ("[]", "'other'"),
            ],
        ),
        // Signed zeros are distinct keys, deduplicated or not.
        (
            r"
match x {
    {[-0.0]: v} => ['negative', v],
    {[0.0]: v} => ['positive', v],
    _ => 'none'
}
",
            &[
                ("{[0.0]: 1}", "['positive', 1]"),
                ("{[-0.0]: 1}", "['negative', 1]"),
                ("{[0]: 1}", "'none'"),
            ],
        ),
        (
            r"
match x {
    [a, b] => [b, a],
    [a, ...rest] => rest,
    [] => 'empty',
    _ => 'other'
}
",
            &[
                ("[1, 2]", "[2, 1]"),
                ("[1, 2, 3]", "[2, 3]"),
                ("[1]", "[]"),
                ("[]", "'empty'"),
                ("'ab'", "'other'"),
                ("{a: 1}", "'other'"),
            ],
        ),
        (
            r"
match x {
    {c: c} => c,
    {a: a} as m => [a, m.b],
    _ => 'other'
}
",
            &[
                ("{c: 3}", "3"),
                ("{a: 1, b: 2}", "[1, 2]"),
                ("{a: null, b: 0}", "[null, 0]"),
                ("[1]", "'other'"),
            ],
        ),
        (
            r"
match x {
    {[1]: a, [true]: b, [x'6b']: c} => [a, b, c],
    _ => 'other'
}
",
            &[
                ("{[1]: 'i', [true]: 'b', [x'6b']: 'k'}", "['i', 'b', 'k']"),
                ("{[1.0]: 'i', [true]: 'b', [x'6b']: 'k'}", "'other'"),
            ],
        ),
        // Alternatives commit to the first branch that matches.
        (
            r"
match x {
    [1] | [2] => 'low',
    [a, 9] | [_, a] => a,
    _ => 'other'
}
",
            &[("[2]", "'low'"), ("[5, 9]", "5"), ("[5, 6]", "6")],
        ),
        (
            r"
match x {
    [[a] | a, (a)] => 'matched',
    _ => 'not'
}
",
            &[
                ("[[1], 1]", "'matched'"),
                ("[[1], [1]]", "'not'"),
                ("[2, 2]", "'matched'"),
            ],
        ),
    ];
    for (source, outcomes) in cases {
        assert_outcomes(source, outcomes);
    }
}

#[test]
fn a_pin_or_key_reads_the_value_its_name_has() {
    // A local, also read by the target and the result.
    assert_outcomes(
        r"
def k = [1]
match x {
    (k) => k + [2],
    _ => k
}
",
        &[("[1]", "[1, 2]"), ("[2]", "[1]")],
    );
    // A global.
    assert_outcomes(
        r"
match x {
    (print) => 'print',
    _ => 'other'
}
",
        &[("print", "'print'"), ("len", "'other'")],
    );
    // A key, from a local and from a lambda's capture.
    assert_outcomes(
        r"
def k = 'a'
def g = fn m -> match m {
    {[k]: v} => v,
    _ => null
}
[match x { {[k]: v} => v, _ => null }, g(x)]
",
        &[("{a: 1}", "[1, 1]"), ("{b: 2}", "[null, null]")],
    );
    // A Function compares by identity. A lambda would be a new Function in
    // each run, so a global stands in for another Function.
    assert_outcomes(
        r"
defn g() -> 1
match x {
    (g) => 'g',
    _ => 'other'
}
",
        &[("len", "'other'")],
    );
    assert_eq!(
        run(r"
defn g() -> 1
match g {
    (g) => 'g',
    _ => 'other'
}
"),
        Value::from("g"),
        "a Function is its own value, even once folding knows it"
    );
}

#[test]
fn a_target_and_its_pin_read_the_same_local() {
    for (source, expected) in [
        (
            r"
match k {
    (k) => 'same',
    _ => 'other'
}
",
            "'same'",
        ),
        (
            r"
match [k] {
    [(k)] => k + [2],
    _ => 0
}
",
            "[1, 2]",
        ),
    ] {
        assert_eq!(
            run_with(source, &[("k", "[1]")]),
            run(expected),
            "{source:?} is {expected}"
        );
    }
}

#[test]
fn a_pin_reads_a_binding_of_the_same_pattern() {
    assert_outcomes(
        r"
match x {
    [a, (a)] => a,
    _ => 'differ'
}
",
        &[("[1, 1]", "1"), ("[1, 2]", "'differ'"), ("[1]", "'differ'")],
    );
    // Also in a later branch, and as a key.
    assert_outcomes(
        r"
match x {
    [a, 0 | (a)] => a,
    _ => 'differ'
}
",
        &[("[1, 0]", "1"), ("[1, 1]", "1"), ("[1, 2]", "'differ'")],
    );
    assert_outcomes(
        r"
match x {
    [a, {[a]: v}] => v,
    _ => 'none'
}
",
        &[("['k', {k: 5}]", "5"), ("['j', {k: 5}]", "'none'")],
    );
    // A failed branch leaves no binding behind, and the alternative does not
    // backtrack into a later branch.
    assert_outcomes(
        r"
match x {
    [[b, 0] | [_, b], (b)] => b,
    _ => 'no'
}
",
        &[
            ("[[1, 0], 1]", "1"),
            ("[[1, 2], 2]", "2"),
            ("[[1, 2], 1]", "'no'"),
        ],
    );
    // A later branch pins its own binding.
    assert_outcomes(
        r"
match x {
    [b, 0] | [b, (b)] => b,
    _ => 'no'
}
",
        &[("[3, 0]", "3"), ("[3, 3]", "3"), ("[3, 4]", "'no'")],
    );
    // A rest and a whole Map, read as earlier bindings.
    assert_outcomes(
        r"
match x {
    [[_, ...r], (r)] => r,
    _ => 'no'
}
",
        &[
            ("[[1, 2, 3], [2, 3]]", "[2, 3]"),
            ("[[1, 2, 3], [2]]", "'no'"),
        ],
    );
    assert_outcomes(
        r"
match x {
    [{a: _} as m, (m)] => 'same',
    _ => 'no'
}
",
        &[("[{a: 1}, {a: 1}]", "'same'"), ("[{a: 1}, {a: 2}]", "'no'")],
    );
}

#[test]
fn a_nested_match_reads_the_outer_arms_binding() {
    // In the result and in the guard, with no closure between.
    assert_outcomes(
        r"
match x {
    [a, b] => match b {
        (a) => 'same',
        _ => 'differ'
    },
    _ => 'other'
}
",
        &[
            ("[1, 1]", "'same'"),
            ("[1, 2]", "'differ'"),
            ("5", "'other'"),
        ],
    );
    assert_outcomes(
        r"
match x {
    [a, b] if: match b { (a) => true, _ => false } => 'same',
    _ => 'differ'
}
",
        &[("[1, 1]", "'same'"), ("[1, 2]", "'differ'")],
    );
}

#[test]
fn a_later_branch_reads_an_outer_name_its_first_branch_shadows() {
    // The first branch binds its own `y`; the second's pin reads the outer one.
    assert_outcomes_with(
        r"
match x {
    [y, 1] | [(y), y] => y,
    _ => 'none'
}
",
        &[("y", "5")],
        &[("[5, 7]", "7"), ("[6, 7]", "'none'"), ("[6, 1]", "6")],
    );
}

#[test]
fn a_pin_reads_the_name_in_view_where_it_is_written() {
    // `m` and `r` are bound only once the parts within the pattern are tested.
    assert_outcomes(
        r"
def m = 1
match x {
    {a: (m)} as m => m,
    _ => 0
}
",
        &[("{a: 1}", "{a: 1}"), ("{a: 2}", "0")],
    );
    assert_outcomes(
        r"
def r = 1
match x {
    [(r), ...r] => r,
    _ => 0
}
",
        &[("[1, 2]", "[2]"), ("[2, 2]", "0")],
    );
}

#[test]
fn a_guarded_bare_binding_matches_as_its_instructions_do() {
    assert_outcomes(
        r"
match x {
    _ if: x == 0 => 'zero',
    n if: n == 11 => n * 2,
    n if: n == 12 => [n],
    [n] | n if: n == 5 => 'five',
    m => [m]
}
",
        &[
            ("0", "'zero'"),
            ("11", "22"),
            ("12", "[12]"),
            ("[5]", "'five'"),
            ("5", "'five'"),
            ("3", "[3]"),
        ],
    );

    // With no catch-all, a failed guard leaves no arm to match, so the match
    // raises.
    let source = r"
match x {
    n if: n > 0 => n
}
";
    assert_outcomes(source, &[("3", "3")]);
    let raised = Script::new(source).capture("x", run("-1")).raises();
    assert!(
        raised.contains("No match arm matches the value: -1"),
        "raised: {raised}"
    );
}

#[test]
fn a_failed_guard_leaves_the_next_arm_to_bind_afresh() {
    assert_outcomes(
        r"
match x {
    [a] if: a > 1 => 'big',
    [a] => a,
    _ => 'other'
}
",
        &[("[5]", "'big'"), ("[1]", "1"), ("[]", "'other'")],
    );
}

#[test]
fn an_invalid_key_read_from_a_name_raises_only_on_a_map() {
    let source = r"
def k = null
match x {
    {[k]: v} => v,
    _ => 'no'
}
";
    let raised = Script::new(source).capture("x", run("{a: 1}")).raises();
    assert!(
        raised.contains("Type Null is not a valid Map key"),
        "a lookup in a Map raises, but raised: {raised}"
    );
    assert_outcomes(source, &[("[1]", "'no'")]);
}

#[test]
fn an_invalid_key_is_looked_up_only_once_the_entries_before_it_match() {
    let source = r"
match x {
    [k, {a: 1, [k]: v}] => v,
    _ => 'no'
}
";
    assert_outcomes(
        source,
        &[
            ("[null, {a: 2}]", "'no'"),
            ("[null, [1]]", "'no'"),
            ("['b', {a: 1, b: 7}]", "7"),
        ],
    );
    let raised = Script::new(source)
        .capture("x", run("[null, {a: 1}]"))
        .raises();
    assert!(
        raised.contains("Type Null is not a valid Map key"),
        "the lookup of `k` raises, but raised: {raised}"
    );

    // A key read from a global.
    let source = r"
match x {
    {[print]: v} => v,
    _ => 'no'
}
";
    let raised = Script::new(source).capture("x", run("{a: 1}")).raises();
    assert!(
        raised.contains("Type Function is not a valid Map key"),
        "the lookup of `print` raises, but raised: {raised}"
    );
}

#[test]
fn a_pattern_raising_at_compile_time_raises_at_runtime() {
    // A constant target makes the match foldable; the fold raises, so it is
    // abandoned, and the run raises instead.
    let raised = raises(
        r"
match [null, {a: 1}] {
    [k, {[k]: _}] => 'found',
    _ => 'none'
}
",
    );
    assert!(
        raised.contains("Type Null is not a valid Map key"),
        "raised: {raised}"
    );
    assert_eq!(
        run(r"
match [null, 5] {
    [k, {[k]: _}] => 'found',
    _ => 'none'
}
"),
        Value::from("none"),
        "a non-Map is no match, folded or not"
    );
}

#[test]
fn no_arm_matching_raises_naming_the_value() {
    let source = r"
match x {
    [a] => a,
    1 => 1
}
";
    let raised = Script::new(source).capture("x", run("[1, 2]")).raises();
    assert!(
        raised.contains("No match arm matches the value: [ 1, 2 ]"),
        "raised: {raised}"
    );
}

#[test]
fn a_pattern_exports_nothing_and_a_pinned_export_stays() {
    let source = r"
match x {
    [a] => a,
    _ => 0
}
";
    let finished = Script::new(source)
        .capture("x", run("[1]"))
        .implicit_export()
        .finish();
    assert_eq!(
        finished.exports,
        BTreeMap::new(),
        "an arm's binding is no export"
    );

    let source = r"
export def k = [1]
match x {
    (k) => k,
    _ => 0
}
";
    let finished = Script::new(source).capture("x", run("[1]")).finish();
    assert_eq!(finished.tail, run("[1]"), "the pinned export is the result");
    assert_eq!(
        finished.exports,
        BTreeMap::from([("k".to_string(), run("[1]"))]),
        "the export the pin read stays"
    );
}
