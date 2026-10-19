//! Dead code: drop a store nothing reads, and a value loaded only to be popped.
//!
//! With `dead_store_eliminate`, a dead store, a `DefLocal` of a local no later
//! code reads, becomes a `Pop`. Every jump goes forward, so any read a store
//! reaches comes after it. Counting every later read, reachable or not, also
//! keeps the store a read in unreachable code needs for its slot. An exported
//! local is never dead: its slot is read once the function finishes.
//!
//! With `discard_eliminate`, a [load](is_load) followed by a `Pop` is removed
//! along with the `Pop`. A load cannot raise, so the pair does nothing.
//!
//! Both run in one backward walk, so each feeds the other: a removed load of a
//! local is no read of it, which can make an earlier store dead, and a dead
//! store's `Pop` can remove the load of its value. Walking backward, each `Pop`
//! is held back as pending, and is emitted only on meeting an instruction it
//! cannot remove. A dead store's `Pop` keeps the store's span.

#[cfg(test)]
mod tests;

use std::collections::{HashSet, VecDeque};

use crate::bytecode::Bytecode;

use crate::compile::OptimizationOptions;
use crate::compile::lower::{Ir, LoweredFunction, Op};

pub(super) fn eliminate_dead_code(
    mut function: LoweredFunction,
    options: &OptimizationOptions,
) -> LoweredFunction {
    let mut read_later = HashSet::new();
    // In code order: the front is nearest the walk.
    let mut pending_pops: VecDeque<Ir> = VecDeque::new();
    // Built back to front.
    let mut reversed = Vec::with_capacity(function.code.len());
    for ir in function.code.into_iter().rev() {
        let is_dead_store = matches!(ir.op, Op::DefLocal(id)
            if options.dead_store_eliminate
                && !read_later.contains(&id)
                && !function.locals.is_exported(id));
        if matches!(ir.op, Op::Ready(Bytecode::Pop)) || is_dead_store {
            pending_pops.push_front(Op::Ready(Bytecode::Pop).at(ir.span));
        } else if !pending_pops.is_empty() && options.discard_eliminate && is_load(&ir.op) {
            // The load's value is what the first Pop after it pops.
            pending_pops.pop_front();
        } else {
            reversed.extend(pending_pops.drain(..).rev());
            if let Op::LoadLocal(id) | Op::ConsumeLocal(id) = ir.op {
                read_later.insert(id);
            }
            reversed.push(ir);
        }
    }
    reversed.extend(pending_pops.drain(..).rev());
    reversed.reverse();
    function.code = reversed;
    function
}

/// Whether `op` only pushes a value, `( -- x )`, and cannot raise.
fn is_load(op: &Op) -> bool {
    match op {
        Op::Ready(bytecode) => matches!(
            bytecode,
            Bytecode::PushNull
                | Bytecode::PushTrue
                | Bytecode::PushFalse
                | Bytecode::PushInt(_)
                | Bytecode::PushFloat(_)
                | Bytecode::PeekDown(_)
                | Bytecode::LoadGlobal(_)
                | Bytecode::MakeArray(0)
                | Bytecode::MakeMap(0)
        ),
        Op::Const(_) | Op::LoadLocal(_) | Op::ConsumeLocal(_) => true,
        // Creating a closure pops its captures.
        Op::Closure { compiled, .. } => compiled.num_captures == 0,
        Op::ConstKey { .. } | Op::DefLocal(_) | Op::Label(_) | Op::Jump { .. } => false,
    }
}
