//! Program images: `image::encode` saves a program, `image::decode` loads it
//! back untrusted, and a load fails, with its reason, for bytes that are not an
//! image, an image another runtime version wrote, or a damaged image.
//!
//! The serialized form of a `CompiledFunction` itself is covered by
//! `serialize.rs`; this covers the image around it.

use std::collections::BTreeMap;
use std::sync::Arc;

use frostlang::bytecode::{CompiledFunction, FormatVersion};
use frostlang::compile::{CompilerOptions, compile_in_scope};
use frostlang::image::{self, ImageError};
use frostlang::{Arity, TrustedProgram, Value, Vm};

/// `source`, compiled with `scope` as its enclosing scope.
fn compile(source: &str, scope: &[&str]) -> TrustedProgram {
    compile_in_scope("image.frst", source, CompilerOptions::new(), scope)
        .unwrap_or_else(|errors| panic!("{source:?} compiles:\n{}", errors.render_plain()))
        .code
}

/// The tail value of running `program` with `captures`.
fn run(program: TrustedProgram, captures: &[(&str, Value)]) -> Value {
    let captures: BTreeMap<String, Value> = captures
        .iter()
        .map(|(name, value)| (name.to_string(), value.clone()))
        .collect();
    let closure = program.close(captures).expect("every capture is supplied");
    Vm::factory()
        .build(closure)
        .unwrap()
        .run()
        .unwrap_or_else(|failure| panic!("runs: {}", failure.error()))
        .tail()
        .clone()
}

/// `program` saved as an image and loaded back.
fn round_trip(program: &TrustedProgram) -> TrustedProgram {
    let bytes = image::encode(program);
    Arc::new(image::decode(&bytes).expect("an image just encoded decodes")).assert_trusted()
}

/// A program exercising what an image must carry: constants of every kind,
/// nested functions, and Map keys.
const SAMPLE: &str = r#"
    defn twice(f) -> fn x -> f(f(x))
    def inc = fn n -> n + 1
    def table = {name: "frost", [1]: [1.5, null, x'00ff'], [true]: {nested: "yes"}}
    [twice(inc)(1), table.name, table[1], table[true].nested]
"#;

// --- Round trip ---

#[test]
fn a_loaded_image_runs_as_the_program_it_was_saved_from() {
    let program = compile(SAMPLE, &[]);
    let expected = run(compile(SAMPLE, &[]), &[]);
    assert_eq!(run(round_trip(&program), &[]), expected);
}

#[test]
fn a_loaded_image_keeps_the_programs_captures() {
    let program = compile("greeting + ', ' + name", &["greeting", "name"]);
    let loaded = round_trip(&program);
    assert_eq!(
        loaded.capture_names().collect::<Vec<_>>(),
        program.capture_names().collect::<Vec<_>>()
    );
    let captures = [
        ("greeting", Value::from("hello")),
        ("name", Value::from("world")),
    ];
    assert_eq!(run(loaded, &captures), Value::from("hello, world"));
}

#[test]
fn an_image_is_recognized_as_one() {
    let bytes = image::encode(&compile(SAMPLE, &[]));
    assert!(image::is_image(&bytes));
}

#[test]
fn source_is_not_an_image() {
    for source in [SAMPLE.as_bytes(), b"", b"\0", b"frost image"] {
        assert!(!image::is_image(source), "{source:?}");
        assert_eq!(
            image::decode(source).unwrap_err(),
            ImageError::NotAnImage,
            "{source:?}"
        );
    }
}

// --- Images that cannot load ---

/// The bytes before the version in an image: its fixed header.
fn header() -> Vec<u8> {
    let bytes = image::encode(&compile("1", &[]));
    let version = postcard::to_allocvec(env!("CARGO_PKG_VERSION")).unwrap();
    let at = bytes
        .windows(version.len())
        .position(|window| window == version.as_slice())
        .expect("an image holds the version that wrote it");
    bytes[..at].to_vec()
}

#[test]
fn an_image_from_another_version_reports_that_version() {
    let mut bytes = header();
    bytes.extend(postcard::to_allocvec("0.0.0-elsewhere").unwrap());
    bytes.extend(b"the rest does not matter");
    let error = image::decode(&bytes).unwrap_err();
    assert_eq!(
        error,
        ImageError::VersionMismatch {
            found: "0.0.0-elsewhere".to_string()
        }
    );
    assert_eq!(
        error.to_string(),
        format!(
            "the image was written by frostlang 0.0.0-elsewhere, but this is {}",
            env!("CARGO_PKG_VERSION")
        )
    );
}

#[test]
fn a_truncated_image_is_damaged() {
    let bytes = image::encode(&compile(SAMPLE, &[]));
    let header_len = header().len();
    // Cut everywhere from just past the header to just short of the end.
    for len in (header_len..bytes.len()).step_by(7) {
        assert_eq!(
            image::decode(&bytes[..len]).unwrap_err(),
            ImageError::Damaged,
            "cut to {len} of {} bytes",
            bytes.len()
        );
    }
}

#[test]
fn a_header_alone_is_damaged() {
    assert_eq!(image::decode(&header()).unwrap_err(), ImageError::Damaged);
}

#[test]
fn errors_explain_themselves() {
    assert_eq!(ImageError::NotAnImage.to_string(), "not a Frost image");
    assert_eq!(ImageError::Damaged.to_string(), "the image is damaged");
}

// --- What cannot be saved ---

#[test]
#[should_panic(expected = "an image's constants hold no function or Opaque value")]
fn a_program_with_a_function_constant_cannot_be_saved() {
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".into(),
        origin: None,
        code: Vec::new(),
        child_fns: Vec::new(),
        constants: vec![Value::native("f", Arity::Exact(0), |_, _| Ok(Value::Null))],
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    })
    .assert_trusted();
    image::encode(&program);
}
