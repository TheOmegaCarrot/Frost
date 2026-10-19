//! The first-party extensions `frost` installs.

use std::process::Command;

/// What `frost -e code` prints, which must succeed.
fn eval(code: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_frost"))
        .args(["-e", code])
        .output()
        .expect("`frost` should run");
    assert!(
        output.status.success(),
        "{code:?} should succeed, but failed with: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("the output is UTF-8")
}

#[test]
fn uuid_is_installed() {
    let code = r"
        def uuid = import('ext.uuid')
        print(uuid.version(uuid.v4()))
    ";
    assert_eq!(eval(code), "4\n");
}

/// `frost` links the bundled SQLite, whose header the bindings were built
/// from.
#[test]
fn sqlite_is_installed_bundled() {
    let code = r"
        def sqlite = import('ext.sqlite')
        print(sqlite.version)
    ";
    let bundled = frostlang_sqlite::rusqlite::ffi::SQLITE_VERSION
        .to_str()
        .expect("the version is UTF-8");
    assert_eq!(eval(code), format!("{bundled}\n"));
}
