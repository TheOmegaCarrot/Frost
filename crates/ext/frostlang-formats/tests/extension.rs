//! The extension as a host installs it, with the formats it chooses, and a
//! script imports it.

use frostlang::{FrostError, ImporterBuilder, Value};
use frostlang_formats::{Format, Formats};

use crate::common::{assert_values, importer, run_source};

/// Run `source` with the extension installed with `formats`.
fn run_with(source: &str, formats: Formats) -> Result<Value, FrostError> {
    run_source(source, importer(formats))
}

/// The names of the modules `formats` provides, sorted.
fn module_names(formats: Formats) -> Value {
    run_with("sorted(keys(import('ext.formats')))", formats).expect("the extension is present")
}

/// A Frost Array of `strings`.
fn strings(strings: &[&str]) -> Value {
    strings.iter().map(|&string| Value::from(string)).collect()
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
        (
            r"
            def formats = import('ext.formats')
            all(keys(formats), fn name -> has(formats[name], 'encode') and has(formats[name], 'decode'))
            ",
            "true",
        ),
        ("sorted(keys(json))", "['decode', 'encode']"),
        ("sorted(keys(toml))", "['datetime', 'decode', 'encode']"),
    ]);
}

#[test]
fn the_default_is_every_format() {
    assert_eq!(Formats::default(), Formats::all());
    for format in [Format::Json, Format::Toml] {
        assert!(Formats::all().contains(format), "{format:?} is compiled in");
    }
}

#[test]
fn with_and_without_choose_formats() {
    let json_only = Formats::none().with(Format::Json);
    assert!(json_only.contains(Format::Json));
    assert!(!json_only.contains(Format::Toml));
    assert_eq!(json_only, Formats::all().without(Format::Toml));
    assert_eq!(json_only.clone().with(Format::Toml), Formats::all());
    assert!(!Formats::none().contains(Format::Json));
    assert_eq!(
        json_only.clone().with(Format::Json),
        json_only,
        "adding a format already present changes nothing"
    );
    assert_eq!(
        Formats::none().without(Format::Json),
        Formats::none(),
        "removing a format absent changes nothing"
    );
}

#[test]
fn only_the_chosen_formats_are_modules() {
    assert_eq!(module_names(Formats::all()), strings(&["json", "toml"]));
    assert_eq!(
        module_names(Formats::all().without(Format::Toml)),
        strings(&["json"])
    );
    assert_eq!(
        module_names(Formats::all().without(Format::Json)),
        strings(&["toml"])
    );
    assert_eq!(module_names(Formats::none()), strings(&[]));
}

#[test]
fn a_format_left_out_cannot_be_imported() {
    for (formats, left_out) in [
        (Formats::none(), "json"),
        (Formats::all().without(Format::Json), "json"),
        (Formats::all().without(Format::Toml), "toml"),
    ] {
        let path = format!("ext.formats.{left_out}");
        let error = run_with(&format!("import('{path}')"), formats.clone())
            .expect_err("the format is left out");
        assert_eq!(
            error.message(),
            format!("Could not resolve import '{path}'"),
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
