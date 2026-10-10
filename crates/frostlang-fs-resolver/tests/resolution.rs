//! Which file a specification names, which root serves it, and what an import
//! receives.

use crate::common;

use std::sync::Arc;

use common::{Tree, importer, run};
use frostlang::{FrostError, HostComponent, ImportCtx, ImportResolver, ImporterBuilder, Value};
use frostlang_fs_resolver::FsResolver;

#[test]
fn an_import_receives_the_modules_exports() {
    let tree = Tree::new("resolution/exports");
    tree.file(
        "m.frst",
        r"
        export def a = 1
        def hidden = 2
        export defn double(x) -> x * 2
        ",
    );
    let importer = importer(tree.resolver(&["."]));
    let value = run(
        r"
        def m = import('m')
        [m.a, m.double(21), keys(m)]
        ",
        &importer,
    )
    .value();
    assert_eq!(
        value,
        Value::array([
            Value::Int(1),
            Value::Int(42),
            Value::array([Value::from("a"), Value::from("double")])
        ]),
        "exported bindings only; `hidden` is not exported"
    );
}

#[test]
fn a_module_without_exports_gives_an_empty_map() {
    let tree = Tree::new("resolution/no_exports");
    tree.file("m.frst", "def private = 1");
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("import('m')", &importer).value(),
        Value::map::<&str, 0>([])
    );
}

#[test]
fn an_exported_null_is_present() {
    let tree = Tree::new("resolution/null_export");
    tree.file("m.frst", "export def nothing = null");
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("has(import('m'), 'nothing')", &importer).value(),
        Value::Bool(true)
    );
}

#[test]
fn a_destructuring_export_exports_each_name_it_binds() {
    let tree = Tree::new("resolution/destructuring");
    tree.file(
        "m.frst",
        r"
        export def {a, inner: {b}} = {a: 1, inner: {b: 2}}
        export def [c, ...rest] = [3, 4]
        ",
    );
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("import('m')", &importer).value(),
        Value::map([
            ("a", Value::Int(1)),
            ("b", Value::Int(2)),
            ("c", Value::Int(3)),
            ("rest", Value::array([4])),
        ])
    );
}

#[test]
fn each_name_of_a_specification_is_a_directory_but_the_last() {
    let tree = Tree::new("resolution/deep");
    tree.file("deep/a/b/c.frst", "export def where = 'deep/a/b/c'");
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("import('deep.a.b.c').where", &importer).value(),
        Value::from("deep/a/b/c")
    );
}

#[test]
fn the_first_root_with_the_file_serves_it() {
    let tree = Tree::new("resolution/first_root");
    tree.file("one/dupe.frst", "export def from = 'one'")
        .file("two/dupe.frst", "export def from = 'two'")
        .file("two/only.frst", "export def from = 'two'");
    let importer = importer(tree.resolver(&["one", "two"]));
    assert_eq!(
        run("[import('dupe').from, import('only').from]", &importer).value(),
        Value::array([Value::from("one"), Value::from("two")]),
        "`dupe` from the first root; `only` from the one root that has it"
    );
}

#[test]
fn a_module_finds_its_imports_through_the_roots_not_its_own_directory() {
    // `pkg.outer` imports `inner`: the root's `inner.frst`, not its sibling.
    let tree = Tree::new("resolution/not_relative");
    tree.file("pkg/outer.frst", "export def inner = import('inner').where")
        .file("pkg/inner.frst", "export def where = 'pkg/inner'")
        .file("inner.frst", "export def where = 'inner'");
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("import('pkg.outer').inner", &importer).value(),
        Value::from("inner")
    );
}

#[test]
fn an_unfound_module_is_left_to_the_next_resolver() {
    let tree = Tree::new("resolution/next_resolver");
    let importer = ImporterBuilder::new()
        .append_resolver(Arc::new(tree.resolver(&["."])))
        .append_resolver(Arc::new(Fallback))
        .build();
    assert_eq!(
        run("import('missing')", &importer).value(),
        Value::from("fallback for missing")
    );
}

#[test]
fn an_unfound_module_with_no_other_resolver_cannot_be_resolved() {
    let tree = Tree::new("resolution/unresolved");
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("import('missing')", &importer).message(),
        "Could not resolve import 'missing'"
    );
}

#[test]
fn a_specification_that_names_no_file_is_left_to_the_next_resolver() {
    // Each would name a file outside the tree, or no file, if taken literally.
    let tree = Tree::new("resolution/not_a_name");
    tree.file("a/b.frst", "export def x = 1")
        .file("a-b.frst", "export def x = 1")
        .file("1a.frst", "export def x = 1");
    let importer = ImporterBuilder::new()
        .append_resolver(Arc::new(tree.resolver(&["."])))
        .append_resolver(Arc::new(Fallback))
        .build();
    for spec in [
        "a..b", ".a.b", "a.b.", "a/b", "a-b", "1a", "..", ".", "a.b.frst", "a b",
    ] {
        assert_eq!(
            run(&format!("import('{spec}')"), &importer).value(),
            Value::from(format!("fallback for {spec}")),
            "{spec:?} names no file"
        );
    }
}

#[test]
fn a_directory_is_not_a_module() {
    let tree = Tree::new("resolution/directory");
    tree.dir("m.frst");
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("import('m')", &importer).message(),
        "Could not resolve import 'm'"
    );
}

#[test]
fn a_file_where_a_directory_belongs_is_not_found() {
    // `a.b` names `a/b.frst`, but `a` is a file.
    let tree = Tree::new("resolution/file_in_the_way");
    tree.file("a", "not a directory");
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("import('a.b')", &importer).message(),
        "Could not resolve import 'a.b'"
    );
}

#[test]
fn the_registry_is_consulted_before_any_file() {
    let tree = Tree::new("resolution/registry_first");
    tree.file("host.frst", "export def from = 'file'");
    let importer = ImporterBuilder::new()
        .with_component(
            HostComponent::new("host", Value::map([("from", Value::from("registry"))]))
                .expect("a valid name"),
        )
        .expect("an unclaimed name")
        .append_resolver(Arc::new(tree.resolver(&["."])))
        .build();
    assert_eq!(
        run("import('host').from", &importer).value(),
        Value::from("registry")
    );
}

#[test]
fn roots_are_canonical() {
    let tree = Tree::new("resolution/canonical_roots");
    tree.dir("modules");
    let resolver = FsResolver::new([tree.path("modules/../modules/.")]).expect("a directory");
    let canonical = tree.path("modules").canonicalize().unwrap();
    assert_eq!(resolver.roots(), [canonical]);
}

#[test]
fn a_root_that_is_missing_or_not_a_directory_is_rejected() {
    let tree = Tree::new("resolution/bad_roots");
    tree.file("file", "");
    for root in ["missing", "file"] {
        let error = FsResolver::new([tree.path(root)]).expect_err("an unusable root");
        assert_eq!(error.root(), tree.path(root), "the error names the root");
        assert!(
            error.to_string().contains("cannot use") && error.to_string().contains(root),
            "{error}"
        );
    }
}

/// Claims every specification, answering with a String naming it.
#[derive(Debug)]
struct Fallback;

impl ImportResolver for Fallback {
    fn resolve(&self, _ctx: &ImportCtx, module_spec: &str) -> Result<Option<Value>, FrostError> {
        Ok(Some(Value::from(format!("fallback for {module_spec}"))))
    }
}
