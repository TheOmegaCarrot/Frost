//! The extension as a host installs it and a script imports it.

use frostlang::{ImporterBuilder, Value};
use frostlang_sqlite::rusqlite;

use crate::common::{assert_values, run_source, value};

#[test]
fn the_extension_is_named_sqlite() {
    assert_eq!(frostlang_sqlite::extension().name(), "sqlite");
}

#[test]
fn the_extension_holds_its_entries() {
    assert_values(&[("sorted(keys(sqlite))", "['version']")]);
}

#[test]
fn version_is_the_version_of_the_library_in_use() {
    assert_eq!(
        value("sqlite.version"),
        Value::from(rusqlite::version()),
        "`version` is what the linked library reports"
    );
    // The tests link the bundled SQLite, whose header the bindings were built
    // from, so the library in use is the one the header describes.
    let header_version = rusqlite::ffi::SQLITE_VERSION
        .to_str()
        .expect("the header version is ASCII");
    assert_eq!(
        value("sqlite.version"),
        Value::from(header_version),
        "`version` is the bundled SQLite's"
    );
    assert_values(&[("sqlite.version", "import('ext.sqlite.version')")]);
}

#[test]
fn the_extension_is_absent_unless_installed() {
    let error = run_source("import('ext.sqlite')", ImporterBuilder::new().build())
        .expect_err("nothing is installed");
    assert_eq!(error.message(), "Could not resolve import 'ext.sqlite'");
}
