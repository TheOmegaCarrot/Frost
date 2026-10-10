//! Symbolic links below the roots: refused unless followed, and when followed,
//! a module is the file the links lead to.

#![cfg(unix)]

use crate::common;

use common::{Tree, count, importer, run};
use frostlang::Value;
use frostlang_fs_resolver::FsResolver;

/// A tree whose root `modules` reaches `real/m.frst` through a linked file and
/// a linked directory.
fn linked(name: &str) -> Tree {
    let tree = Tree::new(name);
    tree.file(
        "real/m.frst",
        r"
        print('ran m')
        export defn f() -> 'm'
        ",
    )
    .symlink("modules/file.frst", "real/m.frst")
    .symlink("modules/dir", "real")
    .file("modules/plain.frst", "export def plain = true");
    tree
}

#[test]
fn a_linked_file_is_refused_unless_links_are_followed() {
    let tree = linked("symlinks/file_refused");
    let importer = importer(tree.resolver(&["modules"]));
    let message = run("import('file')", &importer).message();
    let link = tree
        .path("modules")
        .canonicalize()
        .unwrap()
        .join("file.frst");
    assert_eq!(
        message,
        format!(
            "Cannot import 'file': '{}' is a symbolic link, which this importer does not follow",
            link.display()
        )
    );
}

#[test]
fn a_linked_directory_is_refused_unless_links_are_followed() {
    let tree = linked("symlinks/dir_refused");
    let importer = importer(tree.resolver(&["modules"]));
    let message = run("import('dir.m')", &importer).message();
    assert!(
        message.starts_with("Cannot import 'dir.m': ") && message.contains("is a symbolic link"),
        "{message}"
    );
}

#[test]
fn a_linked_root_is_fine() {
    // Roots are the host's to choose; only what lies below them is checked.
    let tree = linked("symlinks/root");
    tree.symlink("linked_root", "modules");
    let importer = importer(tree.resolver(&["linked_root"]));
    assert_eq!(
        run("import('plain').plain", &importer).value(),
        Value::Bool(true)
    );
}

#[test]
fn followed_links_lead_to_one_module() {
    // `file` and `dir.m` both lead to `real/m.frst`: one module, run once.
    let tree = linked("symlinks/followed");
    let resolver = FsResolver::new([tree.path("modules")])
        .expect("a directory")
        .with_symlinks_followed(true);
    let run = run("import('file').f == import('dir.m').f", &importer(resolver));
    assert_eq!(count(&run.printed, "ran m"), 1, "{:?}", run.printed);
    assert_eq!(run.value(), Value::Bool(true));
}

#[test]
fn a_dangling_link_is_not_a_module() {
    let tree = Tree::new("symlinks/dangling");
    tree.dir("modules")
        .symlink("modules/gone.frst", "nowhere.frst");
    for follow in [false, true] {
        let resolver = FsResolver::new([tree.path("modules")])
            .expect("a directory")
            .with_symlinks_followed(follow);
        let outcome = run("import('gone')", &importer(resolver)).message();
        if follow {
            assert_eq!(outcome, "Could not resolve import 'gone'", "followed");
        } else {
            assert!(outcome.contains("is a symbolic link"), "refused: {outcome}");
        }
    }
}
