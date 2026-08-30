//! Tests for the lexical-scope resolver: slot allocation, shadowing across
//! nested scopes, same-scope duplicate rejection, and scope unwinding.

use super::Locals;
use frost_parse::ast::SourceSpan;

/// A distinct span per test binding, so a duplicate error's returned span is
/// checkable.
fn span(start: usize) -> SourceSpan {
    SourceSpan {
        start,
        end: start + 1,
    }
}

#[test]
fn define_allocates_ascending_slots() {
    let mut locals = Locals::new();
    assert_eq!(locals.define("a".into(), span(0), false), Ok(0));
    assert_eq!(locals.define("b".into(), span(1), false), Ok(1));
    assert_eq!(locals.define("c".into(), span(2), false), Ok(2));
}

#[test]
fn resolve_finds_a_defined_name() {
    let mut locals = Locals::new();
    locals.define("a".into(), span(0), false).unwrap();
    let slot = locals.define("b".into(), span(1), false).unwrap();
    assert_eq!(locals.resolve("b"), Some(slot));
    assert_eq!(locals.resolve("a"), Some(0));
}

#[test]
fn resolve_unknown_name_is_none() {
    let locals = Locals::new();
    assert_eq!(locals.resolve("missing"), None);
}

#[test]
fn same_scope_redefinition_reports_the_original_span() {
    let mut locals = Locals::new();
    locals.define("foo".into(), span(10), false).unwrap();
    // The second `foo` collides; the error carries where the first was bound.
    assert_eq!(locals.define("foo".into(), span(20), false), Err(span(10)));
}

#[test]
fn nested_scope_shadows_then_restores() {
    let mut locals = Locals::new();
    let outer = locals.define("foo".into(), span(0), false).unwrap();

    locals.enter();
    // Shadowing an outer binding is allowed and takes a fresh slot.
    let inner = locals.define("foo".into(), span(1), false).unwrap();
    assert_ne!(inner, outer, "the shadow gets its own slot");
    assert_eq!(locals.resolve("foo"), Some(inner), "innermost wins");
    locals.exit();

    assert_eq!(locals.resolve("foo"), Some(outer), "outer binding restored");
}

#[test]
fn a_name_freed_by_scope_exit_can_be_defined_again() {
    let mut locals = Locals::new();
    locals.enter();
    locals.define("tmp".into(), span(0), false).unwrap();
    locals.exit();
    // `tmp` is out of scope now, so a fresh top-level `tmp` is not a duplicate.
    assert!(locals.resolve("tmp").is_none());
    assert!(locals.define("tmp".into(), span(1), false).is_ok());
}

#[test]
fn duplicate_within_a_nested_scope_is_still_rejected() {
    let mut locals = Locals::new();
    locals.enter();
    locals.define("x".into(), span(0), false).unwrap();
    assert_eq!(locals.define("x".into(), span(1), false), Err(span(0)));
}

#[test]
fn exited_scopes_keep_their_slots_in_the_table() {
    let mut locals = Locals::new();
    locals.define("outer".into(), span(0), false).unwrap();
    locals.enter();
    locals.define("inner".into(), span(1), true).unwrap();
    locals.exit();
    locals.define("after".into(), span(2), false).unwrap();

    // Slots are never reclaimed: every binding ever defined is in the table,
    // in allocation order, with its export flag preserved.
    let table = locals.into_name_table();
    let names: Vec<&str> = table.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["outer", "inner", "after"]);
    assert!(table[1].exported, "the inner binding was exported");
}
