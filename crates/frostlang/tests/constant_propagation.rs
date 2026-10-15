//! Constant propagation, end to end: a binding whose value is compile-time known
//! is loaded directly at each use, so a lookup needs no local slot read.
//!
//! The harness runs each behavioral case under every optimization permutation,
//! so propagation is checked never to change a result; the code assertions
//! confirm a lookup was actually propagated.

use crate::script;

use frostlang::bytecode::Bytecode;
use frostlang::compile::{Optimization, OptimizationOptions};
use frostlang::{MapKey, Value};
use script::{Emitted, Script, UNOPTIMIZED, raises, run};

/// The code of `source` under every optimization permutation with constant
/// propagation on (or off, if `propagate` is false).
fn emitted(source: &str, propagate: bool) -> Vec<Emitted> {
    Script::new(source).code_where(|optimization| optimization.constant_propagate == propagate)
}

/// Whether the code reads a local slot, by copy or by move.
fn loads_a_local(emitted: &Emitted) -> bool {
    emitted
        .code
        .iter()
        .any(|op| matches!(op, Bytecode::LoadLocal(_) | Bytecode::ConsumeLocal(_)))
}

#[test]
fn propagation_preserves_a_scalar_binding_value() {
    let source = r"
        def x = 5
        x
    ";
    assert_eq!(run(source), Value::Int(5));
}

#[test]
fn propagation_preserves_a_pooled_binding_value() {
    let source = r#"
        def s = "hello"
        s
    "#;
    assert_eq!(run(source), Value::from("hello"));
}

#[test]
fn a_propagated_lookup_loads_the_constant_not_the_local() {
    let source = r"
        def x = 5
        x
    ";
    for emitted in emitted(source, false) {
        assert!(
            loads_a_local(&emitted),
            "without propagation the lookup is a local load: {emitted:?}"
        );
    }
    for emitted in emitted(source, true) {
        assert!(
            !loads_a_local(&emitted),
            "a propagated lookup pushes the constant, with no slot read: {emitted:?}"
        );
    }
}

#[test]
fn a_propagated_pooled_lookup_loads_a_constant() {
    let source = r#"
        def s = "hello"
        s
    "#;
    for emitted in emitted(source, true) {
        assert!(
            !loads_a_local(&emitted),
            "a propagated String lookup emits no LoadLocal: {emitted:?}"
        );
        assert!(
            emitted
                .code
                .iter()
                .any(|op| matches!(op, Bytecode::LoadConst(_))),
            "the propagated String is loaded from the constant pool: {emitted:?}"
        );
    }
}

#[test]
fn propagation_preserves_an_array_binding_value() {
    let source = r"
        def a = [1, 2, 3]
        a
    ";
    assert_eq!(
        run(source),
        Value::from(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn propagation_preserves_a_map_binding_value() {
    let expected: Value = [(MapKey::from("a"), Value::Int(1))].into_iter().collect();
    let source = r"
        def m = {a: 1}
        m
    ";
    assert_eq!(run(source), expected);
}

#[test]
fn a_propagated_array_lookup_loads_a_constant_not_the_local() {
    // Unlike a scalar or String literal, an Array literal's code is several
    // ops (each element pushed, then `MakeArray`), not one; propagation alone
    // does not recognize that as a constant. Only once folding has first
    // collapsed it to a single value does propagation have a constant to
    // record and inline.
    let source = r"
        def a = [1, 2, 3]
        a
    ";
    for emitted in Script::new(source).code_where(|o| o.constant_propagate && !o.constant_fold) {
        assert!(
            loads_a_local(&emitted),
            "propagated but not folded, the Array is still built and read as a local: {emitted:?}"
        );
    }
    for emitted in Script::new(source).code_where(|o| o.constant_propagate && o.constant_fold) {
        assert!(
            !loads_a_local(&emitted),
            "folded then propagated, the lookup pushes the constant, with no slot read: {emitted:?}"
        );
        assert_eq!(
            emitted.count(&Bytecode::MakeArray(3)),
            0,
            "the Array is not rebuilt at the use site: {emitted:?}"
        );
        assert!(
            emitted
                .code
                .iter()
                .any(|op| matches!(op, Bytecode::LoadConst(_))),
            "the propagated Array is loaded from the constant pool: {emitted:?}"
        );
    }
}

#[test]
fn a_propagated_map_lookup_loads_a_constant_not_the_local() {
    // Same reasoning as the Array case above: a Map literal needs folding
    // before propagation has a single constant value to work with.
    let source = r"
        def m = {a: 1, b: 2}
        m
    ";
    for emitted in Script::new(source).code_where(|o| o.constant_propagate && o.constant_fold) {
        assert!(
            !loads_a_local(&emitted),
            "folded then propagated, the lookup pushes the constant, with no slot read: {emitted:?}"
        );
        assert_eq!(
            emitted.count(&Bytecode::MakeMap(2)),
            0,
            "the Map is not rebuilt at the use site: {emitted:?}"
        );
    }
}

#[test]
fn an_empty_structure_propagates_without_folding() {
    // An empty literal is a single op, so it is a known constant as written.
    for source in [
        r"
        def a = []
        a
        ",
        r"
        def m = {}
        m
        ",
    ] {
        for emitted in emitted(source, true) {
            assert!(
                !loads_a_local(&emitted),
                "{source:?}: the lookup pushes the constant, with no slot read: {emitted:?}"
            );
        }
        for emitted in emitted(source, false) {
            assert!(loads_a_local(&emitted), "{source:?}: {emitted:?}");
        }
    }
    let arrays = r"
        def a = []
        [a, a + [1]]
    ";
    assert_eq!(run(arrays), run("[[], [1]]"));
    let maps = r"
        def m = {}
        [m, m + {k: 1}]
    ";
    assert_eq!(run(maps), run("[{}, {k: 1}]"));
}

// --- Destructured bindings ---

/// Destructures of a known value, reading back every part they bind, and the
/// value they read.
const KNOWN_DESTRUCTURES: [(&str, &str); 6] = [
    (
        r"
        def [a, b] = [1, 2]
        [b, a]
        ",
        "[2, 1]",
    ),
    (
        r"
        def [a, ...r] = [1, 2, 3]
        [a, r]
        ",
        "[1, [2, 3]]",
    ),
    (
        r"
        def [_, ...r] = [1]
        r
        ",
        "[]",
    ),
    (
        r"
        def {x, y: [z]} as m = {x: 1, y: [2]}
        [x, z, m]
        ",
        "[1, 2, {x: 1, y: [2]}]",
    ),
    (
        r"
        def [[a], {b}] = [[1], {b: 2}]
        [a, b]
        ",
        "[1, 2]",
    ),
    // A computed key reading an earlier part is itself known.
    (
        r#"
        def {a, [a]: b} = {a: "c", c: 3}
        [a, b]
        "#,
        r#"["c", 3]"#,
    ),
];

#[test]
fn propagation_preserves_destructured_values() {
    for (source, expected) in KNOWN_DESTRUCTURES {
        assert_eq!(run(source), run(expected), "{source:?}");
    }
}

#[test]
fn each_part_of_a_known_value_propagates() {
    // A structure literal is a known value once folded.
    for (source, _) in KNOWN_DESTRUCTURES {
        for emitted in Script::new(source).code_where(|o| o.constant_propagate && o.constant_fold) {
            assert!(
                !loads_a_local(&emitted),
                "{source:?}: every lookup pushes its part's constant: {emitted:?}"
            );
        }
    }
}

#[test]
fn a_propagated_part_folds_where_it_is_used() {
    let source = r"
        def [a, {b}] = [6, {b: 7}]
        a * b
    ";
    for emitted in Script::new(source).code_where(|o| o.constant_propagate && o.constant_fold) {
        assert_eq!(emitted.count(&Bytecode::Multiply), 0, "{emitted:?}");
        assert_eq!(emitted.count(&Bytecode::PushInt(42)), 1, "{emitted:?}");
    }
    assert_eq!(run(source), Value::Int(42));
}

#[test]
fn a_part_of_a_runtime_value_is_not_propagated() {
    for source in [
        r"
        def [a] = x
        a
        ",
        r"
        def {k} = y
        k
        ",
    ] {
        let script = Script::new(source).captures(&[("x", run("[1]")), ("y", run("{k: 1}"))]);
        assert_eq!(script.run(), Value::Int(1), "{source:?}");
        for emitted in script.code_where(|o| o.constant_propagate) {
            assert!(loads_a_local(&emitted), "{source:?}: {emitted:?}");
        }
    }
}

#[test]
fn a_known_value_of_the_wrong_shape_still_raises() {
    // No part is bound, so none propagates; the destructure raises as ever.
    for (source, message) in [
        (
            r"
            def [a, b] = [1]
            a
            ",
            "exactly 2 elements",
        ),
        (
            r"
            def [a, b, ...r] = [1]
            a
            ",
            "at least 2 elements",
        ),
        (
            r"
            def {a} = [1]
            a
            ",
            "expected a Map",
        ),
        (
            r"
            def [a] = {a: 1}
            a
            ",
            "expected an Array",
        ),
        (
            r"
            def {a, b} = {a: 1}
            a
            ",
            "no value at key 'b'",
        ),
    ] {
        let raised = raises(source);
        assert!(raised.contains(message), "{source:?}: {raised}");
    }
}

// --- Every pattern form in a def ---

const FOLD_AND_PROPAGATE: OptimizationOptions = UNOPTIMIZED
    .with(Optimization::ConstantFold, true)
    .with(Optimization::ConstantPropagate, true);

/// Whether the code makes a call, tail or not.
fn calls(emitted: &Emitted) -> bool {
    emitted
        .code
        .iter()
        .any(|op| matches!(op, Bytecode::Call(_) | Bytecode::TailCall(_)))
}

/// Assert `source` runs to `expected`, and that under folding and propagation
/// alone its lookups push constants, with no local slot read.
fn assert_propagates(source: &str, expected: &str) {
    assert_eq!(run(source), run(expected), "{source:?} is {expected}");
    let emitted = Script::new(source).code(FOLD_AND_PROPAGATE);
    assert!(
        !loads_a_local(&emitted),
        "{source:?}: every lookup pushes its part's constant: {emitted:?}"
    );
}

#[test]
fn a_type_constrained_name_propagates() {
    assert_propagates(
        r"
        def x is Int = 5
        x
        ",
        "5",
    );
    assert_propagates(
        r"
        def [a, b is String] = [1, 'two']
        [b, a]
        ",
        "['two', 1]",
    );
    assert_propagates(
        r"
        def {host is String} = {host: 'h'}
        host
        ",
        "'h'",
    );
}

#[test]
fn a_part_beside_a_literal_check_propagates() {
    assert_propagates(
        r"
        def [a, 1] = [7, 1]
        a
        ",
        "7",
    );
    assert_propagates(
        r"
        def {k: 'v', w} = {k: 'v', w: 2}
        w
        ",
        "2",
    );
    assert_propagates(
        r"
        def [a, -1, null] = [7, -1, null]
        a
        ",
        "7",
    );
}

#[test]
fn a_part_beside_a_pin_propagates() {
    assert_propagates(
        r"
        def [a, (1 + 1)] = [7, 2]
        a
        ",
        "7",
    );
    let n = r"
        def n = 2
        def [a, (n)] = [7, 2]
        a
    ";
    assert_propagates(n, "7");
}

#[test]
fn an_alternative_propagates_the_branch_that_matched() {
    let source = r"
        def [a, 1] | [1, a] = [1, 2]
        a
    ";
    assert_propagates(source, "2");
    let emitted = Script::new(source).code(FOLD_AND_PROPAGATE);
    // The matched def folds to binding the second branch's part, and the
    // lookup pushes that part.
    assert_eq!(
        emitted.count(&Bytecode::PushInt(2)),
        2,
        "the second branch's binding is bound and pushed: {emitted:?}"
    );
    assert_eq!(
        emitted.count(&Bytecode::PushInt(1)),
        0,
        "the first branch's part is never bound: {emitted:?}"
    );
    assert_propagates(
        r"
        def [a, 1] | [1, a] = [3, 1]
        a
        ",
        "3",
    );
    assert_propagates(
        r"
        def {a} | [a] = [5]
        a
        ",
        "5",
    );
}

#[test]
fn a_nested_alternative_propagates() {
    assert_propagates(
        r"
        def [a, [b] | b] = [1, 2]
        [a, b]
        ",
        "[1, 2]",
    );
    assert_propagates(
        r"
        def [a, [b, 1 | 2] | b] = [1, [3, 2]]
        [a, b]
        ",
        "[1, 3]",
    );
    assert_propagates(
        r"
        def {k: [a] | {a}} = {k: {a: 4}}
        a
        ",
        "4",
    );
}

#[test]
fn a_function_bound_deep_in_a_pattern_folds_its_call() {
    for source in [
        r"
        def {g: [f]} = {g: [fn x -> x * x]}
        f(42)
        ",
        r"
        def [{f}] = [{f: fn x -> x * x}]
        f(42)
        ",
        r"
        def [_, f is Function] = [0, fn x -> x * x]
        f(42)
        ",
    ] {
        assert_eq!(run(source), Value::Int(1764), "{source:?}");
        let folded = Script::new(source).code(FOLD_AND_PROPAGATE);
        assert!(!calls(&folded), "{source:?}: the call folds: {folded:?}");
        assert_eq!(
            folded.code.last(),
            Some(&Bytecode::PushInt(1764)),
            "{source:?}: the result is pushed directly: {folded:?}"
        );
    }
}

#[test]
fn a_function_bound_in_an_alternative_folds_its_call() {
    for (source, result) in [
        (
            r"
            def [h] | h = fn x -> x + 1
            h(1)
            ",
            2,
        ),
        (
            r"
            def [h] | h = [fn x -> x + 10]
            h(1)
            ",
            11,
        ),
        (
            r"
            def {k: [h] | h} = {k: fn x -> x + 3}
            h(1)
            ",
            4,
        ),
    ] {
        assert_eq!(run(source), Value::Int(result), "{source:?}");
        let folded = Script::new(source).code(FOLD_AND_PROPAGATE);
        assert!(!calls(&folded), "{source:?}: the call folds: {folded:?}");
        assert_eq!(
            folded.code.last(),
            Some(&Bytecode::PushInt(result)),
            "{source:?}: the result is pushed directly: {folded:?}"
        );
    }
}

#[test]
fn a_function_bound_in_a_pattern_needs_both_folding_and_propagation() {
    let script = Script::new(
        r"
        def {g: [f]} = {g: [fn x -> x * x]}
        f(42)
    ",
    );
    for options in [
        UNOPTIMIZED.with(Optimization::ConstantFold, true),
        UNOPTIMIZED.with(Optimization::ConstantPropagate, true),
    ] {
        let emitted = script.code(options);
        assert!(calls(&emitted), "the call is made at runtime: {emitted:?}");
    }
}

#[test]
fn a_known_value_that_fails_the_pattern_records_nothing() {
    for (source, message) in [
        (
            r"
            def [a, 1] = [7, 2]
            a
            ",
            "Cannot destructure: expected `1`",
        ),
        (
            r"
            def x is String = 5
            x
            ",
            "Cannot destructure: expected a String",
        ),
        (
            r"
            def [a, (1 + 1)] = [7, 3]
            a
            ",
            "Cannot destructure: expected the value of `(1 + 1)`",
        ),
        (
            r"
            def [a, 1] | [1, a] = [3, 4]
            a
            ",
            "Cannot destructure: the value matches no alternative",
        ),
    ] {
        assert_eq!(raises(source), message, "{source:?}");
        let emitted = Script::new(source).code(FOLD_AND_PROPAGATE);
        assert!(
            loads_a_local(&emitted),
            "{source:?}: with nothing recorded, the lookup is kept: {emitted:?}"
        );
        assert!(
            emitted.count(&Bytecode::ProduceError) > 0,
            "{source:?}: the failing check is kept: {emitted:?}"
        );
    }
}

#[test]
fn a_known_map_missing_a_key_records_nothing() {
    let source = r"
        def {a} = {b: 1}
        a
    ";
    assert_eq!(
        raises(source),
        "Map has no value at key 'a'; did you mean 'b'?"
    );
    let emitted = Script::new(source).code(FOLD_AND_PROPAGATE);
    assert!(
        loads_a_local(&emitted),
        "with nothing recorded, the lookup is kept: {emitted:?}"
    );
}

#[test]
fn a_function_that_fails_its_pattern_is_not_called_at_compile_time() {
    let source = r"
        def [f, 1] = [fn x -> x * x, 2]
        f(42)
    ";
    assert_eq!(
        raises(source),
        "Cannot destructure: expected `1`",
        "the pattern fails before the call"
    );
    let emitted = Script::new(source).code(FOLD_AND_PROPAGATE);
    assert!(calls(&emitted), "the call is kept: {emitted:?}");
}

#[test]
fn a_failing_pattern_raises_only_if_its_code_executes() {
    let source = r"
        defn never() -> {
            def [a, 1] = [7, 2]
            a
        }
        if false: never() else: 'skipped'
    ";
    assert_eq!(run(source), Value::from("skipped"));
}

#[test]
fn a_pin_of_an_earlier_part_propagates() {
    // The pin reads a name its own pattern binds, which is known once the
    // pattern has run on the known value.
    assert_propagates(
        r"
        def [a, (a)] = [7, 7]
        a
        ",
        "7",
    );
}

#[test]
fn a_def_in_a_pattern_may_read_an_earlier_part() {
    // The inner def's value reads `a` before the outer pattern has run, so it
    // cannot be evaluated alone; nor, as its `z` is then unknown, can the outer
    // pattern. Both are left for runtime, under every optimization.
    let source = r#"
        def {a, x: (do {
            def [z] = [a]
            z
        }), [a]: b} = {a: "c", x: "c", c: 3}
        [a, b]
    "#;
    assert_eq!(run(source), run(r#"["c", 3]"#));
}

/// Whether the code tests a value's shape, or raises.
fn checks_shape(emitted: &Emitted) -> bool {
    emitted.code.iter().any(|op| {
        matches!(
            op,
            Bytecode::TypeTest(_)
                | Bytecode::TestArrayLenExact(_)
                | Bytecode::TestArrayLenAtLeast(_)
                | Bytecode::ExtractKey
                | Bytecode::ExtractConstKey(_)
                | Bytecode::ProduceError
        )
    })
}

#[test]
fn a_matched_def_folds_to_binding_its_parts() {
    let source = r"
        def {x, y: [z is Int]} | [x, z] = {x: 1, y: [2]}
        [x, z]
    ";
    assert_eq!(run(source), run("[1, 2]"));
    let fold = UNOPTIMIZED.with(Optimization::ConstantFold, true);
    for options in [fold, FOLD_AND_PROPAGATE] {
        let emitted = Script::new(source).code(options);
        assert!(
            !checks_shape(&emitted),
            "the match was proven at compile time, so no check is left: {emitted:?}"
        );
    }
    let emitted = Script::new(source).code(FOLD_AND_PROPAGATE);
    assert!(!loads_a_local(&emitted), "its parts propagate: {emitted:?}");
}

#[test]
fn a_matched_def_binding_a_function_keeps_its_code() {
    // A function cannot be a constant, so the def is not replaced; the function
    // is still known to later folds.
    let source = r"
        def {f} = {f: fn x -> x * x}
        f(42)
    ";
    assert_eq!(run(source), Value::Int(1764));
    let emitted = Script::new(source).code(FOLD_AND_PROPAGATE);
    assert!(checks_shape(&emitted), "the def is kept: {emitted:?}");
    assert!(!calls(&emitted), "the call folds: {emitted:?}");
    assert_eq!(emitted.count(&Bytecode::PushInt(1764)), 1, "{emitted:?}");
}

#[test]
fn a_def_of_a_runtime_value_keeps_its_code() {
    let source = r"
        def {x} = y
        x
    ";
    let script = Script::new(source).captures(&[("y", run("{x: 1}"))]);
    assert_eq!(script.run(), Value::Int(1));
    let emitted = script.code(FOLD_AND_PROPAGATE);
    assert!(checks_shape(&emitted), "{emitted:?}");
    assert!(loads_a_local(&emitted), "{emitted:?}");
}
