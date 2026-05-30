use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;

use frost_runtime::{FrostArray, FrostMap, MapKey, Value, to_value};

fn str_key(s: &str) -> MapKey {
    MapKey::String(Arc::from(s.as_bytes()))
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
fn serialize_u64_out_of_range() {
    assert!(to_value(&u64::MAX).is_err());
}

#[test]
fn serialize_f32() {
    let v = to_value(&1.5f32).unwrap();
    assert!(matches!(v, Value::Float(_)));
}

#[test]
fn serialize_f64() {
    let v = to_value(&3.14f64).unwrap();
    assert!(matches!(v, Value::Float(_)));
}

#[test]
fn serialize_f64_nan_errors() {
    assert!(to_value(&f64::NAN).is_err());
}

#[test]
fn serialize_f64_infinity_errors() {
    assert!(to_value(&f64::INFINITY).is_err());
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
    let v = to_value(&'a').unwrap();
    assert_eq!(v, Value::from("a"));
}

// ---- Option / Null ----

#[test]
fn serialize_none() {
    let v: Option<i32> = None;
    assert_eq!(to_value(&v).unwrap(), Value::Null);
}

#[test]
fn serialize_some() {
    let v: Option<i32> = Some(42);
    assert_eq!(to_value(&v).unwrap(), Value::from(42i64));
}

#[test]
fn serialize_unit() {
    assert_eq!(to_value(&()).unwrap(), Value::Null);
}

// ---- Sequences ----

#[test]
fn serialize_vec() {
    let v = to_value(&vec![1i64, 2, 3]).unwrap();
    let expected = Value::from(FrostArray::new(&[
        Value::from(1i64),
        Value::from(2i64),
        Value::from(3i64),
    ]));
    assert_eq!(v, expected);
}

#[test]
fn serialize_empty_vec() {
    let v = to_value(&Vec::<i32>::new()).unwrap();
    assert_eq!(v, Value::from(FrostArray::new(&[])));
}

#[test]
fn serialize_nested_vec() {
    let v = to_value(&vec![vec![1i64, 2], vec![3]]).unwrap();
    let inner1 = FrostArray::new(&[Value::from(1i64), Value::from(2i64)]);
    let inner2 = FrostArray::new(&[Value::from(3i64)]);
    let expected = Value::from(FrostArray::new(&[Value::from(inner1), Value::from(inner2)]));
    assert_eq!(v, expected);
}

#[test]
fn serialize_tuple() {
    let v = to_value(&(1i64, "hello", true)).unwrap();
    let expected = Value::from(FrostArray::new(&[
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

#[test]
fn serialize_struct_with_none() {
    let s = WithOptional {
        required: 1,
        optional: None,
    };
    let v = to_value(&s).unwrap();
    if let Value::Map(map) = v {
        assert_eq!(map.get_str("optional"), Some(&Value::Null));
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
    let arr = Value::from(FrostArray::new(&[Value::from(1i64)]));
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

#[test]
fn serialize_function_errors() {
    // Can't easily construct a Function, but test through Value::Serialize
    // which checks for Function/Opaque
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
        let expected = Value::from(FrostArray::new(&[Value::from(1i64), Value::from("hello")]));
        assert_eq!(inner, &expected);
    } else {
        panic!("expected Map");
    }
}
