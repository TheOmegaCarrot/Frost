//! The Types globals, from Frost source: the `is_*` predicates, `type`,
//! rendering with `to_string` and `pretty`, and the conversions `to_int`,
//! `to_float`, `to_bytes`, and `from_utf8`.
//!
//! A predicate accepts any value. A conversion given a value of the wrong type
//! raises a type error; given the right type with content that will not convert,
//! it returns Null.
//!
//! The rendering itself is pinned by the runtime's own tests; these pin which
//! rendering each global is.
//!
//! The harness runs every case under every optimization permutation, so a call
//! over a literal is checked both folded and at run time. Cases that capture
//! their input are never folded.

mod source;

use std::borrow::Cow;

use frostlang_runtime::{FrostOpaque, Value};
use source::assertions::{Library, library_assertions};
use source::{Script, raises, run};

library_assertions!(Library::GLOBALS);

fn bytes(octets: &[u8]) -> Value {
    Value::from(octets)
}

/// Assert each `source` runs to the value `expected`.
fn assert_results(cases: &[(&str, Value)]) {
    for (source, expected) in cases {
        assert_eq!(&run(source), expected, "{source:?} is {expected:?}");
    }
}

/// A value of each type, as Frost source, with the name of its type. Functions
/// include a lambda and globals of several kinds.
const SAMPLES: &[(&str, &str)] = &[
    ("null", "Null"),
    ("false", "Bool"),
    ("true", "Bool"),
    ("0", "Int"),
    ("-7", "Int"),
    ("1.5", "Float"),
    ("''", "String"),
    ("'text'", "String"),
    ("x''", "Bytes"),
    ("x'ff'", "Bytes"),
    ("[]", "Array"),
    ("[null]", "Array"),
    ("{}", "Map"),
    ("{a: 1}", "Map"),
    ("fn -> null", "Function"),
    ("type", "Function"),
    ("plus", "Function"),
    ("inv", "Function"),
    ("opaque", "Opaque"),
];

/// Host data for the `opaque` sample: Frost source cannot make an Opaque.
#[derive(Debug)]
struct Payload;

impl FrostOpaque for Payload {
    fn type_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Payload")
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

/// Run `source`, which may use the samples, with `opaque` captured.
fn run_over_samples(source: &str) -> Value {
    Script::new(source)
        .capture("opaque", Value::opaque(Payload))
        .run()
}

/// Each type predicate, with the types it accepts.
const PREDICATES: &[(&str, &[&str])] = &[
    ("is_null", &["Null"]),
    ("is_bool", &["Bool"]),
    ("is_int", &["Int"]),
    ("is_float", &["Float"]),
    ("is_string", &["String"]),
    ("is_bytes", &["Bytes"]),
    ("is_array", &["Array"]),
    ("is_map", &["Map"]),
    ("is_function", &["Function"]),
    ("is_opaque", &["Opaque"]),
    (
        "is_nonnull",
        &[
            "Bool", "Int", "Float", "String", "Bytes", "Array", "Map", "Function", "Opaque",
        ],
    ),
    ("is_numeric", &["Int", "Float"]),
    (
        "is_primitive",
        &["Null", "Bool", "Int", "Float", "String", "Bytes"],
    ),
    ("is_structured", &["Array", "Map"]),
    ("is_flat", &["String", "Bytes"]),
];

/// Frost source for an Array of `function` applied to each sample.
fn applied_to_samples(function: &str) -> String {
    let calls: Vec<String> = SAMPLES
        .iter()
        .map(|(sample, _)| format!("{function}({sample})"))
        .collect();
    format!("[{}]", calls.join(", "))
}

/// Assert `function`, applied to an argument of each type it rejects, raises
/// its type error: it requires `requires` as argument 1.
fn assert_rejects(function: &str, requires: &str, rejected: &[(&str, &str)]) {
    for (argument, ty) in rejected {
        let source = format!("{function}({argument})");
        assert_eq!(
            raises(&source),
            format!("Function {function} requires {requires} as argument 1, got {ty}"),
            "{source:?}"
        );
    }
}

// --- is_* ---

#[test]
fn each_type_predicate_accepts_exactly_its_types() {
    for (predicate, accepted) in PREDICATES {
        let expected: Value = SAMPLES
            .iter()
            .map(|(_, ty)| Value::Bool(accepted.contains(ty)))
            .collect();
        assert_eq!(
            run_over_samples(&applied_to_samples(predicate)),
            expected,
            "{predicate} over {SAMPLES:?}"
        );
    }
}

#[test]
fn each_type_predicate_agrees_with_its_match_constraint() {
    // `is_flat` tests what the constraint `Flat` does, and so on for each.
    for (predicate, _) in PREDICATES {
        let suffix = predicate.strip_prefix("is_").expect("a predicate is is_*");
        let constraint = suffix[..1].to_uppercase() + &suffix[1..];
        let matches: Vec<String> = SAMPLES
            .iter()
            .map(|(sample, _)| {
                format!("match {sample} {{ _ is {constraint} => true, _ => false }}")
            })
            .collect();
        assert_eq!(
            run_over_samples(&applied_to_samples(predicate)),
            run_over_samples(&format!("[{}]", matches.join(", "))),
            "{predicate} is the constraint {constraint}"
        );
    }
}

#[test]
fn type_predicates_test_runtime_values() {
    let tested = Script::new("[is_string(v), is_flat(v), is_bytes(v), is_nonnull(v)]")
        .capture("v", Value::from("x"))
        .run();
    assert_eq!(tested, run("[true, true, false, true]"));
}

#[test]
fn type_predicates_take_exactly_one_argument() {
    for (predicate, _) in PREDICATES {
        assert_arity(predicate, 1, &[0, 2]);
    }
}

// --- type ---

#[test]
fn type_names_the_type_of_its_argument() {
    let expected: Value = SAMPLES.iter().map(|(_, ty)| Value::from(*ty)).collect();
    assert_eq!(run_over_samples(&applied_to_samples("type")), expected);
}

#[test]
fn type_names_the_type_of_a_runtime_value() {
    let named = Script::new("type(v)").capture("v", bytes(&[0xff])).run();
    assert_eq!(named, Value::from("Bytes"));
}

#[test]
fn type_takes_exactly_one_argument() {
    assert_arity("type", 1, &[0, 2]);
}

// --- to_string ---

#[test]
fn to_string_renders_the_compact_form() {
    assert_values(&[
        // A String at the top level is its own text.
        ("to_string('hi')", "'hi'"),
        ("to_string('')", "''"),
        ("to_string(null)", "'null'"),
        ("to_string(true)", "'true'"),
        ("to_string(42)", "'42'"),
        ("to_string(-1.5)", "'-1.5'"),
        ("to_string(1.0)", "'1.0'"),
        ("to_string(x'6869')", r#""x'6869'""#),
        // A structure renders on one line, with a nested String quoted.
        ("to_string([1, 'a'])", r#"'[ 1, "a" ]'"#),
        ("to_string({b: 1, a: 'x'})", r#"'{ ["a"]: "x", ["b"]: 1 }'"#),
        ("to_string([])", "'[]'"),
        ("to_string({})", "'{}'"),
        ("to_string(fn -> 1)", "'<Function>'"),
        ("to_string(print)", "'<Function>'"),
    ]);
}

#[test]
fn to_string_is_what_a_format_string_interpolates() {
    for (sample, _) in SAMPLES {
        assert_eq!(
            run_over_samples(&format!("to_string({sample})")),
            run_over_samples(&format!("$'${{{sample}}}'")),
            "{sample}"
        );
    }
}

#[test]
fn to_string_renders_a_runtime_value() {
    let rendered = Script::new("to_string(v)")
        .capture("v", Value::array([Value::from("a"), Value::Int(1)]))
        .run();
    assert_eq!(rendered, Value::from(r#"[ "a", 1 ]"#));
}

#[test]
fn to_string_takes_exactly_one_argument() {
    assert_arity("to_string", 1, &[0, 2]);
}

// --- pretty ---

#[test]
fn pretty_renders_a_non_structure_as_to_string_does() {
    for value in [
        "null", "true", "42", "-1.5", "'hi'", "''", "x'6869'", "fn -> 1", "[]", "{}",
    ] {
        assert_eq!(
            run(&format!("pretty({value})")),
            run(&format!("to_string({value})")),
            "{value}"
        );
    }
}

#[test]
fn pretty_renders_a_structure_indented_across_lines() {
    assert_results(&[
        (
            "pretty([1, 'a'])",
            Value::from(
                r#"[
    1,
    "a"
]"#,
            ),
        ),
        (
            "pretty({a: [1, {b: 2}], ['b c']: [], [1]: {}})",
            Value::from(
                r#"{
    [1]: {},
    a: [
        1,
        {
            b: 2
        }
    ],
    ["b c"]: []
}"#,
            ),
        ),
    ]);
}

#[test]
fn pretty_takes_exactly_one_argument() {
    assert_arity("pretty", 1, &[0, 2]);
}

// --- to_int ---

#[test]
fn to_int_converts_a_number() {
    assert_values(&[
        ("to_int(7)", "7"),
        ("to_int(-7)", "-7"),
        // A Float truncates toward zero.
        ("to_int(1.9)", "1"),
        ("to_int(-1.9)", "-1"),
        ("to_int(0.5)", "0"),
        ("to_int(-0.5)", "0"),
        ("to_int(2.0)", "2"),
        // The Floats nearest the ends of the Int range.
        ("to_int(9223372036854774784.0)", "9223372036854774784"),
        ("to_int(-9223372036854775808.0)", "-9223372036854775807 - 1"),
    ]);
}

#[test]
fn to_int_returns_null_for_a_float_past_the_int_range() {
    for source in [
        "to_int(9223372036854775808.0)",
        "to_int(-9223372036854777856.0)",
        "to_int(1e300)",
        "to_int(-1e300)",
    ] {
        assert_eq!(run(source), Value::Null, "{source:?} is Null");
    }
}

#[test]
fn to_int_parses_a_string() {
    assert_values(&[
        ("to_int('42')", "42"),
        ("to_int('-42')", "-42"),
        ("to_int('0')", "0"),
        ("to_int('9223372036854775807')", "9223372036854775807"),
    ]);
    assert_results(&[("to_int('-9223372036854775808')", Value::Int(i64::MIN))]);
}

#[test]
fn to_int_returns_null_for_a_string_that_is_not_an_int() {
    for source in [
        "to_int('')",
        "to_int('abc')",
        "to_int('1.5')",
        "to_int('1e3')",
        "to_int('0x10')",
        "to_int('1_000')",
        "to_int(' 12')",
        "to_int('12 ')",
        // Past the largest Int.
        "to_int('9223372036854775808')",
    ] {
        assert_eq!(run(source), Value::Null, "{source:?} is Null");
    }
}

#[test]
fn to_int_converts_a_runtime_value() {
    let convert = |value: Value| Script::new("to_int(v)").capture("v", value).run();
    assert_eq!(convert(Value::from("-12")), Value::Int(-12));
    assert_eq!(convert(Value::from("twelve")), Value::Null);
}

#[test]
fn to_int_rejects_every_type_but_int_float_and_string() {
    assert_rejects(
        "to_int",
        "Int or Float or String",
        &[
            ("null", "Null"),
            ("true", "Bool"),
            // Bytes spelling "42" are still not a number.
            ("x'3432'", "Bytes"),
            ("[1]", "Array"),
            ("{a: 1}", "Map"),
            ("fn -> 1", "Function"),
        ],
    );
}

#[test]
fn to_int_takes_exactly_one_argument() {
    assert_arity("to_int", 1, &[0, 2]);
}

// --- to_float ---

#[test]
fn to_float_converts_a_number() {
    assert_values(&[
        ("to_float(2.5)", "2.5"),
        ("to_float(-2.5)", "-2.5"),
        ("to_float(3)", "3.0"),
        ("to_float(-3)", "-3.0"),
        ("to_float(0)", "0.0"),
    ]);
}

#[test]
fn to_float_parses_a_string() {
    assert_values(&[
        ("to_float('3.5')", "3.5"),
        ("to_float('-0.25')", "-0.25"),
        ("to_float('3')", "3.0"),
        ("to_float('1e3')", "1000.0"),
    ]);
}

#[test]
fn to_float_returns_null_for_a_string_that_is_not_a_float() {
    for source in [
        "to_float('')",
        "to_float('abc')",
        "to_float('1_000')",
        "to_float(' 1')",
        // A Float is never NaN or infinite.
        "to_float('NaN')",
        "to_float('inf')",
        "to_float('infinity')",
        "to_float('1e400')",
    ] {
        assert_eq!(run(source), Value::Null, "{source:?} is Null");
    }
}

#[test]
fn to_float_converts_a_runtime_value() {
    let convert = |value: Value| Script::new("to_float(v)").capture("v", value).run();
    assert_eq!(convert(Value::Int(4)), run("4.0"));
    assert_eq!(convert(Value::from("four")), Value::Null);
}

#[test]
fn to_float_rejects_every_type_but_int_float_and_string() {
    assert_rejects(
        "to_float",
        "Int or Float or String",
        &[
            ("null", "Null"),
            ("false", "Bool"),
            // Bytes spelling "3.5" are still not a number.
            ("x'332e35'", "Bytes"),
            ("[1.5]", "Array"),
            ("{a: 1.5}", "Map"),
            ("fn -> 1.5", "Function"),
        ],
    );
}

#[test]
fn to_float_takes_exactly_one_argument() {
    assert_arity("to_float", 1, &[0, 2]);
}

// --- to_bytes ---

#[test]
fn to_bytes_encodes_a_string_as_utf8() {
    assert_results(&[
        ("to_bytes('hi')", bytes(b"hi")),
        ("to_bytes('')", bytes(b"")),
        // One two-byte and one three-byte encoding.
        (r"to_bytes('\u{e9}')", bytes(&[0xc3, 0xa9])),
        (r"to_bytes('\u{20ac}')", bytes(&[0xe2, 0x82, 0xac])),
        // Four bytes: a character outside the Basic Multilingual Plane.
        (r"to_bytes('\u{1f600}')", bytes(&[0xf0, 0x9f, 0x98, 0x80])),
        (r"to_bytes('a\u{0}b')", bytes(&[b'a', 0x00, b'b'])),
    ]);
}

#[test]
fn to_bytes_result_is_bytes_not_a_string() {
    assert_results(&[
        ("is_bytes(to_bytes('hi'))", Value::Bool(true)),
        ("is_string(to_bytes('hi'))", Value::Bool(false)),
        ("to_bytes('hi') == x'6869'", Value::Bool(true)),
        ("to_bytes('hi') == 'hi'", Value::Bool(false)),
    ]);
}

#[test]
fn to_bytes_passes_bytes_through_unchanged() {
    assert_results(&[
        ("to_bytes(x'6869')", bytes(b"hi")),
        ("to_bytes(x'')", bytes(b"")),
        // Bytes need not be UTF-8: nothing is decoded or validated.
        ("to_bytes(x'ff00c0af')", bytes(&[0xff, 0x00, 0xc0, 0xaf])),
        ("to_bytes(to_bytes('hi'))", bytes(b"hi")),
    ]);
}

#[test]
fn to_bytes_converts_runtime_values() {
    let convert = |value: Value| Script::new("to_bytes(v)").capture("v", value).run();
    assert_eq!(
        convert(Value::from("\u{20ac}!")),
        bytes(&[0xe2, 0x82, 0xac, b'!'])
    );
    assert_eq!(convert(bytes(&[b'h', 0xff])), bytes(&[b'h', 0xff]));
}

#[test]
fn to_bytes_rejects_every_type_but_string_and_bytes() {
    let message =
        |ty: &str| format!("Function to_bytes requires String or Bytes as argument 1, got {ty}");
    for (argument, ty) in [
        ("null", "Null"),
        ("1", "Int"),
        ("1.5", "Float"),
        ("true", "Bool"),
        ("[104, 105]", "Array"),
        ("{a: 1}", "Map"),
        ("fn -> 'hi'", "Function"),
    ] {
        let source = format!("to_bytes({argument})");
        assert_eq!(raises(&source), message(ty), "{source:?}");
    }
}

#[test]
fn to_bytes_takes_exactly_one_argument() {
    assert_raises(&[
        (
            "to_bytes()",
            "Function to_bytes expects 1 arguments, but was called with 0",
        ),
        (
            "to_bytes('a', 'b')",
            "Function to_bytes expects 1 arguments, but was called with 2",
        ),
    ]);
}

// --- from_utf8 ---

#[test]
fn from_utf8_decodes_utf8_bytes() {
    assert_results(&[
        ("from_utf8(x'6869')", Value::from("hi")),
        ("from_utf8(x'')", Value::from("")),
        ("from_utf8(x'c3a9')", Value::from("\u{e9}")),
        ("from_utf8(x'e282ac')", Value::from("\u{20ac}")),
        ("from_utf8(x'f09f9880')", Value::from("\u{1f600}")),
        // NUL is valid UTF-8, not a terminator.
        ("from_utf8(x'610062')", Value::from("a\0b")),
    ]);
}

#[test]
fn from_utf8_returns_null_for_bytes_that_are_not_utf8() {
    for source in [
        // A byte that never appears in UTF-8.
        "from_utf8(x'ff')",
        // A continuation byte with no lead byte.
        "from_utf8(x'80')",
        // A three-byte sequence cut short.
        "from_utf8(x'e282')",
        // An overlong encoding of '/'.
        "from_utf8(x'c0af')",
        // An encoded UTF-16 surrogate (U+D800).
        "from_utf8(x'eda080')",
        // Past the last code point (U+110000).
        "from_utf8(x'f4908080')",
        // Valid text followed by an invalid byte: no partial decode.
        "from_utf8(x'6869ff')",
    ] {
        assert_eq!(run(source), Value::Null, "{source:?} is Null");
    }
}

#[test]
fn from_utf8_decodes_runtime_bytes() {
    let decode = |octets: &[u8]| {
        Script::new("from_utf8(b)")
            .capture("b", bytes(octets))
            .run()
    };
    assert_eq!(decode(&[0xe2, 0x82, 0xac, b'!']), Value::from("\u{20ac}!"));
    assert_eq!(decode(&[b'h', 0xff]), Value::Null, "invalid UTF-8 is Null");
}

#[test]
fn from_utf8_inverts_to_bytes() {
    assert_results(&[
        (
            r"from_utf8(to_bytes('caf\u{e9} \u{1f600}'))",
            Value::from("caf\u{e9} \u{1f600}"),
        ),
        ("from_utf8(to_bytes(''))", Value::from("")),
        ("to_bytes(from_utf8(x'e282ac'))", bytes(&[0xe2, 0x82, 0xac])),
    ]);
}

#[test]
fn from_utf8_rejects_every_type_but_bytes() {
    // A String is rejected rather than passed through: it is already decoded.
    let message = |ty: &str| format!("Function from_utf8 requires Bytes as argument 1, got {ty}");
    for (argument, ty) in [
        ("'hi'", "String"),
        ("null", "Null"),
        ("1", "Int"),
        ("1.5", "Float"),
        ("true", "Bool"),
        ("[104, 105]", "Array"),
        ("{a: 1}", "Map"),
        ("fn -> x'6869'", "Function"),
    ] {
        let source = format!("from_utf8({argument})");
        assert_eq!(raises(&source), message(ty), "{source:?}");
    }
}

#[test]
fn from_utf8_takes_exactly_one_argument() {
    assert_raises(&[
        (
            "from_utf8()",
            "Function from_utf8 expects 1 arguments, but was called with 0",
        ),
        (
            "from_utf8(x'68', x'69')",
            "Function from_utf8 expects 1 arguments, but was called with 2",
        ),
    ]);
}
