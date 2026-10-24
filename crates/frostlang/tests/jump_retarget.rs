//! Jump retargeting (`jump_retarget`): a jump that lands on another jump whose
//! outcome is already known on arrival goes straight on to where that one leads.
//!
//! The harness runs each behavioral case under every optimization permutation,
//! with debug info, so a debug build also checks the stack heights of every path.
//! The code assertions pin the options they are about.

use crate::script;

use frostlang::Value;
use frostlang::bytecode::Bytecode;
use frostlang::compile::{Optimization, OptimizationOptions};
use script::{Emitted, Script, UNOPTIMIZED};

const RETARGET: OptimizationOptions = UNOPTIMIZED.with(Optimization::JumpRetarget, true);

/// Sources over the runtime values `a`, `b`, and `c`, each with jumps that land
/// on jumps when unoptimized.
const SOURCES: &[&str] = &[
    "[if a: (if b: 1 else: 2) else: 3]",
    "[if a: (if b: (if c: 1 else: 2) else: 3) else: 4]",
    "[if a: 1 elif b: (if c: 2 else: 3) else: 4]",
    "[match a { 1 => (if b: 2 else: 3), _ => c }]",
    "[a and b and c]",
    "[a or b or c]",
    "[(a and b) or c]",
    "[(a or b) and c]",
    "[if c: (a and b) else: 0]",
    "[if c: (a or b) else: 0]",
    "[if c: ((a and b) or a) else: 0]",
    "[if a and b: 1 else: 2]",
    "[if a or b: 1 else: 2]",
    "[if (a and b) or c: 1 else: 2]",
    "[if (a or b) and c: 1 else: 2]",
    "[if a and b and c: a else: c]",
    "(fn x, y, z -> if x: (if y: [1] else: [2]) else: z)(a, b, c)",
];

/// The values each of `a`, `b`, and `c` takes: falsy and truthy, of several types.
fn values() -> [Value; 5] {
    [
        Value::Bool(false),
        Value::Null,
        Value::Bool(true),
        Value::Int(0),
        Value::from("s"),
    ]
}

/// The script for `source` with `a`, `b`, and `c` captured.
fn with_values(source: &str, [a, b, c]: [Value; 3]) -> Script {
    Script::new(source).captures(&[("a", a), ("b", b), ("c", c)])
}

/// `source` compiled under `optimization`, with every value `null`.
fn compiled(source: &str, optimization: OptimizationOptions) -> Emitted {
    with_values(source, [Value::Null, Value::Null, Value::Null]).code(optimization)
}

/// How many instructions `op` skips, if it is a jump.
fn jump_offset(op: Bytecode) -> Option<u64> {
    match op {
        Bytecode::Jump(n)
        | Bytecode::JumpIfTrue(n)
        | Bytecode::JumpIfFalse(n)
        | Bytecode::PeekJumpIfTrue(n)
        | Bytecode::PeekJumpIfFalse(n) => Some(n),
        _ => None,
    }
}

/// Whether `op` is a jump of any kind.
fn is_jump(op: Bytecode) -> bool {
    jump_offset(op).is_some()
}

/// Whether `op` is a jump that leaves the value it tests.
fn is_peek_jump(op: Bytecode) -> bool {
    matches!(
        op,
        Bytecode::PeekJumpIfTrue(_) | Bytecode::PeekJumpIfFalse(_)
    )
}

/// Each jump in `code`, beside the instruction it lands on (`None` past the end).
fn landings(code: &[Bytecode]) -> Vec<(Bytecode, Option<Bytecode>)> {
    code.iter()
        .enumerate()
        .filter_map(|(pc, &op)| {
            let skipped = usize::try_from(jump_offset(op)?).expect("a small offset");
            Some((op, code.get(pc + 1 + skipped).copied()))
        })
        .collect()
}

/// The jumps in `code` that land on a jump, of any kind, beside it.
fn jumps_onto_jumps(code: &[Bytecode]) -> Vec<(Bytecode, Bytecode)> {
    landings(code)
        .into_iter()
        .filter_map(|(jump, landing)| landing.filter(|&op| is_jump(op)).map(|op| (jump, op)))
        .collect()
}

/// The jumps in `code` that land on a jump whose outcome they already know: an
/// unconditional one, or, for a peek jump, another peek jump. A peek jump onto
/// a consuming one is left out: whether it fuses depends on its `Pop`.
fn retargetable_jumps(code: &[Bytecode]) -> Vec<(Bytecode, Bytecode)> {
    jumps_onto_jumps(code)
        .into_iter()
        .filter(|&(jump, landing)| {
            matches!(landing, Bytecode::Jump(_)) || (is_peek_jump(jump) && is_peek_jump(landing))
        })
        .collect()
}

// Behavior, under every permutation.

#[test]
fn retargeting_never_changes_a_result() {
    for source in SOURCES {
        for a in values() {
            for b in values() {
                for c in values() {
                    // The harness checks every permutation against the first.
                    with_values(source, [a.clone(), b.clone(), c]).run();
                }
            }
        }
    }
}

// Code shape, with only this option on.

#[test]
fn no_jump_lands_on_a_jump_whose_outcome_it_knows() {
    // Fusing a peek jump onto a consuming test is checked by the shape tests
    // further down.
    for source in SOURCES {
        // The lambda source's jumps are in its own function.
        let function_code = |optimization| {
            let emitted = compiled(source, optimization);
            if emitted.nested_count() == 0 {
                emitted
            } else {
                emitted.nested(0)
            }
        };
        let before = function_code(UNOPTIMIZED);
        assert!(
            !jumps_onto_jumps(&before.code).is_empty(),
            "{source:?} lands a jump on a jump when unoptimized: {before:?}"
        );
        let after = function_code(RETARGET);
        assert_eq!(retargetable_jumps(&after.code), [], "{source:?}: {after:?}");
    }
}

#[test]
fn a_nested_result_jumps_straight_to_the_outer_end() {
    let emitted = compiled("[if a: (if b: 1 else: 2) else: 3]", RETARGET);
    let a = emitted.slot_named("a");
    let b = emitted.slot_named("b");
    assert_eq!(
        emitted.code,
        [
            Bytecode::Pop,
            Bytecode::LoadLocal(a),
            Bytecode::JumpIfFalse(6),
            Bytecode::LoadLocal(b),
            Bytecode::JumpIfFalse(2),
            Bytecode::PushInt(1),
            // Past the inner `else` and the outer one both.
            Bytecode::Jump(3),
            Bytecode::PushInt(2),
            Bytecode::Jump(1),
            Bytecode::PushInt(3),
            Bytecode::MakeArray(1),
        ],
        "{emitted:?}"
    );
}

#[test]
fn a_result_nested_twice_jumps_straight_to_the_outermost_end() {
    let emitted = compiled(
        "[if a: (if b: (if c: 1 else: 2) else: 3) else: 4]",
        RETARGET,
    );
    let ends: Vec<Option<Bytecode>> = landings(&emitted.code)
        .into_iter()
        .filter(|(jump, _)| matches!(jump, Bytecode::Jump(_)))
        .map(|(_, landing)| landing)
        .collect();
    assert_eq!(
        ends,
        [Some(Bytecode::MakeArray(1)); 3],
        "each branch's jump goes past every `else` at once: {emitted:?}"
    );
}

#[test]
fn an_if_on_and_or_or_tests_each_operand_once() {
    // Each operand is tested and discarded by one consuming jump: no peek
    // jump, and no `Pop` after one.
    let cases = [
        // A falsy `a` decides the `and` false: on to the `else`.
        ("[if a and b: 1 else: 2]", Bytecode::JumpIfFalse(4)),
        // A truthy `a` decides the `or` true: on to the `then`.
        ("[if a or b: 1 else: 2]", Bytecode::JumpIfTrue(2)),
    ];
    for (source, first_test) in cases {
        let emitted = compiled(source, RETARGET);
        let (a, b) = (emitted.slot_named("a"), emitted.slot_named("b"));
        assert_eq!(
            emitted.code,
            [
                Bytecode::Pop,
                Bytecode::LoadLocal(a),
                first_test,
                Bytecode::LoadLocal(b),
                Bytecode::JumpIfFalse(2),
                Bytecode::PushInt(1),
                Bytecode::Jump(1),
                Bytecode::PushInt(2),
                Bytecode::MakeArray(1),
            ],
            "{source:?}: {emitted:?}"
        );
    }
}

#[test]
fn a_chain_of_the_same_operator_jumps_once_to_its_end() {
    for source in ["[a and b and c]", "[a or b or c]"] {
        let emitted = compiled(source, RETARGET);
        let ends: Vec<Option<Bytecode>> = landings(&emitted.code)
            .into_iter()
            .map(|(_, landing)| landing)
            .collect();
        assert_eq!(
            ends,
            [Some(Bytecode::MakeArray(1)); 2],
            "{source:?}: each peek jump lands past the whole chain: {emitted:?}"
        );
    }
}

#[test]
fn an_opposite_operator_is_jumped_past() {
    // On a falsy `a`, the `and`'s peek jump lands on the `or`'s, which falls
    // through on that value. So it goes just past it, onto the `Pop` that
    // discards `a` before `c`.
    let emitted = compiled("[(a and b) or c]", RETARGET);
    let first = landings(&emitted.code)
        .first()
        .copied()
        .expect("the `and` jumps");
    assert!(
        matches!(first, (Bytecode::PeekJumpIfFalse(_), Some(Bytecode::Pop))),
        "{first:?} in {emitted:?}"
    );
}

#[test]
fn a_pop_another_jump_lands_on_is_kept() {
    // A truthy `a` goes past the `and`'s peek jump onto its `Pop`, so that
    // peek jump and `Pop` stay, not fused with the `if`'s test.
    let emitted = compiled("[if (a or b) and c: 1 else: 2]", RETARGET);
    assert!(
        emitted.code.windows(2).any(|pair| {
            matches!(pair[0], Bytecode::PeekJumpIfFalse(_)) && pair[1] == Bytecode::Pop
        }),
        "the `and`'s peek jump and `Pop` stay: {emitted:?}"
    );
    assert!(
        landings(&emitted.code).iter().any(|&(jump, landing)| {
            matches!(jump, Bytecode::PeekJumpIfTrue(_)) && landing == Some(Bytecode::Pop)
        }),
        "the `or` lands on the `and`'s `Pop`: {emitted:?}"
    );
}
