//! Stack heights: the operand stack height on entry to each instruction of an
//! assembled function, found by walking its bytecode with each instruction's
//! [`stack_effect`](Bytecode::stack_effect).
//!
//! Every jump goes forward, so one walk in code order sees every edge into an
//! instruction before reaching it. The walk tracks the stack marks too: a
//! `RewindToMark` returns the height to its mark's.
//!
//! Code that nothing reaches has no height.
//!
//! The compiler's IR is well-formed, so any inconsistency is a compiler bug: a
//! debug assertion catches it, and a release build goes on with a best-effort
//! height.

use crate::Arity;
use crate::bytecode::{Bytecode, OpDebugInfo};
use crate::vm::{from_operand, to_operand};

/// The stack between instructions: its height, and the height each live mark
/// recorded, innermost last.
#[derive(Clone, Debug, PartialEq, Eq)]
struct State {
    height: u64,
    marks: Vec<u64>,
}

/// The operand stack height on entry to each instruction of `code`, a function
/// of `arity`, as [`OpDebugInfo::stack_height`] records it.
pub(super) fn entry_heights(code: &[Bytecode], arity: Arity) -> Vec<Option<u64>> {
    // The callee and its arguments, as the calling convention lays them out.
    let entry = match arity {
        Arity::Exact(params) => to_operand(params) + 1,
        Arity::AtLeast(params) => to_operand(params) + 2,
        Arity::Between(..) => unreachable!("the compiler makes no function of a Between arity"),
    };

    // The states each jump brings to its target, by target; the last is the
    // function's end.
    let mut incoming: Vec<Vec<State>> = vec![Vec::new(); code.len() + 1];
    let mut fall_through = Some(State {
        height: entry,
        marks: Vec::new(),
    });
    let mut heights = Vec::with_capacity(code.len());

    for (pc, &op) in code.iter().enumerate() {
        let arriving: Vec<State> = fall_through
            .take()
            .into_iter()
            .chain(std::mem::take(&mut incoming[pc]))
            .collect();
        let Some((height, mut state)) = merge(&arriving, op) else {
            heights.push(None);
            continue;
        };
        heights.push(height);

        match op {
            Bytecode::MarkStack => state.marks.push(state.height),
            Bytecode::DropMark => {
                let mark = state.marks.pop();
                debug_assert!(mark.is_some(), "DropMark at {pc} has no mark");
            }
            Bytecode::RewindToMark => {
                let mark = state.marks.pop();
                debug_assert!(mark.is_some(), "RewindToMark at {pc} has no mark");
                state.height = mark.unwrap_or(state.height);
            }
            _ => {}
        }
        if let Some(effect) = op.stack_effect() {
            let after = effect.apply(state.height);
            debug_assert!(
                after.is_some(),
                "{op:?} at {pc} underflows a stack of {}",
                state.height
            );
            state.height = after.unwrap_or(0);
        }

        let (target, falls_through) = successors(op, pc);
        if let Some(target) = target {
            incoming[target].push(state.clone());
        }
        if falls_through {
            fall_through = Some(state);
        }
    }

    if cfg!(debug_assertions) {
        // The function returns by running off its end, with only its result:
        // nothing at all, for a top level of only `def`s.
        let ending = fall_through
            .into_iter()
            .chain(incoming.pop().into_iter().flatten());
        for state in ending {
            debug_assert!(
                state.height <= 1 && state.marks.is_empty(),
                "a function must end with only its result, and no mark: {state:?}"
            );
        }
    }
    heights
}

/// The entry height of `op`, reached in each of the `arriving` states, and the
/// state to run it from, or `None` if nothing reaches it. The height is `None`
/// where the states disagree on it.
fn merge(arriving: &[State], op: Bytecode) -> Option<(Option<u64>, State)> {
    let first = arriving.first()?;
    debug_assert!(
        arriving.iter().all(|state| state.marks == first.marks),
        "{op:?} is reached with different stack marks: {arriving:?}"
    );
    let agreed = arriving.iter().all(|state| state.height == first.height);
    // A rewind restores its mark's height, whatever the height before it.
    debug_assert!(
        agreed || op == Bytecode::RewindToMark,
        "{op:?} is reached at different stack heights: {arriving:?}"
    );
    Some((agreed.then_some(first.height), first.clone()))
}

/// The jump target of `op` at `pc`, if it jumps, and whether it can go on to the
/// next instruction.
fn successors(op: Bytecode, pc: usize) -> (Option<usize>, bool) {
    let target = |offset| Some(pc + 1 + from_operand(offset));
    match op {
        Bytecode::Jump(offset) => (target(offset), false),
        Bytecode::JumpIfTrue(offset)
        | Bytecode::JumpIfFalse(offset)
        | Bytecode::PeekJumpIfTrue(offset)
        | Bytecode::PeekJumpIfFalse(offset) => (target(offset), true),
        // It raises, so never goes on.
        Bytecode::ProduceError => (None, false),
        _ => (None, true),
    }
}
