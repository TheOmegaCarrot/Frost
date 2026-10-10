//! A module the filesystem refuses access to is an error for the import, not a
//! module left to the next resolver: it is the host's misconfiguration to fix.
//!
//! These rely on permissions being enforced, which they are not for root, so
//! each first checks that they are, and fails if not.

#![cfg(unix)]

use crate::common;

use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use common::{Tree, importer, run};

/// Withholds all permissions from a path until dropped, then restores them, so
/// the tree can be cleared even after a failed assertion.
struct Locked {
    path: PathBuf,
    restore: Permissions,
}

impl Locked {
    fn new(path: PathBuf) -> Self {
        let restore = fs::metadata(&path).expect("the path exists").permissions();
        fs::set_permissions(&path, Permissions::from_mode(0o000))
            .expect("permissions are settable");
        Self { path, restore }
    }
}

impl Drop for Locked {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.path, self.restore.clone());
    }
}

/// Panic unless `locked` really is inaccessible: as root, it would not be.
fn assert_enforced(locked: &Locked) {
    let readable = if locked.path.is_dir() {
        fs::read_dir(&locked.path).is_ok()
    } else {
        fs::read(&locked.path).is_ok()
    };
    assert!(
        !readable,
        "{} is still readable with no permissions: these tests must not run as root",
        locked.path.display()
    );
}

#[test]
fn an_inaccessible_directory_on_the_way_is_an_error() {
    let tree = Tree::new("permissions/directory");
    tree.file("locked/m.frst", "export def x = 1");
    let locked = Locked::new(tree.path("locked"));
    assert_enforced(&locked);
    for follow in [false, true] {
        let resolver = tree.resolver(&["."]).with_symlinks_followed(follow);
        let message = run("import('locked.m')", &importer(resolver)).message();
        assert!(
            message.starts_with("Cannot import 'locked.m': cannot access '")
                && message.contains("Permission denied"),
            "following symlinks: {follow}\n{message}"
        );
    }
}

#[test]
fn an_unreadable_module_is_an_error() {
    let tree = Tree::new("permissions/file");
    tree.file("m.frst", "export def x = 1");
    let locked = Locked::new(tree.path("m.frst"));
    assert_enforced(&locked);
    let message = run("import('m')", &importer(tree.resolver(&["."]))).message();
    let path = tree.path("m.frst").canonicalize().unwrap();
    assert!(
        message.starts_with(&format!(
            "Cannot import 'm': cannot read '{}': ",
            path.display()
        )) && message.contains("Permission denied"),
        "{message}"
    );
}
