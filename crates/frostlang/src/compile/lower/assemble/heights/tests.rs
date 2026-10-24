//! Heights of code that nothing reaches.
//!
//! Compiled code need not hold any, so these walk hand-written bytecode. The
//! other cases are tested from compiled code, in `tests/emitted_debug_info.rs`.

use crate::Arity;
use crate::bytecode::Bytecode::{self, *};
use crate::compile::lower::assemble::heights::entry_heights;

/// The `Bytecode::Pop` alias, an associated constant `use Bytecode::*` cannot
/// import.
#[allow(non_upper_case_globals)]
const Pop: Bytecode = Bytecode::Pop;

/// The entry heights of `code`, a function taking no arguments. Each case starts
/// by popping the function's own value, as compiled code does: the walk checks
/// that a function ends with at most its result.
fn heights(code: &[Bytecode]) -> Vec<Option<u64>> {
    entry_heights(code, Arity::Exact(0))
}

#[test]
fn code_after_an_unconditional_jump_has_no_height() {
    assert_eq!(
        heights(&[Pop, PushInt(1), Jump(1), PushInt(2)]),
        [Some(1), Some(0), Some(1), None],
        "the jump skips the last push, and nothing else reaches it"
    );
}

#[test]
fn code_after_a_raise_has_no_height() {
    assert_eq!(
        heights(&[Pop, PushInt(1), ProduceError, PushNull]),
        [Some(1), Some(0), Some(1), None],
        "the raise never goes on, and nothing jumps past it"
    );
}

#[test]
fn code_a_jump_lands_on_is_reached_again() {
    assert_eq!(
        heights(&[Pop, PushInt(1), Jump(2), PushInt(2), Pop, PushNull, Pop]),
        [Some(1), Some(0), Some(1), None, None, Some(1), Some(2)],
        "only the code the jump skips has no height"
    );
}

#[test]
fn a_jump_nothing_reaches_reaches_nothing() {
    assert_eq!(
        heights(&[Pop, PushInt(1), Jump(2), Jump(0), PushNull, Pop]),
        [Some(1), Some(0), Some(1), None, None, Some(1)],
        "the skipped jump lands only on code the first jump also skips"
    );
}

#[test]
fn a_rewind_nothing_reaches_has_no_height() {
    assert_eq!(
        heights(&[Pop, MarkStack, PushInt(1), DropMark, Jump(1), RewindToMark]),
        [Some(1), Some(0), Some(0), Some(1), Some(1), None],
        "a rewind's height comes from its mark only when something reaches it"
    );
}
