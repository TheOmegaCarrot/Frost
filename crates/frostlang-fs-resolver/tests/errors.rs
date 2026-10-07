//! A found module that fails to import is an error for the importing script,
//! and a module runs as a module: imported, identified, and attributed in
//! backtraces.

mod common;

use std::sync::Arc;

use common::{Tree, importer, run};
use frostlang_fs_resolver::DiagnosticStyle;
use frostlang_runtime::{FrostError, ImportCtx, ImportResolver, ImporterBuilder, ModuleId, Value};

#[test]
fn a_compile_error_names_the_specification_and_file_and_shows_the_diagnostic() {
    let tree = Tree::new("errors/compile_plain");
    tree.file("bad.frst", "export def x = undefined_name");
    let importer = importer(tree.resolver(&["."]));
    let message = run("import('bad')", &importer).message();
    let path = tree.path("bad.frst").canonicalize().unwrap();
    let headline = format!(
        "Cannot import 'bad': '{}' does not compile:\n",
        path.display()
    );
    assert!(message.starts_with(&headline), "{message}");
    assert!(message.contains("undefined_name"), "{message}");
    assert!(message.is_ascii(), "plain by default:\n{message}");
}

#[test]
fn a_compile_error_may_render_with_unicode_but_never_color() {
    let tree = Tree::new("errors/compile_unicode");
    tree.file("bad.frst", "export def x = undefined_name");
    let importer = importer(
        tree.resolver(&["."])
            .with_diagnostics(DiagnosticStyle::Unicode),
    );
    let message = run("import('bad')", &importer).message();
    assert!(!message.is_ascii(), "box-drawing expected:\n{message}");
    assert!(!message.contains('\x1b'), "no color:\n{message:?}");
}

#[test]
fn a_compile_error_may_be_narrated() {
    let tree = Tree::new("errors/compile_narrated");
    tree.file("bad.frst", "export def x = undefined_name");
    let importer = importer(
        tree.resolver(&["."])
            .with_diagnostics(DiagnosticStyle::Narrated),
    );
    let message = run("import('bad')", &importer).message();
    assert!(message.contains("does not compile"), "{message}");
    assert!(
        message.contains("snippet line 1: export def x = undefined_name"),
        "{message}"
    );
    assert!(!message.contains('|'), "nothing drawn:\n{message}");
}

#[test]
fn a_parse_error_is_a_compile_error() {
    let tree = Tree::new("errors/parse");
    tree.file("bad.frst", "export def = ");
    let importer = importer(tree.resolver(&["."]));
    let message = run("import('bad')", &importer).message();
    assert!(message.contains("does not compile"), "{message}");
}

#[test]
fn a_compile_error_is_catchable() {
    let tree = Tree::new("errors/catchable");
    tree.file("bad.frst", "export def = ");
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("try_call(import, ['bad']).ok", &importer).value(),
        Value::Bool(false)
    );
}

#[test]
fn a_file_that_is_not_utf8_cannot_be_read() {
    let tree = Tree::new("errors/not_utf8");
    tree.file("bad.frst", b"export def x = '\xff'");
    let importer = importer(tree.resolver(&["."]));
    let message = run("import('bad')", &importer).message();
    let path = tree.path("bad.frst").canonicalize().unwrap();
    let headline = format!("Cannot import 'bad': cannot read '{}': ", path.display());
    assert!(message.starts_with(&headline), "{message}");
}

#[test]
fn a_modules_runtime_error_reaches_the_importer_as_raised() {
    let tree = Tree::new("errors/runtime");
    tree.file("bad.frst", "error({reason: 'boom'})");
    let importer = importer(tree.resolver(&["."]));
    let value = run(
        r"
        def caught = try_call(import, ['bad'])
        caught.error.reason
        ",
        &importer,
    )
    .value();
    assert_eq!(value, Value::from("boom"), "the thrown value, unwrapped");
}

#[test]
fn a_backtrace_through_a_module_names_each_file() {
    let tree = Tree::new("errors/backtrace");
    tree.file("lib.frst", "export defn boom() -> [error('boom')]");
    let importer = importer(tree.resolver(&["."]));
    let error = run(
        r"
        defn go() -> [import('lib').boom()]
        go()
        ",
        &importer,
    )
    .error();
    assert_eq!(
        error.with_backtrace().to_string(),
        "boom\n  \
         in error\n  \
         in boom (lib.frst)\n  \
         in go (main.frst)\n  \
         in <main> (main.frst)"
    );
}

#[test]
fn a_module_runs_as_imported() {
    let tree = Tree::new("errors/imported");
    tree.file("m.frst", "export def was_imported = imported()");
    let importer = importer(tree.resolver(&["."]));
    assert_eq!(
        run("[imported(), import('m').was_imported]", &importer).value(),
        Value::array([false, true])
    );
}

#[test]
fn a_module_is_identified_by_its_canonical_path() {
    // `probe` is served by another resolver, which reports who imported it.
    let tree = Tree::new("errors/module_id");
    tree.file("m.frst", "export def importer = import('probe')");
    let importer = ImporterBuilder::new()
        .append_resolver(Arc::new(tree.resolver(&["."])))
        .append_resolver(Arc::new(ImportingModule))
        .build();
    let path = tree.path("m.frst").canonicalize().unwrap();
    assert_eq!(
        run("import('m').importer", &importer).value(),
        Value::from(path.to_string_lossy().as_ref())
    );
}

/// Answers every import with the id of the module importing it, or `null`.
#[derive(Debug)]
struct ImportingModule;

impl ImportResolver for ImportingModule {
    fn resolve(&self, ctx: &ImportCtx, _module_spec: &str) -> Result<Option<Value>, FrostError> {
        Ok(Some(
            ctx.importing_module()
                .map_or(Value::Null, |id: &ModuleId| Value::from(id.as_str())),
        ))
    }
}
