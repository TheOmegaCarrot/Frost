//! The extension as a host installs it, with the formats it chooses, and a
//! script imports it.

use frostlang::{FrostError, ImporterBuilder, Value, ValueMap};
use frostlang_formats::{Format, Formats};

use crate::common::{assert_values, importer, run_source};

/// Run `source` with the extension installed with `formats`.
fn run_with(source: &str, formats: Formats) -> Result<Value, FrostError> {
    run_source(source, importer(formats))
}

#[test]
fn the_extension_is_named_formats() {
    assert_eq!(
        frostlang_formats::extension(Formats::all()).name(),
        "formats"
    );
}

#[test]
fn every_format_is_a_module_holding_encode_and_decode() {
    assert_values(&[
        ("sorted(keys(import('ext.formats')))", "['json']"),
        (
            r"
            def formats = import('ext.formats')
            all(keys(formats), fn name -> has(formats[name], 'encode') and has(formats[name], 'decode'))
            ",
            "true",
        ),
        ("sorted(keys(json))", "['decode', 'encode']"),
    ]);
}

#[test]
fn the_default_is_every_format() {
    assert_eq!(Formats::default(), Formats::all());
    assert!(Formats::all().contains(Format::Json), "JSON is compiled in");
}

#[test]
fn with_and_without_choose_formats() {
    assert!(!Formats::none().contains(Format::Json));
    assert!(Formats::none().with(Format::Json).contains(Format::Json));
    assert!(!Formats::all().without(Format::Json).contains(Format::Json));
    assert_eq!(Formats::none().with(Format::Json), Formats::all());
    assert_eq!(
        Formats::all().with(Format::Json),
        Formats::all(),
        "adding a format already present changes nothing"
    );
    assert_eq!(
        Formats::none().without(Format::Json),
        Formats::none(),
        "removing a format absent changes nothing"
    );
}

#[test]
fn a_format_left_out_is_absent() {
    for formats in [Formats::none(), Formats::all().without(Format::Json)] {
        assert_eq!(
            run_with("import('ext.formats')", formats.clone()).expect("the extension is present"),
            Value::from(ValueMap::new()),
            "{formats:?} provides no module"
        );
        let error =
            run_with("import('ext.formats.json')", formats.clone()).expect_err("JSON is left out");
        assert_eq!(
            error.message(),
            "Could not resolve import 'ext.formats.json'",
            "{formats:?}"
        );
    }
}

#[test]
fn the_extension_is_absent_unless_installed() {
    let error = run_source("import('ext.formats')", ImporterBuilder::new().build())
        .expect_err("nothing is installed");
    assert_eq!(error.message(), "Could not resolve import 'ext.formats'");
}
