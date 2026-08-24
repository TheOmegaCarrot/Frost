//! Tests for the stack-mark trio: `MarkStack`, `DropMark`, `RewindToMark`.
//!
//! The marks are a per-frame side stack of saved operand-stack heights, with no
//! match awareness of their own:
//!   `MarkStack`    saves the current height (no operand-stack effect).
//!   `DropMark`     discards the most recent saved height, committing whatever was
//!                  pushed since (no operand-stack effect).
//!   `RewindToMark` pops the most recent saved height and truncates the operand
//!                  stack back to it.
//!
//! Only `RewindToMark` moves the operand stack, so every test folds the residual
//! stack into a single Array with `MakeArray` and reads it as the tail value. Each
//! program balances its marks, since an unbalanced mark trips a debug assertion at
//! return.

mod common;

use common::run;
use frost_runtime::Value;

use Bytecode::*;
use frost_runtime::Bytecode;

/// An Array `Value` of the given ints, matching what `MakeArray` folds `PushInt`s into.
fn iarr(xs: &[i64]) -> Value {
    Value::from(xs.iter().copied().map(Value::from).collect::<Vec<_>>())
}

// ---- RewindToMark: truncates above the mark, keeps below ----

#[test]
fn rewind_drops_everything_pushed_after_the_mark() {
    // Mark at height 2, push two more, rewind: back to the first two.
    let tail = run(vec![
        PushInt(1),
        PushInt(2),
        MarkStack,
        PushInt(3),
        PushInt(4),
        RewindToMark,
        MakeArray(2),
    ])
    .tail()
    .clone();
    assert_eq!(tail, iarr(&[1, 2]));
}

#[test]
fn rewind_keeps_values_below_the_mark_untouched() {
    // The two values beneath the mark survive; only the three above are cut.
    let tail = run(vec![
        PushInt(10),
        PushInt(20),
        MarkStack,
        PushInt(30),
        PushInt(40),
        PushInt(50),
        RewindToMark,
        MakeArray(2),
    ])
    .tail()
    .clone();
    assert_eq!(tail, iarr(&[10, 20]));
}

#[test]
fn rewind_with_nothing_pushed_since_the_mark_is_a_stack_no_op() {
    // Mark and immediately rewind: the height is unchanged.
    let tail = run(vec![PushInt(7), MarkStack, RewindToMark, MakeArray(1)])
        .tail()
        .clone();
    assert_eq!(tail, iarr(&[7]));
}

#[test]
fn rewind_can_empty_the_stack_back_to_the_base() {
    // Mark on an empty stack, push, rewind to nothing, then a lone sentinel.
    let tail = run(vec![
        MarkStack,
        PushInt(9),
        PushInt(8),
        RewindToMark,
        PushInt(42),
    ])
    .tail()
    .clone();
    assert_eq!(tail, Value::from(42));
}

// ---- DropMark: commits, never truncates ----

#[test]
fn drop_mark_leaves_the_operand_stack_intact() {
    // The value pushed after the mark survives, because DropMark only discards
    // the saved height.
    let tail = run(vec![
        PushInt(1),
        PushInt(2),
        MarkStack,
        PushInt(3),
        DropMark,
        MakeArray(3),
    ])
    .tail()
    .clone();
    assert_eq!(tail, iarr(&[1, 2, 3]));
}

// ---- LIFO nesting ----

#[test]
fn nested_marks_rewind_in_lifo_order() {
    // Two marks; the inner rewind uses the newer mark, the outer rewind the older.
    let tail = run(vec![
        PushInt(1),
        MarkStack, // outer: height 1
        PushInt(2),
        MarkStack, // inner: height 2
        PushInt(3),
        RewindToMark, // -> [1, 2]
        RewindToMark, // -> [1]
        MakeArray(1),
    ])
    .tail()
    .clone();
    assert_eq!(tail, iarr(&[1]));
}

#[test]
fn drop_mark_commits_the_inner_mark_so_the_outer_governs_the_next_rewind() {
    // DropMark removes the inner height without truncating; the following
    // RewindToMark therefore restores to the outer mark.
    let tail = run(vec![
        PushInt(1),
        MarkStack, // outer: height 1
        PushInt(2),
        MarkStack, // inner: height 2
        PushInt(3),
        DropMark,     // discard inner; stack stays [1, 2, 3]
        RewindToMark, // -> outer: [1]
        MakeArray(1),
    ])
    .tail()
    .clone();
    assert_eq!(tail, iarr(&[1]));
}

// ---- Underflow guards (always-on, both build profiles) ----

#[test]
#[should_panic(expected = "MARKS UNDERFLOW")]
fn rewind_with_no_saved_mark_panics() {
    let _ = run(vec![RewindToMark]);
}

#[test]
#[should_panic(expected = "MARKS UNDERFLOW")]
fn drop_mark_with_no_saved_mark_panics() {
    let _ = run(vec![DropMark]);
}

#[test]
fn marks_can_be_reused_across_sequential_cycles() {
    // Balanced mark/rewind pairs leave no residual mark state behind, so a second
    // cycle behaves identically to the first.
    let tail = run(vec![
        PushInt(1),
        MarkStack,
        PushInt(2),
        RewindToMark, // -> [1]
        MarkStack,
        PushInt(3),
        RewindToMark, // -> [1]
        MakeArray(1),
    ])
    .tail()
    .clone();
    assert_eq!(tail, iarr(&[1]));
}
