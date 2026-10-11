//! `Extension::builder` and `HostComponent::builder`: a component's content
//! built entry by entry, then imported from Frost source, as `ext.{name}` for
//! an extension and as `{name}` for a host component.

use std::sync::Arc;

use frostlang::{ComponentBuilder, Extension, HostComponent, ImporterBuilder, Value, frost_fn};

use crate::script::{Script, UNOPTIMIZED};

/// The Frost function `source` evaluates to. Run under one set of options, as
/// the function's code differs between them.
fn frost_function(source: &str) -> Value {
    Script::new(source).run_under(UNOPTIMIZED)
}

#[frost_fn(name = "demo.double")]
fn double(n: i64) -> i64 {
    n * 2
}

#[frost_fn(name = "demo.inner.triple")]
fn triple(n: i64) -> i64 {
    n * 3
}

#[frost_fn(name = "plain")]
fn plain() -> &'static str {
    "plain"
}

/// The tail of `source`, with `extension` importable.
fn run_with(extension: Extension, source: &str) -> Value {
    let importer = ImporterBuilder::new()
        .with_extension(extension)
        .expect("a lone extension is accepted")
        .build();
    Script::new(source).importer(Arc::clone(&importer)).run()
}

/// The tail of `source`, with `component` importable.
fn run_with_component(component: HostComponent, source: &str) -> Value {
    let importer = ImporterBuilder::new()
        .with_component(component)
        .expect("a lone component is accepted")
        .build();
    Script::new(source).importer(Arc::clone(&importer)).run()
}

// ============================================================
// Entries
// ============================================================

#[test]
fn each_kind_of_entry_is_importable() {
    let demo = Extension::builder("demo")
        .function(double_frost())
        .function(plain_frost())
        .value("answer", Value::Int(42))
        .module("inner", |inner| inner.function(triple_frost()))
        .build();
    assert_eq!(demo.name(), "demo");
    let source = r"
        def demo = import('ext.demo')
        [demo.double(2), demo.plain(), demo.answer, demo.inner.triple(2)]
    ";
    assert_eq!(
        run_with(demo, source),
        Value::array([
            Value::Int(4),
            Value::from("plain"),
            Value::Int(42),
            Value::Int(6)
        ])
    );
}

#[test]
fn a_function_is_keyed_by_its_name_after_the_last_dot() {
    let demo = Extension::builder("demo")
        .function(double_frost())
        .module("inner", |inner| inner.function(triple_frost()))
        .build();
    let source = r"
        def demo = import('ext.demo')
        [keys(demo), keys(demo.inner)]
    ";
    assert_eq!(
        run_with(demo, source),
        Value::array([
            Value::array([Value::from("double"), Value::from("inner")]),
            Value::array([Value::from("triple")]),
        ])
    );
}

#[test]
fn a_frost_function_is_keyed_by_its_own_name() {
    let halve = frost_function("fn halve(n) -> n / 2");
    let demo = Extension::builder("demo").function(halve).build();
    let source = r"
        def demo = import('ext.demo')
        demo.halve(8)
    ";
    assert_eq!(run_with(demo, source), Value::Int(4));
}

/// A group adding `double`, to a component of either kind.
fn doubling<T>(component: ComponentBuilder<T>) -> ComponentBuilder<T> {
    component.function(double_frost())
}

#[test]
fn groups_are_added_with_with_and_with_if_its_condition_holds_at_any_level() {
    let demo = |extra: bool| {
        Extension::builder("demo")
            .with(doubling)
            .module("inner", |inner| {
                inner
                    .with_if(extra, doubling)
                    .with_if(!extra, |inner| inner.function(triple_frost()))
            })
            .build()
    };
    let source = r"
        def demo = import('ext.demo')
        [keys(demo), keys(demo.inner)]
    ";
    assert_eq!(
        run_with(demo(true), source),
        Value::array([
            Value::array([Value::from("double"), Value::from("inner")]),
            Value::array([Value::from("double")]),
        ])
    );
    assert_eq!(
        run_with(demo(false), source),
        Value::array([
            Value::array([Value::from("double"), Value::from("inner")]),
            Value::array([Value::from("triple")]),
        ])
    );
}

#[test]
fn an_empty_extension_is_an_empty_map() {
    let source = "import('ext.demo')";
    assert_eq!(
        run_with(Extension::builder("demo").build(), source),
        Value::map::<&str, 0>([])
    );
}

// ============================================================
// Host components
// ============================================================

#[test]
fn a_host_component_is_built_alike_and_imported_by_its_name() {
    let app = HostComponent::builder("app")
        .with(doubling)
        .value("answer", Value::Int(42))
        .module("inner", |inner| inner.function(triple_frost()))
        .build();
    assert_eq!(app.name(), "app");
    let source = r"
        def app = import('app')
        [app.double(2), app.answer, app.inner.triple(2)]
    ";
    assert_eq!(
        run_with_component(app, source),
        Value::array([Value::Int(4), Value::Int(42), Value::Int(6)])
    );
}

#[test]
#[should_panic(expected = r#"invalid component name "not-valid""#)]
fn a_host_component_name_that_is_not_an_identifier_panics() {
    let _ = HostComponent::builder("not-valid");
}

#[test]
#[should_panic(
    expected = r#"host component `app` cannot key an entry "my-key": a key is a Frost identifier, not a keyword"#
)]
fn a_host_component_names_itself_in_a_key_panic() {
    let _ = HostComponent::builder("app").value("my-key", Value::Null);
}

#[test]
#[should_panic(expected = "host component `app.inner` already has an entry keyed `triple`")]
fn a_host_component_names_its_module_path_in_a_repeat_panic() {
    let _ = HostComponent::builder("app").module("inner", |inner| {
        inner.function(triple_frost()).function(triple_frost())
    });
}

#[test]
#[should_panic(expected = "host component `app` takes only a Function through `function`, got Int")]
fn a_host_component_names_itself_in_a_function_panic() {
    let _ = HostComponent::builder("app").function(Value::Int(1));
}

// ============================================================
// Panics
// ============================================================

#[test]
#[should_panic(expected = r#"invalid component name "not-valid""#)]
fn a_name_that_is_not_an_identifier_panics() {
    let _ = Extension::builder("not-valid");
}

#[test]
#[should_panic(expected = r#"invalid component name "map""#)]
fn a_keyword_name_panics() {
    let _ = Extension::builder("map");
}

#[test]
#[should_panic(
    expected = r#"extension `demo` cannot key an entry "my-key": a key is a Frost identifier, not a keyword"#
)]
fn a_key_that_is_not_an_identifier_panics() {
    let _ = Extension::builder("demo").value("my-key", Value::Null);
}

#[test]
#[should_panic(
    expected = r#"extension `demo` cannot key an entry "if": a key is a Frost identifier, not a keyword"#
)]
fn a_keyword_key_panics() {
    let _ = Extension::builder("demo").value("if", Value::Null);
}

#[test]
#[should_panic(expected = "extension `demo` already has an entry keyed `double`")]
fn a_repeated_key_panics() {
    let _ = Extension::builder("demo")
        .function(double_frost())
        .value("double", Value::Null);
}

#[test]
#[should_panic(expected = "extension `demo.inner` already has an entry keyed `triple`")]
fn a_repeated_key_in_a_module_panics_naming_its_path() {
    let _ = Extension::builder("demo").module("inner", |inner| {
        inner.function(triple_frost()).function(triple_frost())
    });
}

#[test]
#[should_panic(expected = "extension `demo` takes only a Function through `function`, got Int")]
fn a_function_that_is_not_a_function_panics() {
    let _ = Extension::builder("demo").function(Value::Int(1));
}

#[test]
#[should_panic(expected = "extension `demo` cannot key an entry")]
fn an_anonymous_frost_function_panics_for_want_of_a_key() {
    let anonymous = frost_function("fn n -> n");
    let _ = Extension::builder("demo").function(anonymous);
}
