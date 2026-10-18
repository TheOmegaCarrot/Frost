//! `ModuleId`, a resolver-assigned module identity, through its public API.

use std::collections::{BTreeSet, HashSet};

use frostlang::ModuleId;

#[test]
fn it_is_the_text_it_was_made_from() {
    let id = ModuleId::new("app/config");
    assert_eq!(id.as_str(), "app/config");
    assert_eq!(AsRef::<str>::as_ref(&id), "app/config", "AsRef<str>");
    assert_eq!(id.to_string(), "app/config", "Display is the text");
}

#[test]
fn ids_are_equal_exactly_when_their_text_is() {
    assert_eq!(ModuleId::new("a"), ModuleId::new(String::from("a")));
    assert_ne!(ModuleId::new("a"), ModuleId::new("A"));
}

#[test]
fn it_is_found_by_its_text_in_a_set() {
    // `Borrow<str>` with a matching hash: a set of them answers for a `&str`.
    let set: HashSet<ModuleId> = ["a", "b"].into_iter().map(ModuleId::new).collect();
    assert!(set.contains("a"));
    assert!(!set.contains("c"));
}

#[test]
fn it_orders_as_its_text_does() {
    let ids: BTreeSet<ModuleId> = ["b", "", "ab", "a"]
        .into_iter()
        .map(ModuleId::new)
        .collect();
    let order: Vec<&str> = ids.iter().map(ModuleId::as_str).collect();
    assert_eq!(order, ["", "a", "ab", "b"]);
    assert!(ids.contains("ab"), "a sorted set answers for a `&str` too");
}
