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
        ("sorted(keys(cbor))", "['decode', 'encode']"),
        ("sorted(keys(json))", "['decode', 'encode']"),
        ("sorted(keys(msgpack))", "['decode', 'encode']"),
        ("sorted(keys(toml))", "['datetime', 'decode', 'encode']"),
        ("sorted(keys(yaml))", "['decode', 'encode']"),
    ]);
}

#[test]
fn the_u64_module_holds_its_functions() {
    assert_values(&[(
        "sorted(keys(u64))",
        "['from_string', 'to_float', 'to_string']",
    )]);
}

#[test]
fn the_default_is_every_format() {
    assert_eq!(Formats::default(), Formats::all());
    for format in [
        Format::Cbor,
        Format::Json,
        Format::Msgpack,
        Format::Toml,
        Format::Yaml,
    ] {
        assert!(Formats::all().contains(format), "{format:?} is compiled in");
    }
}

#[test]
fn with_and_without_choose_formats() {
    let json_only = Formats::none().with(Format::Json);
    assert!(json_only.contains(Format::Json));
    assert!(!json_only.contains(Format::Toml));
    assert_eq!(
        json_only,
        Formats::all()
            .without(Format::Cbor)
            .without(Format::Msgpack)
            .without(Format::Toml)
            .without(Format::Yaml)
    );
    assert_eq!(
        json_only
            .clone()
            .with(Format::Cbor)
            .with(Format::Msgpack)
            .with(Format::Toml)
            .with(Format::Yaml),
        Formats::all()
    );
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
    // `u64` comes with any format that has unsigned 64-bit integers.
    assert_eq!(
        module_names(Formats::all()),
        strings(&["cbor", "json", "msgpack", "toml", "u64", "yaml"])
    );
    assert_eq!(
        module_names(Formats::all().without(Format::Toml)),
        strings(&["cbor", "json", "msgpack", "u64", "yaml"])
    );
    assert_eq!(
        module_names(Formats::all().without(Format::Msgpack)),
        strings(&["cbor", "json", "toml", "u64", "yaml"])
    );
    assert_eq!(
        module_names(
            Formats::all()
                .without(Format::Cbor)
                .without(Format::Msgpack)
        ),
        strings(&["json", "toml", "yaml"])
    );
    assert_eq!(
        module_names(Formats::none().with(Format::Cbor)),
        strings(&["cbor", "u64"])
    );
    assert_eq!(
        module_names(Formats::none().with(Format::Msgpack)),
        strings(&["msgpack", "u64"])
    );
    assert_eq!(
        module_names(Formats::none().with(Format::Toml)),
        strings(&["toml"])
    );
    assert_eq!(
        module_names(Formats::none().with(Format::Yaml)),
        strings(&["yaml"])
    );
    assert_eq!(module_names(Formats::none()), strings(&[]));
}

#[test]
fn a_format_left_out_cannot_be_imported() {
    for (formats, left_out) in [
        (Formats::none(), "json"),
        (Formats::all().without(Format::Cbor), "cbor"),
        (Formats::all().without(Format::Json), "json"),
        (Formats::all().without(Format::Msgpack), "msgpack"),
        (Formats::all().without(Format::Toml), "toml"),
        (Formats::all().without(Format::Yaml), "yaml"),
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
