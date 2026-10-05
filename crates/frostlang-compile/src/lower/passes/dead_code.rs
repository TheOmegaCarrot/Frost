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
//! cannot remove.

use std::collections::HashSet;
use std::iter;

use frostlang_runtime::Bytecode;

use crate::OptimizationOptions;
use crate::lower::{Ir, LoweredFunction};

pub(super) fn eliminate_dead_code(
    mut function: LoweredFunction,
    options: &OptimizationOptions,
) -> LoweredFunction {
    let mut read_later = HashSet::new();
    let mut pending_pops = 0;
    // Built back to front.
    let mut reversed = Vec::with_capacity(function.code.len());
    for ir in function.code.into_iter().rev() {
        let is_dead_store = matches!(ir, Ir::DefLocal(id)
            if options.dead_store_eliminate
                && !read_later.contains(&id)
                && !function.locals.is_exported(id));
        if matches!(ir, Ir::Ready(Bytecode::Pop)) || is_dead_store {
            pending_pops += 1;
        } else if pending_pops > 0 && options.discard_eliminate && is_load(&ir) {
            pending_pops -= 1;
        } else {
            reversed.extend(iter::repeat_n(Ir::Ready(Bytecode::Pop), pending_pops));
            pending_pops = 0;
            if let Ir::LoadLocal(id) | Ir::ConsumeLocal(id) = ir {
                read_later.insert(id);
            }
            reversed.push(ir);
        }
    }
    reversed.extend(iter::repeat_n(Ir::Ready(Bytecode::Pop), pending_pops));
    reversed.reverse();
    function.code = reversed;
    function
}

/// Whether `ir` only pushes a value, `( -- x )`, and cannot raise.
fn is_load(ir: &Ir) -> bool {
    match ir {
        Ir::Ready(bytecode) => matches!(
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
        Ir::Const(_) | Ir::LoadLocal(_) | Ir::ConsumeLocal(_) => true,
        // Creating a closure pops its captures.
        Ir::Closure { compiled, .. } => compiled.num_captures == 0,
        Ir::ConstKey { .. } | Ir::DefLocal(_) | Ir::Label(_) | Ir::Jump { .. } => false,
    }
}
