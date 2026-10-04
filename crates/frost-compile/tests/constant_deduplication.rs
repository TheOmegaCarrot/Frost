//! Constant deduplication: equal constants in a function share one pool entry,
//! however far apart their uses, except where a Float's sign of zero tells
//! them apart.
//!
//! Code-shape cases pin exactly the options they are about. Behavioral cases
//! run under every optimization permutation, so a wrongly shared constant shows
//! up as a permutation that disagrees.

mod common;

use common::{Emitted, Script, UNOPTIMIZED, run};
use frost_compile::{Optimization, OptimizationOptions};
use frost_runtime::{Bytecode, MapKey, Value};

const DEDUPLICATE: OptimizationOptions = UNOPTIMIZED.with(Optimization::DeduplicateConstants, true);

/// Folding builds structures into constants, giving deduplication more to share.
const FOLD_AND_DEDUPLICATE: OptimizationOptions = UNOPTIMIZED
    .with(Optimization::ConstantFold, true)
    .with(Optimization::DeduplicateConstants, true);

/// The constant-pool index of each `LoadConst` in `emitted`, in order.
fn loads(emitted: &Emitted) -> Vec<usize> {
    emitted
        .code
        .iter()
        .filter_map(|op| match *op {
            Bytecode::LoadConst(index) => Some(index),
            _ => None,
        })
        .collect()
}

/// The key-constant index of each constant-key op in `emitted`, in order.
fn key_loads(emitted: &Emitted) -> Vec<usize> {
    emitted
        .code
        .iter()
        .filter_map(|op| match *op {
            Bytecode::HardIndexMap(index)
            | Bytecode::TestConstKey(index)
            | Bytecode::ExtractConstKey(index) => Some(index),
            _ => None,
        })
        .collect()
}

// --- Equal constants share an entry ---

#[test]
fn equal_constants_share_one_entry() {
    let emitted = Script::new(r#"["abc", "xyz", "abc", "xyz", "abc"]"#).code(DEDUPLICATE);
    assert_eq!(
        emitted.constants(),
        [Value::from("abc"), Value::from("xyz")],
        "{emitted:?}"
    );
    assert_eq!(loads(&emitted), [0, 1, 0, 1, 0], "{emitted:?}");
}

#[test]
fn constants_far_apart_share_one_entry() {
    let source = r#"
        def greeting = "hello"
        def f = fn -> 1
        def g = fn -> 2
        [greeting, f(), g(), "hello"]
    "#;
    let emitted = Script::new(source).code(DEDUPLICATE);
    assert_eq!(emitted.constants(), [Value::from("hello")], "{emitted:?}");
    assert_eq!(loads(&emitted), [0, 0], "{emitted:?}");
}

#[test]
fn equal_maps_built_in_different_orders_share_one_entry() {
    let source = "fn x -> [x, {a: 1, b: [2]}, {b: [2], a: 1}]";
    let emitted = Script::new(source).code(FOLD_AND_DEDUPLICATE).nested(0);
    assert_eq!(emitted.constants().len(), 1, "{emitted:?}");
    assert_eq!(loads(&emitted), [0, 0], "{emitted:?}");
}

#[test]
fn equal_structures_share_one_entry() {
    // The parameter keeps the outer Array from folding; the rest fold to constants.
    let source = "fn x -> [x, [1, [2]], {a: [3]}, [1, [2]], {a: [3]}]";
    let emitted = Script::new(source).code(FOLD_AND_DEDUPLICATE).nested(0);
    assert_eq!(emitted.constants().len(), 2, "{emitted:?}");
    assert_eq!(loads(&emitted), [0, 1, 0, 1], "{emitted:?}");
}

#[test]
fn equal_map_keys_share_one_entry() {
    let emitted = Script::new("fn m -> [m.a, m.b, m.a, m.b]")
        .code(DEDUPLICATE)
        .nested(0);
    assert_eq!(
        emitted.key_constants(),
        [MapKey::from("a"), MapKey::from("b")],
        "{emitted:?}"
    );
    assert_eq!(key_loads(&emitted), [0, 1, 0, 1], "{emitted:?}");
}

#[test]
fn without_the_option_each_use_has_its_own_entry() {
    let emitted = Script::new(r#"fn m -> [m.a, m.a, "abc", "abc"]"#)
        .code(UNOPTIMIZED)
        .nested(0);
    assert_eq!(emitted.constants().len(), 2, "{emitted:?}");
    assert_eq!(emitted.key_constants().len(), 2, "{emitted:?}");
}

// --- Constants that differ stay apart ---

#[test]
fn unequal_constants_and_opposite_zeros_stay_apart() {
    for (a, b) in [
        ("[1]", "[1.0]"),
        (r#"["1"]"#, "[x'31']"),
        ("[[1]]", "[1]"),
        ("{a: 1}", "{a: 2}"),
        // Equal under `==`, but printed differently.
        ("[0.0]", "[-0.0]"),
        ("{a: 0.0}", "{a: -0.0}"),
        ("{[0.0]: 1}", "{[-0.0]: 1}"),
    ] {
        let source = format!("fn x -> [x, {a}, {b}]");
        let emitted = Script::new(&source).code(FOLD_AND_DEDUPLICATE).nested(0);
        assert_eq!(emitted.constants().len(), 2, "{source}: {emitted:?}");
    }
}

#[test]
fn map_keys_that_print_differently_stay_apart() {
    let source = r"
        fn m -> match m {
            {[0.0]: a} => a,
            {[-0.0]: b} => b,
            _ => 0
        }
    ";
    let emitted = Script::new(source).code(FOLD_AND_DEDUPLICATE).nested(0);
    let keys = emitted.key_constants();
    assert_eq!(keys.len(), 2, "{emitted:?}");
    let signs: Vec<bool> = keys
        .iter()
        .map(|key| match key {
            MapKey::Float(float) => float.get().is_sign_negative(),
            other => panic!("a Float key, not {other:?}"),
        })
        .collect();
    assert_eq!(signs, [false, true], "{emitted:?}");
}

// --- Sharing never changes a result ---

#[test]
fn opposite_zeros_keep_printing_differently() {
    let source = r"
        def show = fn x -> $'${[x, [0.0], [-0.0], {a: 0.0}, {a: -0.0}, {[0.0]: 1}, {[-0.0]: 1}]}'
        show(1)
    ";
    assert_eq!(
        run(source),
        Value::from(
            r#"[ 1, [ 0.0 ], [ -0.0 ], { ["a"]: 0.0 }, { ["a"]: -0.0 }, { [0.0]: 1 }, { [-0.0]: 1 } ]"#
        )
    );
}

#[test]
fn a_shared_constant_is_never_changed_through_one_of_its_uses() {
    // Each use builds on the constant; a use that changed it in place would
    // change what the others see.
    let source = r"
        def f = fn x -> [[1] + [x], [1] + [2], [1]]
        f(0)
    ";
    assert_eq!(run(source), run("[[1, 0], [1, 2], [1]]"));
}
