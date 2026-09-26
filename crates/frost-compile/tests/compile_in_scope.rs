//! `compile_in_scope` and `implicit_export`, the two pieces an embedder (a REPL)
//! needs: a top-level compiled against an enclosing scope captures the names it
//! uses from that scope, and implicit export lets the caller harvest every
//! top-level binding afterward.

mod common;

use common::{Script, compile_errors};
use frost_runtime::Value;

#[test]
fn an_outer_binding_is_captured() {
    // `x` is free and named in the outer scope, so it becomes a capture supplied
    // at close time.
    let tail = Script::new("x").capture("x", Value::Int(5)).run();
    assert_eq!(tail, Value::Int(5));
}

#[test]
fn a_captured_name_shadows_a_global() {
    // `len` is a global, but here it is also an outer-scope binding, so the
    // capture wins: the lookup yields the captured value, not the builtin.
    let tail = Script::new("len").capture("len", Value::Int(42)).run();
    assert_eq!(
        tail,
        Value::Int(42),
        "the captured `len` shadows the global function"
    );
}

#[test]
fn an_unused_outer_name_is_not_captured() {
    // The script never mentions `x`, so it is not captured: closing with no
    // value for `x` still succeeds (a spurious capture would make close fail).
    let tail = Script::new("42").in_scope("x").run();
    assert_eq!(tail, Value::Int(42));
}

#[test]
fn implicit_export_exports_every_top_level_binding() {
    // `def y = x` is not written `export`, but implicit export makes it harvestable.
    let finished = Script::new("def y = x; y")
        .capture("x", Value::Int(5))
        .implicit_export()
        .finish();
    assert_eq!(finished.tail, Value::Int(5));
    assert_eq!(
        finished.exports.get("y"),
        Some(&Value::Int(5)),
        "the top-level binding is exported"
    );
}

#[test]
fn a_top_level_binding_may_shadow_a_capture_of_the_same_name() {
    // `def x = x`: the rhs captures the enclosing `x`, and the def binds a new
    // `x` from it. The capture must not read as a duplicate binding.
    let finished = Script::new("def x = x; x")
        .capture("x", Value::Int(7))
        .implicit_export()
        .finish();
    assert_eq!(finished.tail, Value::Int(7));
    assert_eq!(
        finished.exports.get("x"),
        Some(&Value::Int(7)),
        "the redefined `x` is what gets exported"
    );
}

#[test]
fn without_implicit_export_a_plain_def_is_not_exported() {
    // The same program, implicit export off: `y` runs but is not harvested.
    let finished = Script::new("def y = x; y")
        .capture("x", Value::Int(5))
        .finish();
    assert_eq!(finished.tail, Value::Int(5));
    assert!(
        finished.exports.is_empty(),
        "a plain def is not exported without the option"
    );
}

#[test]
fn explicit_export_works_even_without_the_implicit_export_option() {
    // `export` on a binding is independent of the compiler option: it always
    // harvests that binding, implicit export or not.
    let finished = Script::new("export def y = x; y")
        .capture("x", Value::Int(5))
        .finish();
    assert_eq!(
        finished.exports.get("y"),
        Some(&Value::Int(5)),
        "an explicit export is harvested with the option off"
    );
}

#[test]
fn multiple_outer_names_are_each_captured_independently() {
    // Two free names, both in the outer scope: each becomes its own capture,
    // seated with its own supplied value.
    let tail = Script::new("x - y")
        .captures(&[("x", Value::Int(10)), ("y", Value::Int(3))])
        .run();
    assert_eq!(tail, Value::Int(7));
}

#[test]
fn an_outer_name_the_script_never_uses_may_still_have_a_value_supplied() {
    // An embedder need not filter its environment down to exactly what a
    // fragment captures: a supplied value for an unused outer name is simply
    // ignored, not an error.
    let tail = Script::new("x")
        .captures(&[("x", Value::Int(1)), ("y", Value::Int(2))])
        .run();
    assert_eq!(tail, Value::Int(1));
}

#[test]
fn implicit_export_does_not_reach_inside_a_nested_scope() {
    // `z` is bound inside a `do` block, not at the top level, so implicit
    // export leaves it out even though `y`, the top-level binding around it,
    // is exported.
    let finished = Script::new("def y = do { def z = 1; z + 1 }; y")
        .implicit_export()
        .finish();
    assert_eq!(finished.tail, Value::Int(2));
    assert_eq!(finished.exports.get("y"), Some(&Value::Int(2)));
    assert!(
        !finished.exports.contains_key("z"),
        "a nested-scope binding is never implicitly exported: {:?}",
        finished.exports
    );
}

#[test]
fn a_duplicate_top_level_binding_is_rejected_even_after_shadowing_a_capture() {
    // The capture-shadowing def is allowed once (see
    // `a_top_level_binding_may_shadow_a_capture_of_the_same_name`), but it is
    // itself an ordinary binding: a further `def x` in the same scope collides
    // with it, not with the capture.
    let rendered = Script::new("def x = x; def x = 2; x")
        .capture("x", Value::Int(7))
        .compile_errors()
        .render_plain();
    assert!(
        rendered.contains("already bound"),
        "the second `def x` is a duplicate binding:\n{rendered}"
    );
}

#[test]
fn a_free_name_absent_from_the_outer_scope_is_a_compile_error() {
    // Neither a local, nor an outer-scope name, nor a global: an unbound name.
    let rendered = compile_errors("nope").render_plain();
    assert!(
        rendered.contains("nope"),
        "the diagnostic names the offending identifier:\n{rendered}"
    );
}
