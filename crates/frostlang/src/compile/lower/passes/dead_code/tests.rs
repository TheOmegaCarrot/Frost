//! Tests for the dead-code pass: which instructions each `Pop` cancels, and
//! which span each surviving `Pop` keeps.
//!
//! Every IR instruction carries a distinct span, so a test fails if the pass
//! cancels or re-spans the wrong instruction.

use crate::Arity;
use crate::bytecode::{Bytecode, Span};
use crate::compile::OptimizationOptions;
use crate::compile::lower::locals::{LocalInfo, LocalKind, Locals};
use crate::compile::lower::passes::dead_code::eliminate_dead_code;
use crate::compile::lower::{Ir, Label, LocalId, LoweredFunction, Op};
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

/// A function capturing `count` values, to create a closure over.
fn function_capturing(count: usize) -> LoweredFunction {
    let mut locals = Locals::new();
    for i in 0..count {
        locals
            .define(LocalInfo {
                name: format!("capture{i}"),
                span: SourceSpan::default(),
                exported: false,
                constant: None,
                kind: LocalKind::Capture,
            })
            .expect("the capture names are distinct");
    }
    function(vec![Op::Ready(Bytecode::PushNull).sourceless()], locals)
}

/// A discarded closure's `Pop` becomes a `Pop` of each capture, each carrying
/// the discarding `Pop`'s span. A capture that is a load cancels its `Pop`; one
/// that is not keeps it.
#[test]
fn a_discarded_closure_pops_its_captures_instead() {
    let code = vec![
        Op::Ready(Bytecode::Add).at(span(0, 1)),
        Op::Ready(Bytecode::PushInt(1)).at(span(1, 2)),
        Op::closure(function_capturing(2)).at(span(2, 3)),
        Op::Ready(Bytecode::Pop).at(span(10, 11)),
    ];

    let result = eliminate_dead_code(function(code, Locals::new()), &DISCARD_ONLY).code;

    assert_eq!(
        result.len(),
        2,
        "the closure, the load of its second capture, and that capture's Pop are \
         gone: {result:?}"
    );
    assert!(
        is_ready_at(&result[0], Bytecode::Add, span(0, 1)),
        "the first capture, not a load, is still computed: {result:?}"
    );
    assert!(
        is_ready_at(&result[1], Bytecode::Pop, span(10, 11)),
        "the first capture is popped at the discarding Pop's span: {result:?}"
    );
}

/// The `Pop`s a discarded closure becomes come before any already pending, as
/// its captures are the values on top of the stack.
#[test]
fn a_discarded_closures_pops_come_before_those_already_pending() {
    let code = vec![
        Op::Ready(Bytecode::Add).at(span(0, 1)),
        Op::Ready(Bytecode::PushInt(1)).at(span(1, 2)),
        Op::closure(function_capturing(2)).at(span(2, 3)),
        Op::Ready(Bytecode::Pop).at(span(10, 11)),
        Op::Ready(Bytecode::Pop).at(span(20, 21)),
    ];

    let result = eliminate_dead_code(function(code, Locals::new()), &DISCARD_ONLY).code;

    assert_eq!(
        result.len(),
        3,
        "the closure, its second capture's load, and that load's Pop are gone: \
         {result:?}"
    );
    assert!(
        is_ready_at(&result[0], Bytecode::Add, span(0, 1)),
        "the first capture, not a load, is still computed: {result:?}"
    );
    assert!(
        is_ready_at(&result[1], Bytecode::Pop, span(10, 11)),
        "the first capture's Pop takes the closure's discarding Pop's span: {result:?}"
    );
    assert!(
        is_ready_at(&result[2], Bytecode::Pop, span(20, 21)),
        "the Pop pending before the closure keeps its own span, after: {result:?}"
    );
}

/// A label between a closure's capture loads stops the cancelling: the load
/// after the label still cancels its `Pop`, but the one before it is kept, and
/// its `Pop` lands after the label.
#[test]
fn a_label_stops_a_discarded_closures_pops() {
    let code = vec![
        Op::Ready(Bytecode::PushInt(0)).at(span(0, 1)),
        Op::Label(Label(0)).at(span(1, 1)),
        Op::Ready(Bytecode::PushInt(1)).at(span(1, 2)),
        Op::closure(function_capturing(2)).at(span(2, 3)),
        Op::Ready(Bytecode::Pop).at(span(10, 11)),
    ];

    let result = eliminate_dead_code(function(code, Locals::new()), &DISCARD_ONLY).code;

    assert_eq!(
        result.len(),
        3,
        "the closure, its second capture's load, and that load's Pop are gone: \
         {result:?}"
    );
    assert!(
        is_ready_at(&result[0], Bytecode::PushInt(0), span(0, 1)),
        "the load before the label is kept: {result:?}"
    );
    assert!(
        matches!(result[1].op, Op::Label(Label(0))),
        "the label stays where it was: {result:?}"
    );
    assert!(
        is_ready_at(&result[2], Bytecode::Pop, span(10, 11)),
        "the first capture's Pop lands after the label: {result:?}"
    );
}

/// A discarded closure whose captures are all loads vanishes whole.
#[test]
fn a_discarded_closure_over_loads_vanishes() {
    let code = vec![
        Op::Ready(Bytecode::Add).at(span(0, 1)),
        Op::Ready(Bytecode::PushInt(1)).at(span(1, 2)),
        Op::Ready(Bytecode::PushInt(2)).at(span(2, 3)),
        Op::closure(function_capturing(2)).at(span(3, 4)),
        Op::Ready(Bytecode::Pop).at(span(10, 11)),
    ];

    let result = eliminate_dead_code(function(code, Locals::new()), &DISCARD_ONLY).code;

    assert_eq!(result.len(), 1, "only the unrelated op remains: {result:?}");
    assert!(is_ready_at(&result[0], Bytecode::Add, span(0, 1)));
}

/// With `discard_eliminate` off, a discarded closure is still created.
#[test]
fn a_discarded_closure_stays_without_discard_elimination() {
    let code = vec![
        Op::Ready(Bytecode::PushInt(1)).at(span(1, 2)),
        Op::closure(function_capturing(1)).at(span(2, 3)),
        Op::Ready(Bytecode::Pop).at(span(10, 11)),
    ];

    let result = eliminate_dead_code(function(code, Locals::new()), &DEAD_STORE_ONLY).code;

    assert_eq!(result.len(), 3, "nothing is removed: {result:?}");
    assert!(
        matches!(result[1].op, Op::Closure { .. }),
        "the closure is still created: {result:?}"
    );
}
