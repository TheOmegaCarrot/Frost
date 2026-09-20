//! Tests for the lexical-scope resolver: id allocation, shadowing across nested
//! scopes, same-scope duplicate rejection, scope unwinding, the constant
//! metadata a binding carries, and the slot plan assembly derives from the code.

use crate::lower::locals::{LocalInfo, LocalKind, Locals};
use crate::lower::{Ir, LocalId};

use frost_parse::ast::SourceSpan;
use frost_runtime::Value;

/// A distinct span per test binding, so a duplicate error's returned span is
/// checkable.
fn span(start: usize) -> SourceSpan {
    SourceSpan {
        start,
        end: start + 1,
    }
}

/// Define a plain (non-exported, non-constant) binding.
fn bind(locals: &mut Locals, name: &str, span: SourceSpan) -> Result<LocalId, SourceSpan> {
    locals.define(LocalInfo {
        name: name.to_string(),
        span,
        exported: false,
        constant: None,
        kind: LocalKind::Binding,
    })
}

#[test]
fn define_allocates_distinct_ids() {
    let mut locals = Locals::new();
    let a = bind(&mut locals, "a", span(0)).unwrap();
    let b = bind(&mut locals, "b", span(1)).unwrap();
    let c = bind(&mut locals, "c", span(2)).unwrap();
    assert_ne!(a, b);
    assert_ne!(b, c);
    assert_ne!(a, c);
}

#[test]
fn resolve_finds_a_defined_name() {
    let mut locals = Locals::new();
    let a = bind(&mut locals, "a", span(0)).unwrap();
    let b = bind(&mut locals, "b", span(1)).unwrap();
    assert_eq!(locals.resolve("b"), Some(b));
    assert_eq!(locals.resolve("a"), Some(a));
}

#[test]
fn resolve_unknown_name_is_none() {
    let locals = Locals::new();
    assert_eq!(locals.resolve("missing"), None);
}

#[test]
fn same_scope_redefinition_reports_the_original_span() {
    let mut locals = Locals::new();
    bind(&mut locals, "foo", span(10)).unwrap();
    // The second `foo` collides; the error carries where the first was bound.
    assert_eq!(bind(&mut locals, "foo", span(20)), Err(span(10)));
}

#[test]
fn nested_scope_shadows_then_restores() {
    let mut locals = Locals::new();
    let outer = bind(&mut locals, "foo", span(0)).unwrap();

    locals.enter();
    // Shadowing an outer binding is allowed and takes a fresh id.
    let inner = bind(&mut locals, "foo", span(1)).unwrap();
    assert_ne!(inner, outer, "the shadow gets its own id");
    assert_eq!(locals.resolve("foo"), Some(inner), "innermost wins");
    locals.exit();

    assert_eq!(locals.resolve("foo"), Some(outer), "outer binding restored");
}

#[test]
fn a_name_freed_by_scope_exit_can_be_defined_again() {
    let mut locals = Locals::new();
    locals.enter();
    bind(&mut locals, "tmp", span(0)).unwrap();
    locals.exit();
    // `tmp` is out of scope now, so a fresh top-level `tmp` is not a duplicate.
    assert!(locals.resolve("tmp").is_none());
    assert!(bind(&mut locals, "tmp", span(1)).is_ok());
}

#[test]
fn duplicate_within_a_nested_scope_is_still_rejected() {
    let mut locals = Locals::new();
    locals.enter();
    bind(&mut locals, "x", span(0)).unwrap();
    assert_eq!(bind(&mut locals, "x", span(1)), Err(span(0)));
}

#[test]
fn a_constant_binding_reports_its_value() {
    let mut locals = Locals::new();
    let plain = bind(&mut locals, "plain", span(0)).unwrap();
    let known = locals
        .define(LocalInfo {
            name: "known".to_string(),
            span: span(1),
            exported: false,
            constant: Some(Value::Int(42)),
            kind: LocalKind::Binding,
        })
        .unwrap();
    assert!(locals.constant(plain).is_none());
    assert!(matches!(locals.constant(known), Some(Value::Int(42))));
}

#[test]
fn a_shadow_constant_is_independent_of_the_binding_it_shadows() {
    let mut locals = Locals::new();
    let outer = locals
        .define(LocalInfo {
            name: "x".to_string(),
            span: span(0),
            exported: false,
            constant: Some(Value::Int(1)),
            kind: LocalKind::Binding,
        })
        .unwrap();

    locals.enter();
    // The shadow is a non-constant binding; it must not inherit the outer's value.
    let inner = bind(&mut locals, "x", span(1)).unwrap();
    assert!(locals.constant(inner).is_none(), "shadow carries no constant");
    locals.exit();

    assert!(
        matches!(locals.constant(outer), Some(Value::Int(1))),
        "the outer constant is undisturbed after the shadow exits"
    );
}

// -- Slot plan --

/// A `DefLocal` of `id`, the way body code introduces a binding.
fn def(id: LocalId) -> Ir {
    Ir::DefLocal(id)
}

#[test]
fn plan_lists_captures_first_then_defined_locals() {
    let mut locals = Locals::new();
    let captured = locals
        .define(LocalInfo {
            name: "captured".to_string(),
            span: span(0),
            exported: false,
            constant: None,
            kind: LocalKind::Capture,
        })
        .unwrap();
    let a = bind(&mut locals, "a", span(1)).unwrap();
    let b = bind(&mut locals, "b", span(2)).unwrap();

    let plan = locals.plan_slots(&[def(a), def(b)]);

    // The capture leads, then the bindings in definition order.
    assert_eq!(plan.num_captures(), 1);
    assert_eq!(plan.slot_of(captured), 0);
    assert_eq!(plan.slot_of(a), 1);
    assert_eq!(plan.slot_of(b), 2);
    let table = plan.into_name_table();
    let names: Vec<&str> = table.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["captured", "a", "b"]);
}

#[test]
fn a_capture_gets_a_slot_even_when_the_code_never_references_it() {
    let mut locals = Locals::new();
    let captured = locals
        .define(LocalInfo {
            name: "captured".to_string(),
            span: span(0),
            exported: false,
            constant: None,
            kind: LocalKind::Capture,
        })
        .unwrap();

    // The VM seats captures regardless of use, so a capture always holds slot 0.
    let plan = locals.plan_slots(&[]);
    assert_eq!(plan.num_captures(), 1);
    assert_eq!(plan.slot_of(captured), 0);
}

#[test]
fn a_binding_the_code_never_defines_is_left_out_without_a_hole() {
    let mut locals = Locals::new();
    let a = bind(&mut locals, "a", span(0)).unwrap();
    let dropped = bind(&mut locals, "dropped", span(1)).unwrap();
    let c = bind(&mut locals, "c", span(2)).unwrap();

    // `dropped` is never defined by the code, as if an optimization removed it.
    // The survivors pack densely: no slot is wasted on the gap.
    let plan = locals.plan_slots(&[def(a), def(c)]);
    assert_eq!(plan.slot_of(a), 0);
    assert_eq!(plan.slot_of(c), 1, "c fills the slot the gap would have held");
    assert_eq!(plan.into_name_table().len(), 2);
    let _ = dropped;
}

#[test]
#[should_panic(expected = "no slot")]
fn slot_of_a_local_with_no_slot_panics() {
    let mut locals = Locals::new();
    let orphan = bind(&mut locals, "orphan", span(0)).unwrap();
    // Never defined by the code, so it has no slot; asking for one is a bug.
    locals.plan_slots(&[]).slot_of(orphan);
}
