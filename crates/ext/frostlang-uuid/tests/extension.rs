//! The extension as a host installs it and a script imports it.

use frostlang::ImporterBuilder;

use crate::common::{assert_values, run_source};

#[test]
fn the_extension_is_named_uuid() {
    assert_eq!(frostlang_uuid::extension().name(), "uuid");
}

#[test]
fn the_extension_holds_its_functions() {
    assert_values(&[
        (
            "sorted(keys(uuid))",
            "['timestamp', 'to_bytes', 'to_string', 'ulid', 'v4', 'v7', 'version']",
        ),
        (
            "sorted(keys(uuid.ulid))",
            "['new', 'timestamp', 'to_bytes', 'to_string']",
        ),
        (
            "uuid.ulid.to_string(x'ffffffffffffffffffffffffffffffff')",
            "import('ext.uuid.ulid').to_string(x'ffffffffffffffffffffffffffffffff')",
        ),
    ]);
}

#[test]
fn the_extension_is_absent_unless_installed() {
    let error = run_source("import('ext.uuid')", ImporterBuilder::new().build())
        .expect_err("nothing is installed");
    assert_eq!(error.message(), "Could not resolve import 'ext.uuid'");
}
