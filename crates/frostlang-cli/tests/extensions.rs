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

#[test]
fn formats_is_installed_with_every_format() {
    let code = r"
        print(sorted(keys(import('ext.formats'))) == ['json', 'toml'])
    ";
    assert_eq!(eval(code), "true\n");
}

#[test]
fn formats_json_round_trips() {
    let code = r#"
        def json = import('ext.formats.json')
        print(json.encode(json.decode('{"a": [1]}')) == '{"a":[1]}')
    "#;
    assert_eq!(eval(code), "true\n");
}

#[test]
fn formats_toml_round_trips() {
    let code = r"
        def toml = import('ext.formats.toml')
        def text = 'when = 1979-05-27\n\n[t]\na = [1, 2]\n'
        print(toml.encode(toml.decode(text)) == text)
    ";
    assert_eq!(eval(code), "true\n");
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
