//! Tests for the [`FrostOpaque`] surface: the trait methods through a
//! type-erased handle, `dyn`-level downcasting, and `try_extract`'s
//! steal-when-unique / give-back-when-not contract.
//!
//! Value-level accessors (`as_opaque`, `downcast_opaque`, `try_into_opaque`)
//! are covered in `value_accessors.rs`; identity equality in
//! `value_equality.rs`; stringify properties in `value_to_string.rs`.

use std::borrow::Cow;
use std::sync::Arc;

use frost_runtime::{FrostOpaque, Value};

/// The primary payload type: carries data, has a string approximation.
#[derive(Debug, PartialEq)]
struct Widget {
    id: u32,
}

impl FrostOpaque for Widget {
    fn type_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Widget")
    }

    fn try_to_string(&self) -> Option<String> {
        Some(format!("widget #{}", self.id))
    }
}

/// A second payload type, so wrong-type probes have a real target.
#[derive(Debug)]
struct Gadget;

impl FrostOpaque for Gadget {
    fn type_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Gadget")
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

fn widget(id: u32) -> Arc<dyn FrostOpaque> {
    Arc::new(Widget { id })
}

// ============================================================
// Trait methods through the erased handle
// ============================================================

#[test]
fn trait_methods_dispatch_through_the_handle() {
    let h = widget(7);
    assert_eq!(h.type_name(), "Widget");
    assert_eq!(h.try_to_string(), Some("widget #7".to_string()));
}

#[test]
fn try_to_string_may_decline() {
    let h: Arc<dyn FrostOpaque> = Arc::new(Gadget);
    assert_eq!(h.try_to_string(), None);
}

// ============================================================
// downcast_ref / downcast_mut
// ============================================================

#[test]
fn downcast_ref_recovers_the_concrete_type() {
    let h = widget(5);
    assert_eq!(h.downcast_ref::<Widget>(), Some(&Widget { id: 5 }));
}

#[test]
fn downcast_ref_to_the_wrong_type_is_none() {
    let h = widget(5);
    assert!(h.downcast_ref::<Gadget>().is_none());
}

#[test]
fn downcast_mut_mutates_through_a_unique_handle() {
    let mut h = widget(1);
    let exclusive = Arc::get_mut(&mut h).expect("handle is unique");
    exclusive.downcast_mut::<Widget>().expect("is a Widget").id = 9;
    assert_eq!(h.downcast_ref::<Widget>(), Some(&Widget { id: 9 }));
}

// ============================================================
// try_extract
// ============================================================

#[test]
fn try_extract_steals_from_a_unique_handle() {
    let h = widget(5);
    assert_eq!(h.try_extract::<Widget>().unwrap(), Widget { id: 5 });
}

#[test]
fn try_extract_from_a_shared_handle_gives_the_handle_back() {
    let h = widget(5);
    let keep = h.clone();
    let back = h
        .try_extract::<Widget>()
        .expect_err("a shared handle must not be stolen");
    assert!(Arc::ptr_eq(&back, &keep));
    // The returned handle is fully usable.
    assert_eq!(back.downcast_ref::<Widget>(), Some(&Widget { id: 5 }));
}

#[test]
fn try_extract_of_the_wrong_type_gives_the_handle_back() {
    let h = widget(5);
    let back = h
        .try_extract::<Gadget>()
        .expect_err("wrong payload type must not extract");
    assert_eq!(back.downcast_ref::<Widget>(), Some(&Widget { id: 5 }));
}

#[test]
fn try_extract_succeeds_once_the_extra_handle_drops() {
    // The give-back is a retriable outcome, not a dead end: after the other
    // handle drops, the same extraction succeeds.
    let h = widget(3);
    let keep = h.clone();
    let back = h.try_extract::<Widget>().expect_err("shared for now");
    drop(keep);
    assert_eq!(back.try_extract::<Widget>().unwrap(), Widget { id: 3 });
}

// ============================================================
// Value round trip
// ============================================================

#[test]
fn value_round_trip_ends_in_extraction() {
    let v = Value::opaque(Widget { id: 11 });
    assert!(v.is_opaque());
    let handle = v.try_into_opaque().expect("an Opaque extracts");
    assert_eq!(handle.try_extract::<Widget>().unwrap(), Widget { id: 11 });
}
