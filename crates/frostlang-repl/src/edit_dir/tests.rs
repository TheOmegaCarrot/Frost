//! White-box: the editor's directory cannot be observed through the public API
//! without driving an interactive terminal.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;

use super::{ATTEMPTS, EditDir};

/// A fresh, empty directory for one test to work in, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("frost-edit-dir-test-{}-{test}", process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).expect("the scratch directory should be created");
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `count` names, `name-0` onward.
fn names(count: usize) -> Vec<String> {
    (0..count).map(|n| format!("name-{n}")).collect()
}

#[cfg(unix)]
fn mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn a_new_edit_dir_is_a_fresh_private_directory_that_is_removed_when_dropped() {
    let dir = EditDir::new().expect("the directory should be created");
    let path = dir.0.clone();
    assert!(path.is_dir(), "{path:?}");
    assert!(path.starts_with(std::env::temp_dir()), "{path:?}");
    let name = path.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("frost-edit-"), "{name}");
    #[cfg(unix)]
    assert_eq!(mode(&path), 0o700, "only its owner may enter it");

    fs::write(dir.file(), "1 + 1").unwrap();
    drop(dir);
    assert!(!path.exists(), "{path:?} should be removed, with its file");
}

#[test]
fn each_new_edit_dir_has_its_own_name() {
    let (first, second) = (EditDir::new().unwrap(), EditDir::new().unwrap());
    assert_ne!(first.0, second.0);
}

#[test]
fn the_file_is_frost_source_inside_the_directory() {
    let dir = EditDir::new().unwrap();
    let file = dir.file();
    assert_eq!(file.parent(), Some(dir.0.as_path()));
    assert_eq!(file.extension().and_then(|e| e.to_str()), Some("frst"));
}

#[test]
fn a_name_already_taken_is_skipped_and_left_untouched() {
    let scratch = Scratch::new("taken");
    let target = scratch.0.join("target");
    fs::write(&target, "not yours").unwrap();
    fs::write(scratch.0.join("name-0"), "a file").unwrap();
    fs::create_dir(scratch.0.join("name-1")).unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, scratch.0.join("name-2")).unwrap();
        std::os::unix::fs::symlink(scratch.0.join("nowhere"), scratch.0.join("name-3")).unwrap();
    }
    let taken = if cfg!(unix) { 4 } else { 2 };

    let dir = EditDir::create_in(&scratch.0, names(taken + 1)).expect("a free name remains");
    assert_eq!(dir.0, scratch.0.join(format!("name-{taken}")));

    assert_eq!(
        fs::read_to_string(scratch.0.join("name-0")).unwrap(),
        "a file"
    );
    assert!(scratch.0.join("name-1").is_dir());
    #[cfg(unix)]
    {
        assert!(
            fs::symlink_metadata(scratch.0.join("name-2"))
                .unwrap()
                .is_symlink()
        );
        assert!(
            fs::symlink_metadata(scratch.0.join("name-3"))
                .unwrap()
                .is_symlink()
        );
        assert!(
            !scratch.0.join("nowhere").exists(),
            "a dangling symlink is not followed"
        );
    }
    assert_eq!(fs::read_to_string(&target).unwrap(), "not yours");
}

#[test]
fn creation_gives_up_after_its_attempts_are_all_taken() {
    let scratch = Scratch::new("exhausted");
    for name in names(ATTEMPTS) {
        fs::write(scratch.0.join(name), "").unwrap();
    }
    // A free name past the attempts is never reached.
    let error = EditDir::create_in(&scratch.0, names(ATTEMPTS + 1))
        .err()
        .expect("every attempt is taken");
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert!(!scratch.0.join(format!("name-{ATTEMPTS}")).exists());
}

#[test]
fn another_failure_ends_creation_at_once() {
    let scratch = Scratch::new("missing-parent");
    let missing = scratch.0.join("missing");
    let error = EditDir::create_in(&missing, names(ATTEMPTS))
        .err()
        .expect("the parent does not exist");
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}
