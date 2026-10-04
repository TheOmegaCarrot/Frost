//! Tests for the lexical-scope resolver: id allocation, shadowing across nested
//! scopes, same-scope duplicate rejection, scope unwinding, the constant
//! metadata a binding carries, and the slot plan assembly derives from the code.

use crate::lower::locals::{LocalInfo, LocalKind, Locals, SlotPlan};
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
fn rewinding_hides_bindings_made_since_the_checkpoint() {
    let mut locals = Locals::new();
    let outer = bind(&mut locals, "x", span(0)).unwrap();
    locals.enter();
    let checkpoint = locals.checkpoint();
    bind(&mut locals, "x", span(1)).unwrap();
    bind(&mut locals, "y", span(2)).unwrap();
    locals.rewind(checkpoint);
    assert_eq!(
        locals.resolve("x"),
        Some(outer),
        "the shadowed `x` shows again"
    );
    assert_eq!(locals.resolve("y"), None);
    // Hidden, the names may be defined again without colliding.
    assert!(bind(&mut locals, "y", span(3)).is_ok());
}

#[test]
fn a_revived_local_resolves_again_under_its_old_id() {
    let mut locals = Locals::new();
    locals.enter();
    let checkpoint = locals.checkpoint();
    let x = bind(&mut locals, "x", span(0)).unwrap();
    locals.rewind(checkpoint);
    locals.revive(x);
    assert_eq!(locals.resolve("x"), Some(x));
    // It is in view in the current scope, so a redefinition is a duplicate.
    assert_eq!(bind(&mut locals, "x", span(1)), Err(span(0)));
    // And it leaves with that scope.
    locals.exit();
    assert_eq!(locals.resolve("x"), None);
}

#[test]
#[should_panic(expected = "within its own scope")]
fn rewinding_past_the_current_scope_panics() {
    let mut locals = Locals::new();
    let checkpoint = locals.checkpoint();
    bind(&mut locals, "x", span(0)).unwrap();
    locals.enter();
    locals.rewind(checkpoint);
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
    assert!(
        locals.constant(inner).is_none(),
        "shadow carries no constant"
    );
    locals.exit();

    assert!(
        matches!(locals.constant(outer), Some(Value::Int(1))),
        "the outer constant is undisturbed after the shadow exits"
    );
}

#[test]
fn at_top_scope_reflects_the_open_scope_depth() {
    let mut locals = Locals::new();
    assert!(locals.at_top_scope(), "no nested scope open yet");
    locals.enter();
    assert!(!locals.at_top_scope(), "inside a nested scope");
    locals.enter();
    assert!(!locals.at_top_scope(), "still nested, two levels deep");
    locals.exit();
    assert!(!locals.at_top_scope(), "one level is still open");
    locals.exit();
    assert!(locals.at_top_scope(), "back to the top after both exits");
}

#[test]
#[should_panic(expected = "exit without a matching enter")]
fn exit_without_a_matching_enter_panics() {
    let mut locals = Locals::new();
    locals.exit();
}

#[test]
fn two_inherited_locals_of_the_same_name_do_not_collide() {
    // Only a binding is checked for duplicates; two inherited locals (captures
    // here) sharing a name is not itself flagged, since the caller is trusted to
    // supply distinct capture names.
    let mut locals = Locals::new();
    let first = locals
        .define(LocalInfo {
            name: "x".to_string(),
            span: span(0),
            exported: false,
            constant: None,
            kind: LocalKind::Capture,
        })
        .unwrap();
    let second = locals
        .define(LocalInfo {
            name: "x".to_string(),
            span: span(1),
            exported: false,
            constant: None,
            kind: LocalKind::Capture,
        })
        .unwrap();
    assert_ne!(first, second);
    assert_eq!(
        locals.resolve("x"),
        Some(second),
        "the more recently defined inherited local wins resolution"
    );
}

#[test]
fn a_binding_may_shadow_a_same_scope_hoisted_capture() {
    // A hoisted capture is inherited, just like a plain capture, so it too is
    // exempt from the duplicate check when a real binding shadows it.
    let mut locals = Locals::new();
    let hoisted = locals
        .define(LocalInfo {
            name: "x".to_string(),
            span: span(0),
            exported: false,
            constant: Some(Value::Int(1)),
            kind: LocalKind::Hoisted,
        })
        .unwrap();
    let bound = bind(&mut locals, "x", span(1)).unwrap();
    assert_ne!(hoisted, bound);
    assert_eq!(
        locals.resolve("x"),
        Some(bound),
        "the binding wins once defined"
    );
    assert_eq!(
        bind(&mut locals, "x", span(2)),
        Err(span(1)),
        "a second real binding is still a duplicate"
    );
}

#[test]
fn with_captures_seeds_each_given_value() {
    let locals = Locals::with_captures(
        [
            ("plain".to_string(), None),
            ("known".to_string(), Some(Value::Int(7))),
        ],
        [("hoisted".to_string(), Value::Int(99))],
    );
    let plain = locals
        .resolve("plain")
        .expect("a capture is live from the start");
    let known = locals
        .resolve("known")
        .expect("a capture with a known value is live from the start");
    let hoisted = locals
        .resolve("hoisted")
        .expect("a hoisted capture is live from the start");
    assert!(
        locals.constant(plain).is_none(),
        "a plain capture carries no compile-time value"
    );
    assert!(
        matches!(locals.constant(known), Some(Value::Int(7))),
        "a capture carries the value it was given"
    );
    assert!(matches!(locals.constant(hoisted), Some(Value::Int(99))));
}

#[test]
fn a_capture_with_a_known_value_is_still_seated() {
    // Only hoisting builds a value in; a known value on a capture is for folds.
    let locals = Locals::with_captures([("known".to_string(), Some(Value::Int(7)))], []);
    let known = locals.resolve("known").unwrap();
    let plan = locals.plan_slots(&[]);
    assert_eq!(plan.num_captures(), 1);
    assert_eq!(plan.slot_of(known), 0);
}

#[test]
fn a_binding_may_shadow_a_same_scope_capture() {
    let mut locals = Locals::new();
    let captured = locals
        .define(LocalInfo {
            name: "x".to_string(),
            span: span(0),
            exported: false,
            constant: None,
            kind: LocalKind::Capture,
        })
        .unwrap();
    // `def x = x`: the rhs reads the capture, then a fresh `x` is bound. The
    // binding shadows the capture rather than colliding with it.
    let bound = bind(&mut locals, "x", span(1)).unwrap();
    assert_ne!(captured, bound);
    assert_eq!(
        locals.resolve("x"),
        Some(bound),
        "the binding wins once defined"
    );
    // A second real binding of the same name is still a duplicate.
    assert_eq!(bind(&mut locals, "x", span(2)), Err(span(1)));
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
fn plan_slots_orders_body_locals_by_first_definition_not_creation() {
    // `a` and `b` are created in that order, but the code defines `b` first:
    // the slot order follows the code, not `infos`.
    let mut locals = Locals::new();
    let captured = capture(&mut locals, "captured");
    let a = bind(&mut locals, "a", span(1)).unwrap();
    let b = bind(&mut locals, "b", span(2)).unwrap();

    let plan = locals.plan_slots(&[def(b), def(a)]);
    assert_eq!(plan.num_captures(), 1);
    assert_eq!(plan.slot_of(captured), 0, "the capture always leads");
    assert_eq!(plan.slot_of(b), 1, "b is defined first in the code");
    assert_eq!(plan.slot_of(a), 2);
}

#[test]
fn a_hoisted_capture_is_not_counted_or_seated_as_a_capture() {
    // A hoisted constant is built into the function body; it never occupies a
    // frame slot, unlike an ordinary (seated) capture.
    let locals = Locals::with_captures([], [("h".to_string(), Value::Int(1))]);
    let plan = locals.plan_slots(&[]);
    assert_eq!(
        plan.num_captures(),
        0,
        "a hoisted constant does not count as a seated capture"
    );
    assert!(plan.into_name_table().is_empty());
}

#[test]
#[should_panic(expected = "no slot")]
fn a_hoisted_capture_has_no_slot_to_ask_for() {
    let locals = Locals::with_captures([], [("h".to_string(), Value::Int(1))]);
    let hoisted = locals.resolve("h").unwrap();
    locals.plan_slots(&[]).slot_of(hoisted);
}

#[test]
fn empty_slot_plan_has_no_captures_or_names() {
    let plan = SlotPlan::empty();
    assert_eq!(plan.num_captures(), 0);
    assert!(plan.into_name_table().is_empty());
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
    assert_eq!(
        plan.slot_of(c),
        1,
        "c fills the slot the gap would have held"
    );
    assert_eq!(plan.into_name_table().len(), 2);
    let _ = dropped;
}

/// Define a capture, the way a function's enclosing-scope names are seeded.
fn capture(locals: &mut Locals, name: &str) -> LocalId {
    locals
        .define(LocalInfo {
            name: name.to_string(),
            span: span(0),
            exported: false,
            constant: None,
            kind: LocalKind::Capture,
        })
        .unwrap()
}

/// Define a binding whose value is compile-time known.
fn bind_known(locals: &mut Locals, name: &str, value: Value) -> LocalId {
    locals
        .define(LocalInfo {
            name: name.to_string(),
            span: span(0),
            exported: false,
            constant: Some(value),
            kind: LocalKind::Binding,
        })
        .unwrap()
}

fn load(id: LocalId) -> Ir {
    Ir::LoadLocal(id)
}

#[test]
fn a_fragment_plan_seats_only_what_the_fragment_uses() {
    // A capture of the function the fragment does not read is no capture of the
    // fragment: only the locals the fragment defines get slots, from 0.
    let mut locals = Locals::new();
    capture(&mut locals, "captured");
    let a = bind(&mut locals, "a", span(1)).unwrap();

    let (plan, captures) = locals.plan_fragment_slots(&[def(a)]);
    assert_eq!(plan.num_captures(), 0);
    assert!(captures.is_empty(), "nothing to seat");
    assert_eq!(plan.slot_of(a), 0, "the fragment's own local leads");
    let table = plan.into_name_table();
    let names: Vec<&str> = table.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["a"], "no capture in the name table");
}

#[test]
#[should_panic(expected = "no slot")]
fn a_fragment_plan_gives_an_unread_capture_no_slot() {
    let mut locals = Locals::new();
    let captured = capture(&mut locals, "captured");
    locals.plan_fragment_slots(&[]).0.slot_of(captured);
}

#[test]
fn a_fragment_plan_orders_locals_by_first_definition() {
    let mut locals = Locals::new();
    let a = bind(&mut locals, "a", span(0)).unwrap();
    let b = bind(&mut locals, "b", span(1)).unwrap();
    let unused = bind(&mut locals, "unused", span(2)).unwrap();

    // `b` is defined first, and twice; `unused` is never defined.
    let (plan, _) = locals.plan_fragment_slots(&[def(b), def(a), def(b)]);
    assert_eq!(plan.slot_of(b), 0);
    assert_eq!(plan.slot_of(a), 1);
    assert_eq!(plan.into_name_table().len(), 2, "one slot each, no hole");
    let _ = unused;
}

#[test]
fn a_fragment_plan_seats_each_known_outer_local_it_reads() {
    // An outer local the fragment reads is a capture, seated with its known
    // value; the captures lead, in first-read order, and are seated once each.
    let mut locals = Locals::new();
    let f = bind_known(&mut locals, "f", Value::Int(1));
    let g = bind_known(&mut locals, "g", Value::Int(2));
    let unread = bind_known(&mut locals, "unread", Value::Int(3));
    let own = bind(&mut locals, "own", span(1)).unwrap();

    let (plan, captures) =
        locals.plan_fragment_slots(&[load(g), def(own), load(f), load(g), load(own)]);
    assert_eq!(plan.num_captures(), 2, "f and g, not the unread local");
    assert_eq!(plan.slot_of(g), 0, "g is read first");
    assert_eq!(plan.slot_of(f), 1);
    assert_eq!(plan.slot_of(own), 2, "a defined local follows the captures");
    assert_eq!(
        captures.into_iter().collect::<Vec<_>>(),
        vec![
            ("f".to_string(), Value::Int(1)),
            ("g".to_string(), Value::Int(2)),
        ],
        "each capture is seated with its known value, by name"
    );
    let names: Vec<String> = plan.into_name_table().into_iter().map(|e| e.name).collect();
    assert_eq!(names, vec!["g", "f", "own"]);
    let _ = unread;
}

#[test]
fn a_local_the_fragment_defines_is_not_a_capture_even_if_read_first() {
    // Reading a local the fragment also defines reads the fragment's own slot.
    let mut locals = Locals::new();
    let known = bind_known(&mut locals, "known", Value::Int(1));

    let (plan, captures) = locals.plan_fragment_slots(&[load(known), def(known)]);
    assert_eq!(plan.num_captures(), 0);
    assert!(captures.is_empty());
    assert_eq!(plan.slot_of(known), 0);
}

#[test]
#[should_panic(expected = "reads only known outer locals")]
fn a_fragment_reading_an_unknown_outer_local_panics() {
    // Such a fragment is never foldable, so planning one is a compiler bug.
    let mut locals = Locals::new();
    let unknown = bind(&mut locals, "unknown", span(0)).unwrap();
    locals.plan_fragment_slots(&[load(unknown)]);
}

#[test]
#[should_panic(expected = "no slot")]
fn slot_of_a_local_with_no_slot_panics() {
    let mut locals = Locals::new();
    let orphan = bind(&mut locals, "orphan", span(0)).unwrap();
    // Never defined by the code, so it has no slot; asking for one is a bug.
    locals.plan_slots(&[]).slot_of(orphan);
}
