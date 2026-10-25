//! Serializing Rust types into values with `to_value`. `Option` and
//! `Nullable` are tested in `serde_option_nullable.rs`.

use crate::common;

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use frostlang::bytecode::Bytecode;
use frostlang::{FrostArray, FrostFloat, FrostMap, MapKey, Value, frost_closure, to_value};

fn str_key(s: &str) -> MapKey {
    MapKey::from(s)
}

// ---- Primitives ----

#[test]
fn serialize_bool() {
    assert_eq!(to_value(&true).unwrap(), Value::from(true));
    assert_eq!(to_value(&false).unwrap(), Value::from(false));
}

#[test]
fn serialize_i8() {
    assert_eq!(to_value(&42i8).unwrap(), Value::from(42i64));
}

#[test]
fn serialize_i16() {
    assert_eq!(to_value(&42i16).unwrap(), Value::from(42i64));
}

#[test]
fn serialize_i32() {
    assert_eq!(to_value(&42i32).unwrap(), Value::from(42i64));
}

#[test]
fn serialize_i64() {
    assert_eq!(to_value(&42i64).unwrap(), Value::from(42i64));
}

#[test]
fn serialize_u8() {
    assert_eq!(to_value(&42u8).unwrap(), Value::from(42i64));
}

#[test]
fn serialize_u16() {
    assert_eq!(to_value(&42u16).unwrap(), Value::from(42i64));
}

#[test]
fn serialize_u32() {
    assert_eq!(to_value(&42u32).unwrap(), Value::from(42i64));
}

#[test]
fn serialize_u64_in_range() {
    assert_eq!(to_value(&42u64).unwrap(), Value::from(42i64));
}

#[test]
fn serialize_i128_and_u128_in_the_int_range() {
    for int in [i64::MIN, -1, 0, i64::MAX] {
        assert_eq!(to_value(&i128::from(int)).unwrap(), Value::Int(int));
    }
    assert_eq!(to_value(&0u128).unwrap(), Value::Int(0));
    assert_eq!(to_value(&(i64::MAX as u128)).unwrap(), Value::Int(i64::MAX));
}

#[test]
fn serialize_integer_out_of_int_range() {
    let cases = [
        (to_value(&(i64::MAX as u64 + 1)), "9223372036854775808"),
        (to_value(&u64::MAX), "18446744073709551615"),
        (
            to_value(&(i128::from(i64::MIN) - 1)),
            "-9223372036854775809",
        ),
        (to_value(&(i128::from(i64::MAX) + 1)), "9223372036854775808"),
        (
            to_value(&u128::MAX),
            "340282366920938463463374607431768211455",
        ),
    ];
    for (result, int) in cases {
        assert_eq!(
            result.expect_err("refused").message(),
            format!("integer {int} is beyond the Int range")
        );
    }
}

#[test]
fn serialize_f32() {
    assert_eq!(
        to_value(&1.5f32).unwrap(),
        Value::Float(FrostFloat::new(1.5).unwrap())
    );
}

#[test]
fn serialize_f64() {
    assert_eq!(
        to_value(&3.14f64).unwrap(),
        Value::Float(FrostFloat::new(3.14).unwrap())
    );
}

#[test]
fn serialize_str() {
    assert_eq!(to_value(&"hello").unwrap(), Value::from("hello"));
}

#[test]
fn serialize_string() {
    assert_eq!(
        to_value(&String::from("hello")).unwrap(),
        Value::from("hello")
    );
}

#[test]
fn serialize_char() {
    assert_eq!(to_value(&'a').unwrap(), Value::from("a"));
    assert_eq!(to_value(&'\u{1F600}').unwrap(), Value::from("\u{1F600}"));
}

// ---- Unit ----

#[test]
fn serialize_unit() {
    assert_eq!(to_value(&()).unwrap(), Value::Null);
}

// ---- Sequences ----

#[test]
fn serialize_vec() {
    let v = to_value(&vec![1i64, 2, 3]).unwrap();
    let expected = Value::from(FrostArray::from(vec![
        Value::from(1i64),
        Value::from(2i64),
        Value::from(3i64),
    ]));
    assert_eq!(v, expected);
}

#[test]
fn serialize_empty_vec() {
    let v = to_value(&Vec::<i32>::new()).unwrap();
    assert_eq!(v, Value::from(FrostArray::from(vec![])));
}

#[test]
fn serialize_nested_vec() {
    let v = to_value(&vec![vec![1i64, 2], vec![3]]).unwrap();
    let inner1 = FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]);
    let inner2 = FrostArray::from(vec![Value::from(3i64)]);
    let expected = Value::from(FrostArray::from(vec![
        Value::from(inner1),
        Value::from(inner2),
    ]));
    assert_eq!(v, expected);
}

#[test]
fn serialize_tuple() {
    let v = to_value(&(1i64, "hello", true)).unwrap();
    let expected = Value::from(FrostArray::from(vec![
        Value::from(1i64),
        Value::from("hello"),
        Value::from(true),
    ]));
    assert_eq!(v, expected);
}

// ---- Maps ----

#[test]
fn serialize_hashmap_string_keys() {
    let mut m = HashMap::new();
    m.insert("key", 42i64);
    let v = to_value(&m).unwrap();
    assert!(matches!(v, Value::Map(_)));
    if let Value::Map(map) = v {
        assert_eq!(map.get_str("key"), Some(&Value::from(42i64)));
    }
}

#[test]
fn serialize_empty_hashmap() {
    let m: HashMap<String, i32> = HashMap::new();
    let v = to_value(&m).unwrap();
    assert_eq!(v, Value::from(FrostMap::empty()));
}

// ---- Structs ----

#[derive(Serialize)]
struct Simple {
    name: String,
    age: i64,
    active: bool,
}

#[test]
fn serialize_struct() {
    let s = Simple {
        name: "alice".into(),
        age: 30,
        active: true,
    };
    let v = to_value(&s).unwrap();
    assert!(matches!(v, Value::Map(_)));
    if let Value::Map(map) = v {
        assert_eq!(map.get_str("name"), Some(&Value::from("alice")));
        assert_eq!(map.get_str("age"), Some(&Value::from(30i64)));
        assert_eq!(map.get_str("active"), Some(&Value::from(true)));
    }
}

#[derive(Serialize)]
struct Nested {
    label: String,
    items: Vec<i64>,
}

#[test]
fn serialize_struct_with_nested() {
    let s = Nested {
        label: "test".into(),
        items: vec![1, 2, 3],
    };
    let v = to_value(&s).unwrap();
    if let Value::Map(map) = v {
        assert_eq!(map.get_str("label"), Some(&Value::from("test")));
        let items = map.get_str("items").unwrap();
        assert!(matches!(items, Value::Array(_)));
    } else {
        panic!("expected Map");
    }
}

#[derive(Serialize)]
struct WithOptional {
    required: i64,
    optional: Option<String>,
}

#[test]
fn serialize_struct_with_some() {
    let s = WithOptional {
        required: 1,
        optional: Some("yes".into()),
    };
    let v = to_value(&s).unwrap();
    if let Value::Map(map) = v {
        assert_eq!(map.get_str("optional"), Some(&Value::from("yes")));
    } else {
        panic!("expected Map");
    }
}

// ---- Enums ----

#[derive(Serialize)]
enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn serialize_unit_enum() {
    assert_eq!(to_value(&Color::Red).unwrap(), Value::from("Red"));
    assert_eq!(to_value(&Color::Green).unwrap(), Value::from("Green"));
    assert_eq!(to_value(&Color::Blue).unwrap(), Value::from("Blue"));
}

#[derive(Serialize)]
enum Shape {
    Circle(f64),
    Rectangle { width: f64, height: f64 },
    Point,
}

#[test]
fn serialize_newtype_variant() {
    let v = to_value(&Shape::Circle(5.0)).unwrap();
    if let Value::Map(map) = v {
        assert!(map.get_str("Circle").is_some());
    } else {
        panic!("expected Map");
    }
}

#[test]
fn serialize_struct_variant() {
    let v = to_value(&Shape::Rectangle {
        width: 10.0,
        height: 20.0,
    })
    .unwrap();
    if let Value::Map(outer) = v {
        let inner = outer.get_str("Rectangle").unwrap();
        if let Value::Map(fields) = inner {
            assert!(fields.get_str("width").is_some());
            assert!(fields.get_str("height").is_some());
        } else {
            panic!("expected inner Map");
        }
    } else {
        panic!("expected outer Map");
    }
}

#[test]
fn serialize_unit_variant_in_enum() {
    assert_eq!(to_value(&Shape::Point).unwrap(), Value::from("Point"));
}

// ---- Value itself serializes ----

#[test]
fn serialize_value_null() {
    let v = to_value(&Value::Null).unwrap();
    assert_eq!(v, Value::Null);
}

#[test]
fn serialize_value_int() {
    let v = to_value(&Value::from(42i64)).unwrap();
    assert_eq!(v, Value::from(42i64));
}

#[test]
fn serialize_value_array() {
    let arr = Value::from(FrostArray::from(vec![Value::from(1i64)]));
    let v = to_value(&arr).unwrap();
    assert_eq!(v, arr);
}

#[test]
fn serialize_value_map() {
    let map: FrostMap = vec![(str_key("a"), Value::from(1i64))]
        .into_iter()
        .collect();
    let original = Value::from(map);
    let v = to_value(&original).unwrap();
    assert_eq!(v, original);
}

// ---- Errors ----

/// The error message of serializing `value`, which must fail.
fn error_of<T: Serialize>(value: &T) -> String {
    to_value(value)
        .expect_err("serializing fails")
        .message()
        .into_owned()
}

#[derive(Serialize)]
struct Reading {
    level: f64,
}

#[derive(Serialize)]
enum Event {
    Newtype(f64),
    Tuple(i64, f64),
    Fields { level: f64 },
}

#[test]
fn an_error_at_the_top_level_names_no_place() {
    assert_eq!(error_of(&f64::NAN), "NaN is not finite");
}

#[test]
fn an_error_names_where_in_the_value_it_happened() {
    // Written as Frost would index the value, as `from_value` writes it.
    #[derive(Serialize)]
    struct Pair(f64, f64);

    let cases = [
        (
            "a struct field",
            error_of(&Reading { level: f64::NAN }),
            "level",
        ),
        ("an element", error_of(&vec![1.0, f64::NAN]), "[1]"),
        ("a tuple's element", error_of(&(1.0, f64::NAN)), "[1]"),
        (
            "a tuple struct's field",
            error_of(&Pair(1.0, f64::NAN)),
            "[1]",
        ),
        (
            "an entry not keyed by a name",
            error_of(&HashMap::from([("my-key", f64::NAN)])),
            r#"["my-key"]"#,
        ),
        (
            "an entry keyed by an Int",
            error_of(&HashMap::from([(7, f64::NAN)])),
            "[7]",
        ),
        (
            "a newtype variant",
            error_of(&Event::Newtype(f64::NAN)),
            "Newtype",
        ),
        (
            "a tuple variant",
            error_of(&Event::Tuple(1, f64::NAN)),
            "Tuple[1]",
        ),
        (
            "a struct variant",
            error_of(&Event::Fields { level: f64::NAN }),
            "Fields.level",
        ),
        (
            "nested",
            error_of(&HashMap::from([(
                "readings",
                vec![Reading { level: 1.0 }, Reading { level: f64::NAN }],
            )])),
            "readings[1].level",
        ),
    ];
    for (case, message, place) in cases {
        assert_eq!(
            message,
            format!("NaN is not finite (at `{place}`)"),
            "{case}"
        );
    }
}

#[test]
fn an_error_names_its_place_in_the_value_serde_writes() {
    // The place follows the value's shape, which serde's attributes decide,
    // rather than the Rust type's.
    #[derive(Serialize)]
    #[serde(tag = "type")]
    enum Tagged {
        Fields { level: f64 },
    }
    #[derive(Serialize)]
    #[serde(tag = "type", content = "content")]
    enum Adjacent {
        Fields { level: f64 },
    }
    #[derive(Serialize)]
    enum Renamed {
        #[serde(rename = "my-variant")]
        Fields { level: f64 },
    }
    let cases = [
        (
            "an internally tagged variant's field",
            error_of(&Tagged::Fields { level: f64::NAN }),
            "level",
        ),
        (
            "an adjacently tagged variant's field",
            error_of(&Adjacent::Fields { level: f64::NAN }),
            "content.level",
        ),
        (
            "a variant whose name is not identifier-like",
            error_of(&Renamed::Fields { level: f64::NAN }),
            r#"["my-variant"].level"#,
        ),
    ];
    for (case, message, place) in cases {
        assert_eq!(
            message,
            format!("NaN is not finite (at `{place}`)"),
            "{case}"
        );
    }
}

#[test]
fn an_error_in_a_flattened_field_names_the_field() {
    // serde writes a flattened field's entries into the enclosing Map.
    #[derive(Serialize)]
    struct Outer {
        #[serde(flatten)]
        inner: Reading,
    }
    assert_eq!(
        error_of(&Outer {
            inner: Reading { level: f64::NAN }
        }),
        "NaN is not finite (at `level`)",
        "the flattened struct adds no level of its own"
    );
}

#[test]
fn an_error_in_a_key_says_so() {
    let unkeyable = || BTreeMap::from([(vec![1], 2)]);
    assert_eq!(
        error_of(&unkeyable()),
        "Type Array is not a valid Map key (at a key)",
        "at the top level"
    );
    assert_eq!(
        error_of(&HashMap::from([("outer", unkeyable())])),
        "Type Array is not a valid Map key (at a key of `outer`)",
        "nested, with the path"
    );
    // No Frost index reaches inside a key, so the place within one goes unsaid.
    assert_eq!(
        error_of(&HashMap::from([(
            "outer",
            BTreeMap::from([((1, u64::MAX), 1)])
        )])),
        "integer 18446744073709551615 is beyond the Int range (at a key of `outer`)",
        "an error inside a key"
    );
}

/// Both kinds of Function value refuse a foreign serializer.
#[test]
fn serialize_function_errors() {
    let native = frost_closure!("f", || Value::Null);
    let closure = Value::Closure(common::closure(vec![Bytecode::PushNull], vec![]));

    for (kind, function) in [("native", native), ("closure", closure)] {
        let err = serde_json::to_string(&function).unwrap_err();
        assert!(
            err.to_string().contains("cannot serialize Function"),
            "{kind}: got: {err}"
        );
    }
}

#[test]
fn serialize_opaque_errors() {
    #[derive(Debug)]
    struct Blob;

    impl frostlang::FrostOpaque for Blob {
        fn type_name() -> &'static str {
            "Blob"
        }

        fn try_to_string(&self) -> Option<String> {
            None
        }
    }

    let err = serde_json::to_string(&Value::opaque(Blob)).unwrap_err();
    assert!(
        err.to_string().contains("cannot serialize Opaque"),
        "got: {err}"
    );
}

// ---- Newtype struct ----

#[derive(Serialize)]
struct Wrapper(i64);

#[test]
fn serialize_newtype_struct() {
    assert_eq!(to_value(&Wrapper(42)).unwrap(), Value::from(42i64));
}

// ---- Tuple variant ----

#[derive(Serialize)]
enum Expr {
    Pair(i64, String),
}

#[test]
fn serialize_tuple_variant() {
    let v = to_value(&Expr::Pair(1, "hello".into())).unwrap();
    if let Value::Map(outer) = v {
        let inner = outer.get_str("Pair").unwrap();
        let expected = Value::from(FrostArray::from(vec![
            Value::from(1i64),
            Value::from("hello"),
        ]));
        assert_eq!(inner, &expected);
    } else {
        panic!("expected Map");
    }
}

// ---- Bytes vs sequence ----
// serde's own convention decides: `serialize_bytes` yields Bytes, `serialize_seq`
// yields an Array. The Rust type declares which, so there is nothing to infer.

/// A type that serializes through `serialize_bytes`, as `serde_bytes` would.
struct RawBytes(Vec<u8>);

impl Serialize for RawBytes {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.0)
    }
}

#[test]
fn serialize_bytes_produces_bytes() {
    let v = to_value(&RawBytes(vec![0xff, 0x00])).unwrap();
    assert_eq!(v, Value::from(vec![0xff, 0x00]));
    assert!(v.is_bytes());
}

#[test]
fn a_plain_vec_u8_produces_an_array_of_ints() {
    // serde serializes `Vec<u8>` as a sequence, so it lands as an Array. This is
    // the distinction that a byte-string type could not express.
    let v = to_value(&vec![1u8, 2]).unwrap();
    assert!(v.is_array());
    assert_eq!(v, Value::array([1i64, 2i64]));
}

#[test]
fn bytes_and_a_byte_array_are_not_interchangeable() {
    let as_bytes = to_value(&RawBytes(vec![1, 2])).unwrap();
    let as_array = to_value(&vec![1u8, 2]).unwrap();
    assert_ne!(as_bytes, as_array);
}
