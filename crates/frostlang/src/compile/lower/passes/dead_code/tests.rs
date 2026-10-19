//! Tests for the dead-code pass: which `Pop` a load cancels, which span the
//! surviving `Pop`s keep, and how a dead store becomes a `Pop`.
//!
//! Every IR instruction carries a distinct span, so a test fails if the pass
//! cancels or re-spans the wrong instruction.

use crate::Arity;
use crate::bytecode::{Bytecode, Span};
use crate::compile::OptimizationOptions;
use crate::compile::lower::locals::{LocalInfo, LocalKind, Locals};
use crate::compile::lower::passes::dead_code::eliminate_dead_code;
use crate::compile::lower::{Ir, LocalId, LoweredFunction, Op};
use frostlang_parse::ast::SourceSpan;

const DISCARD_ONLY: OptimizationOptions = OptimizationOptions {
    discard_eliminate: true,
    ..OptimizationOptions::NONE
};

const DEAD_STORE_ONLY: OptimizationOptions = OptimizationOptions {
    dead_store_eliminate: true,
    ..OptimizationOptions::NONE
};

const BOTH: OptimizationOptions = OptimizationOptions {
    dead_store_eliminate: true,
    discard_eliminate: true,
    ..OptimizationOptions::NONE
};

fn span(start: u32, end: u32) -> Span {
    Span { start, end }
}

/// A function over `code` and `locals`, as the pass receives it.
fn function(code: Vec<Ir>, locals: Locals) -> LoweredFunction {
    LoweredFunction {
        name: "<test>".to_string(),
        origin: None,
        arity: Arity::Exact(0),
        code,
        locals,
        num_labels: 0,
        effectful: false,
        deduplicate_constants: false,
        debug_lines: None,
    }
}

/// Define a plain binding named `name`, `exported` or not.
fn define(locals: &mut Locals, name: &str, exported: bool) -> LocalId {
    locals
        .define(LocalInfo {
            name: name.to_string(),
            span: SourceSpan::default(),
            exported,
            constant: None,
            kind: LocalKind::Binding,
        })
        .expect("the test names are distinct")
}

/// Whether `ir` is a `DefLocal` of `local`, at exactly `expected_span`.
fn is_def_at(ir: &Ir, local: LocalId, expected_span: Span) -> bool {
    matches!(&ir.op, Op::DefLocal(id) if *id == local) && ir.span == expected_span
}

/// Whether `ir` is the plain bytecode `expected`, at exactly `expected_span`.
fn is_ready_at(ir: &Ir, expected: Bytecode, expected_span: Span) -> bool {
    matches!(&ir.op, Op::Ready(bytecode) if *bytecode == expected) && ir.span == expected_span
}

/// A `Pop` cancelled by a load is the nearest one after it, not the last one.
/// The surviving `Pop` keeps its own span.
#[test]
fn a_load_cancels_the_nearest_pop_and_the_rest_keep_their_spans() {
    let code = vec![
        Op::Ready(Bytecode::Add).at(span(0, 1)),
        Op::Ready(Bytecode::PushInt(7)).at(span(1, 2)),
        Op::Ready(Bytecode::Pop).at(span(10, 11)),
        Op::Ready(Bytecode::Pop).at(span(20, 21)),
    ];

    let result = eliminate_dead_code(function(code, Locals::new()), &DISCARD_ONLY).code;

    assert_eq!(result.len(), 2, "the load and one Pop are gone: {result:?}");
    assert!(
        is_ready_at(&result[0], Bytecode::Add, span(0, 1)),
        "the unrelated op is untouched: {result:?}"
    );
    assert!(
        is_ready_at(&result[1], Bytecode::Pop, span(20, 21)),
        "the nearer Pop (span 10..11) is the one cancelled, so the survivor is \
         the later one: {result:?}"
    );
}

/// Each load cancels its own nearest `Pop`, pairing from the inside out, so
/// nested pairs vanish whole.
#[test]
fn nested_loads_cancel_pops_from_the_inside_out() {
    let code = vec![
        Op::Ready(Bytecode::Add).at(span(0, 1)),
        Op::Ready(Bytecode::PushInt(1)).at(span(1, 2)),
        Op::Ready(Bytecode::PushInt(2)).at(span(2, 3)),
        Op::Ready(Bytecode::Pop).at(span(10, 11)),
        Op::Ready(Bytecode::Pop).at(span(20, 21)),
    ];

    let result = eliminate_dead_code(function(code, Locals::new()), &DISCARD_ONLY).code;

    assert_eq!(result.len(), 1, "both load/Pop pairs are gone: {result:?}");
    assert!(
        is_ready_at(&result[0], Bytecode::Add, span(0, 1)),
        "only the unrelated op remains: {result:?}"
    );
}

/// With `discard_eliminate` off, a load followed by `Pop`s is left as written,
/// each `Pop` keeping its span.
#[test]
fn pops_survive_untouched_without_discard_elimination() {
    let code = vec![
        Op::Ready(Bytecode::PushInt(7)).at(span(1, 2)),
        Op::Ready(Bytecode::Pop).at(span(10, 11)),
        Op::Ready(Bytecode::Pop).at(span(20, 21)),
    ];

    let result = eliminate_dead_code(function(code, Locals::new()), &DEAD_STORE_ONLY).code;

    assert_eq!(result.len(), 3, "nothing is removed: {result:?}");
    assert!(is_ready_at(&result[0], Bytecode::PushInt(7), span(1, 2)));
    assert!(
        is_ready_at(&result[1], Bytecode::Pop, span(10, 11)),
        "the first Pop keeps its span: {result:?}"
    );
    assert!(
        is_ready_at(&result[2], Bytecode::Pop, span(20, 21)),
        "the second Pop keeps its span: {result:?}"
    );
}

/// A `DefLocal` of a local nothing reads becomes a `Pop` carrying the store's
/// own span.
#[test]
fn a_dead_store_becomes_a_pop_with_the_stores_span() {
    let mut locals = Locals::new();
    let unread = define(&mut locals, "unread", false);
    let code = vec![
        Op::Ready(Bytecode::Add).at(span(0, 1)),
        Op::DefLocal(unread).at(span(5, 9)),
    ];

    let result = eliminate_dead_code(function(code, locals), &DEAD_STORE_ONLY).code;

    assert_eq!(result.len(), 2, "the store is replaced, not dropped");
    assert!(is_ready_at(&result[0], Bytecode::Add, span(0, 1)));
    assert!(
        is_ready_at(&result[1], Bytecode::Pop, span(5, 9)),
        "the Pop takes the dead store's span: {result:?}"
    );
}

/// With discard elimination also on, a dead store's `Pop` cancels the load of
/// its value, so the whole `def` vanishes.
#[test]
fn a_dead_store_of_a_load_vanishes_with_discard_elimination() {
    let mut locals = Locals::new();
    let unread = define(&mut locals, "unread", false);
    let code = vec![
        Op::Ready(Bytecode::Add).at(span(0, 1)),
        Op::Ready(Bytecode::PushInt(3)).at(span(2, 3)),
        Op::DefLocal(unread).at(span(5, 9)),
    ];

    let result = eliminate_dead_code(function(code, locals), &BOTH).code;

    assert_eq!(result.len(), 1, "load and store are both gone: {result:?}");
    assert!(is_ready_at(&result[0], Bytecode::Add, span(0, 1)));
}

/// A store is live when a later instruction reads its local, and an exported
/// local is never dead.
#[test]
fn read_and_exported_stores_are_kept() {
    let mut locals = Locals::new();
    let read = define(&mut locals, "read", false);
    let exported = define(&mut locals, "exported", true);
    let code = vec![
        Op::Ready(Bytecode::Add).at(span(0, 1)),
        Op::DefLocal(read).at(span(5, 9)),
        Op::DefLocal(exported).at(span(12, 16)),
        Op::LoadLocal(read).at(span(20, 24)),
    ];

    let result = eliminate_dead_code(function(code, locals), &DEAD_STORE_ONLY).code;

    assert_eq!(result.len(), 4, "nothing is removed: {result:?}");
    assert!(
        is_def_at(&result[1], read, span(5, 9)),
        "the read local's store is kept with its span: {result:?}"
    );
    assert!(
        is_def_at(&result[2], exported, span(12, 16)),
        "the exported local's store is kept with its span: {result:?}"
    );
}
