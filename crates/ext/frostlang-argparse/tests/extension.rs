//! The extension as a host installs it and a script imports it.

use frostlang::ImporterBuilder;

use crate::common::{run_source, value};

#[test]
fn the_extension_is_named_argparse() {
    assert_eq!(frostlang_argparse::extension().name(), "argparse");
}

#[test]
fn the_extension_holds_parse() {
    assert_eq!(value("keys(argparse)"), value("['parse']"));
}

#[test]
fn the_extension_is_absent_unless_installed() {
    let error = run_source("import('ext.argparse')", ImporterBuilder::new().build())
        .expect_err("nothing is installed");
    assert_eq!(error.message(), "Could not resolve import 'ext.argparse'");
}
