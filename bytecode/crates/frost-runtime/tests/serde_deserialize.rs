use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use frost_runtime::{FrostArray, FrostFloat, FrostMap, MapKey, Value, from_value, to_value};

fn str_key(s: &str) -> MapKey {
    MapKey::String(Arc::from(s))
}

// ---- Primitives ----

#[test]
fn deserialize_null_to_unit() {
    let v: () = from_value(Value::Null).unwrap();
    assert_eq!(v, ());
}

#[test]
fn deserialize_bool() {
    assert_eq!(from_value::<bool>(Value::from(true)).unwrap(), true);
    assert_eq!(from_value::<bool>(Value::from(false)).unwrap(), false);
}

#[test]
fn deserialize_i64() {
    assert_eq!(from_value::<i64>(Value::from(42i64)).unwrap(), 42);
}

#[test]
fn deserialize_i32() {
    assert_eq!(from_value::<i32>(Value::from(42i64)).unwrap(), 42);
}

#[test]
fn deserialize_f64() {
    let v: Value = 3.14.try_into().unwrap();
    let f: f64 = from_value(v).unwrap();
    assert!((f - 3.14).abs() < 1e-10);
}

#[test]
fn deserialize_string() {
    assert_eq!(from_value::<String>(Value::from("hello")).unwrap(), "hello");
}

// ---- Option ----

#[test]
fn deserialize_none_from_null() {
    let v: Option<i64> = from_value(Value::Null).unwrap();
    assert_eq!(v, None);
}

#[test]
fn deserialize_some_from_value() {
    let v: Option<i64> = from_value(Value::from(42i64)).unwrap();
    assert_eq!(v, Some(42));
}

#[test]
fn deserialize_some_string() {
    let v: Option<String> = from_value(Value::from("hi")).unwrap();
    assert_eq!(v, Some("hi".to_owned()));
}

// ---- Sequences ----

#[test]
fn deserialize_vec() {
    let arr = FrostArray::from(vec![
        Value::from(1i64),
        Value::from(2i64),
        Value::from(3i64),
    ]);
    let v: Vec<i64> = from_value(Value::from(arr)).unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

#[test]
fn deserialize_empty_vec() {
    let arr = FrostArray::from(vec![]);
    let v: Vec<i64> = from_value(Value::from(arr)).unwrap();
    assert!(v.is_empty());
}

#[test]
fn deserialize_nested_vec() {
    let inner1 = FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]);
    let inner2 = FrostArray::from(vec![Value::from(3i64)]);
    let outer = FrostArray::from(vec![Value::from(inner1), Value::from(inner2)]);
    let v: Vec<Vec<i64>> = from_value(Value::from(outer)).unwrap();
    assert_eq!(v, vec![vec![1, 2], vec![3]]);
}

#[test]
fn deserialize_tuple() {
    let arr = FrostArray::from(vec![
        Value::from(1i64),
        Value::from("hi"),
        Value::from(true),
    ]);
    let v: (i64, String, bool) = from_value(Value::from(arr)).unwrap();
    assert_eq!(v, (1, "hi".to_owned(), true));
}

// ---- Maps ----

#[test]
fn deserialize_hashmap() {
    let map: FrostMap = vec![
        (str_key("a"), Value::from(1i64)),
        (str_key("b"), Value::from(2i64)),
    ]
    .into_iter()
    .collect();
    let v: HashMap<String, i64> = from_value(Value::from(map)).unwrap();
    assert_eq!(v.get("a"), Some(&1));
    assert_eq!(v.get("b"), Some(&2));
}

#[test]
fn deserialize_empty_hashmap() {
    let v: HashMap<String, i64> = from_value(Value::from(FrostMap::empty())).unwrap();
    assert!(v.is_empty());
}

// ---- Structs ----

#[derive(Deserialize, Debug, PartialEq)]
struct Simple {
    name: String,
    age: i64,
    active: bool,
}

#[test]
fn deserialize_struct() {
    let map: FrostMap = vec![
        (str_key("name"), Value::from("alice")),
        (str_key("age"), Value::from(30i64)),
        (str_key("active"), Value::from(true)),
    ]
    .into_iter()
    .collect();
    let s: Simple = from_value(Value::from(map)).unwrap();
    assert_eq!(
        s,
        Simple {
            name: "alice".into(),
            age: 30,
            active: true,
        }
    );
}

#[test]
fn deserialize_struct_missing_field_errors() {
    let map: FrostMap = vec![(str_key("name"), Value::from("alice"))]
        .into_iter()
        .collect();
    let r = from_value::<Simple>(Value::from(map));
    assert!(r.is_err());
}

#[derive(Deserialize, Debug, PartialEq)]
struct WithOptional {
    required: i64,
    optional: Option<String>,
}

#[test]
fn deserialize_struct_with_some() {
    let map: FrostMap = vec![
        (str_key("required"), Value::from(1i64)),
        (str_key("optional"), Value::from("yes")),
    ]
    .into_iter()
    .collect();
    let s: WithOptional = from_value(Value::from(map)).unwrap();
    assert_eq!(
        s,
        WithOptional {
            required: 1,
            optional: Some("yes".into()),
        }
    );
}

#[test]
fn deserialize_struct_with_null_option() {
    let map: FrostMap = vec![
        (str_key("required"), Value::from(1i64)),
        (str_key("optional"), Value::Null),
    ]
    .into_iter()
    .collect();
    let s: WithOptional = from_value(Value::from(map)).unwrap();
    assert_eq!(
        s,
        WithOptional {
            required: 1,
            optional: None,
        }
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Nested {
    label: String,
    items: Vec<i64>,
}

#[test]
fn deserialize_struct_with_nested() {
    let items = FrostArray::from(vec![Value::from(1i64), Value::from(2i64)]);
    let map: FrostMap = vec![
        (str_key("label"), Value::from("test")),
        (str_key("items"), Value::from(items)),
    ]
    .into_iter()
    .collect();
    let s: Nested = from_value(Value::from(map)).unwrap();
    assert_eq!(
        s,
        Nested {
            label: "test".into(),
            items: vec![1, 2],
        }
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct WithDefault {
    name: String,
    #[serde(default)]
    count: i64,
}

#[test]
fn deserialize_struct_with_default() {
    let map: FrostMap = vec![(str_key("name"), Value::from("hello"))]
        .into_iter()
        .collect();
    let s: WithDefault = from_value(Value::from(map)).unwrap();
    assert_eq!(
        s,
        WithDefault {
            name: "hello".into(),
            count: 0,
        }
    );
}

// ---- Enums ----

#[derive(Deserialize, Debug, PartialEq)]
enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn deserialize_unit_enum() {
    let v: Color = from_value(Value::from("Red")).unwrap();
    assert_eq!(v, Color::Red);
}

#[test]
fn deserialize_unit_enum_invalid_variant() {
    let r = from_value::<Color>(Value::from("Purple"));
    assert!(r.is_err());
}

#[derive(Deserialize, Debug, PartialEq)]
enum Shape {
    Circle(f64),
    Rectangle { width: f64, height: f64 },
    Point,
}

#[test]
fn deserialize_newtype_variant() {
    let map: FrostMap = vec![(
        str_key("Circle"),
        Value::Float(FrostFloat::new(5.0).unwrap()),
    )]
    .into_iter()
    .collect();
    let v: Shape = from_value(Value::from(map)).unwrap();
    assert_eq!(v, Shape::Circle(5.0));
}

#[test]
fn deserialize_struct_variant() {
    let fields: FrostMap = vec![
        (
            str_key("width"),
            Value::Float(FrostFloat::new(10.0).unwrap()),
        ),
        (
            str_key("height"),
            Value::Float(FrostFloat::new(20.0).unwrap()),
        ),
    ]
    .into_iter()
    .collect();
    let map: FrostMap = vec![(str_key("Rectangle"), Value::from(fields))]
        .into_iter()
        .collect();
    let v: Shape = from_value(Value::from(map)).unwrap();
    assert_eq!(
        v,
        Shape::Rectangle {
            width: 10.0,
            height: 20.0,
        }
    );
}

#[test]
fn deserialize_unit_variant_via_string() {
    let v: Shape = from_value(Value::from("Point")).unwrap();
    assert_eq!(v, Shape::Point);
}

// ---- Value round-trip ----

#[test]
fn deserialize_value_from_value() {
    let original = Value::from(42i64);
    let v: Value = from_value(original.clone()).unwrap();
    assert_eq!(v, original);
}

#[test]
fn deserialize_value_array() {
    let arr = Value::from(FrostArray::from(vec![Value::from(1i64), Value::from("hi")]));
    let v: Value = from_value(arr.clone()).unwrap();
    assert_eq!(v, arr);
}

#[test]
fn deserialize_value_map() {
    let map: FrostMap = vec![(str_key("a"), Value::from(1i64))]
        .into_iter()
        .collect();
    let original = Value::from(map);
    let v: Value = from_value(original.clone()).unwrap();
    assert_eq!(v, original);
}

// ---- Type mismatches ----

#[test]
fn deserialize_int_from_string_errors() {
    let r = from_value::<i64>(Value::from("hello"));
    assert!(r.is_err());
}

#[test]
fn deserialize_string_from_int_errors() {
    let r = from_value::<String>(Value::from(42i64));
    assert!(r.is_err());
}

#[test]
fn deserialize_vec_from_map_errors() {
    let r = from_value::<Vec<i64>>(Value::from(FrostMap::empty()));
    assert!(r.is_err());
}

#[test]
fn deserialize_struct_from_array_errors() {
    let arr = FrostArray::from(vec![Value::from(1i64)]);
    let r = from_value::<Simple>(Value::from(arr));
    assert!(r.is_err());
}

// ---- Newtype struct ----

#[derive(Deserialize, Debug, PartialEq)]
struct Wrapper(i64);

#[test]
fn deserialize_newtype_struct() {
    let v: Wrapper = from_value(Value::from(42i64)).unwrap();
    assert_eq!(v, Wrapper(42));
}

// ---- Tuple variant ----

#[derive(Deserialize, Debug, PartialEq)]
enum Expr {
    Pair(i64, String),
}

#[test]
fn deserialize_tuple_variant() {
    let inner = FrostArray::from(vec![Value::from(1i64), Value::from("hello")]);
    let map: FrostMap = vec![(str_key("Pair"), Value::from(inner))]
        .into_iter()
        .collect();
    let v: Expr = from_value(Value::from(map)).unwrap();
    assert_eq!(v, Expr::Pair(1, "hello".into()));
}

// ---- Nested struct ----

#[derive(Deserialize, Debug, PartialEq)]
struct Address {
    city: String,
    zip: i64,
}

#[derive(Deserialize, Debug, PartialEq)]
struct Person {
    name: String,
    address: Address,
}

#[test]
fn deserialize_nested_struct() {
    let address: FrostMap = vec![
        (str_key("city"), Value::from("Portland")),
        (str_key("zip"), Value::from(97201i64)),
    ]
    .into_iter()
    .collect();
    let person: FrostMap = vec![
        (str_key("name"), Value::from("alice")),
        (str_key("address"), Value::from(address)),
    ]
    .into_iter()
    .collect();
    let p: Person = from_value(Value::from(person)).unwrap();
    assert_eq!(
        p,
        Person {
            name: "alice".into(),
            address: Address {
                city: "Portland".into(),
                zip: 97201,
            },
        }
    );
}

// ---- Round-trip: serialize then deserialize ----

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Config {
    name: String,
    count: i64,
    enabled: bool,
    tags: Vec<String>,
    threshold: Option<f64>,
}

#[test]
fn round_trip_struct() {
    let original = Config {
        name: "test".into(),
        count: 42,
        enabled: true,
        tags: vec!["a".into(), "b".into()],
        threshold: Some(0.5),
    };
    let value = to_value(&original).unwrap();
    let recovered: Config = from_value(value).unwrap();
    assert_eq!(original, recovered);
}

#[test]
fn round_trip_struct_with_none() {
    let original = Config {
        name: "minimal".into(),
        count: 0,
        enabled: false,
        tags: vec![],
        threshold: None,
    };
    let value = to_value(&original).unwrap();
    let recovered: Config = from_value(value).unwrap();
    assert_eq!(original, recovered);
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Outer {
    label: String,
    inner: Inner,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Inner {
    x: i64,
    y: i64,
}

#[test]
fn round_trip_nested_struct() {
    let original = Outer {
        label: "origin".into(),
        inner: Inner { x: 0, y: 0 },
    };
    let value = to_value(&original).unwrap();
    let recovered: Outer = from_value(value).unwrap();
    assert_eq!(original, recovered);
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Action {
    Click,
    Move(i64, i64),
    Resize { width: i64, height: i64 },
}

#[test]
fn round_trip_unit_enum() {
    let original = Action::Click;
    let value = to_value(&original).unwrap();
    let recovered: Action = from_value(value).unwrap();
    assert_eq!(original, recovered);
}

#[test]
fn round_trip_tuple_enum() {
    let original = Action::Move(10, 20);
    let value = to_value(&original).unwrap();
    let recovered: Action = from_value(value).unwrap();
    assert_eq!(original, recovered);
}

#[test]
fn round_trip_struct_enum() {
    let original = Action::Resize {
        width: 100,
        height: 200,
    };
    let value = to_value(&original).unwrap();
    let recovered: Action = from_value(value).unwrap();
    assert_eq!(original, recovered);
}

#[test]
fn round_trip_vec_of_structs() {
    let original = vec![Inner { x: 1, y: 2 }, Inner { x: 3, y: 4 }];
    let value = to_value(&original).unwrap();
    let recovered: Vec<Inner> = from_value(value).unwrap();
    assert_eq!(original, recovered);
}

// ---- Bytes vs sequence ----
// The mirror of the serialize side: the Rust target declares which it wants, and
// the wrong Frost type is an error rather than a silent conversion.

/// A type that deserializes through `deserialize_bytes`, as `serde_bytes` would.
#[derive(Debug, PartialEq)]
struct RawBytes(Vec<u8>);

impl Serialize for RawBytes {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for RawBytes {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct BytesVisitor;

        impl<'de> serde::de::Visitor<'de> for BytesVisitor {
            type Value = RawBytes;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("bytes")
            }

            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<RawBytes, E> {
                Ok(RawBytes(v.to_vec()))
            }
        }

        d.deserialize_bytes(BytesVisitor)
    }
}

#[test]
fn deserialize_bytes_from_a_bytes_value() {
    let v: RawBytes = from_value(Value::from(vec![0xff, 0x00])).unwrap();
    assert_eq!(v, RawBytes(vec![0xff, 0x00]));
}

#[test]
fn deserialize_bytes_rejects_a_string() {
    // Text is not binary, even when its bytes would decode.
    let err = from_value::<RawBytes>(Value::from("hi")).unwrap_err();
    assert!(
        err.message().contains("expected Bytes, got String"),
        "{}",
        err.message()
    );
}

#[test]
fn deserialize_string_rejects_bytes() {
    // The other direction: valid UTF-8 in a Bytes is still not text.
    let err = from_value::<String>(Value::from(vec![b'h', b'i'])).unwrap_err();
    assert!(
        err.message().contains("expected String, got Bytes"),
        "{}",
        err.message()
    );
}

#[test]
fn a_plain_vec_u8_comes_from_an_array() {
    // `Vec<u8>` asks for a sequence, so an Array serves it and a Bytes does not.
    let v: Vec<u8> = from_value(Value::array([1i64, 2i64])).unwrap();
    assert_eq!(v, vec![1u8, 2]);

    let err = from_value::<Vec<u8>>(Value::from(vec![1u8, 2])).unwrap_err();
    assert!(err.message().contains("expected Array"), "{}", err.message());
}

#[test]
fn a_bytes_key_cannot_name_a_struct_field() {
    // serde's derived identifier visitor matches field names by bytes as well as by
    // text, so without an explicit guard a binary key would select a text-named field.
    let map: FrostMap = vec![(MapKey::from(b"x".to_vec()), Value::from(1i64))]
        .into_iter()
        .collect();
    let err = from_value::<Inner>(Value::from(map)).unwrap_err();
    assert!(
        err.message().contains("expected String, got Bytes"),
        "{}",
        err.message()
    );
}

#[test]
fn a_string_key_still_names_a_struct_field() {
    let map: FrostMap = vec![
        (str_key("x"), Value::from(1i64)),
        (str_key("y"), Value::from(2i64)),
    ]
    .into_iter()
    .collect();
    let v: Inner = from_value(Value::from(map)).unwrap();
    assert_eq!(v, Inner { x: 1, y: 2 });
}

#[test]
fn deserialize_value_from_bytes_round_trips() {
    let original = Value::from(vec![0xff, 0x00]);
    let v: Value = from_value(original.clone()).unwrap();
    assert_eq!(v, original);
    assert!(v.is_bytes());
}

#[test]
fn bytes_and_array_targets_round_trip_independently() {
    // Whichever the Rust type declares, it survives a trip through Value and back:
    // the annotated field stays binary, the plain one stays a sequence.
    let bytes = from_value::<RawBytes>(to_value(&RawBytes(vec![1, 2])).unwrap()).unwrap();
    assert_eq!(bytes, RawBytes(vec![1, 2]));

    let seq = from_value::<Vec<u8>>(to_value(&vec![1u8, 2]).unwrap()).unwrap();
    assert_eq!(seq, vec![1u8, 2]);
}
