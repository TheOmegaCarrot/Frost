//! `Option` and `Nullable` through the serde bridge, per `Nullable`'s docs.

use std::collections::HashMap;
use std::fmt::Debug;

use serde::de::value::{Error as ValueError, I64Deserializer, UnitDeserializer};
use serde::de::{DeserializeOwned, IntoDeserializer};
use serde::{Deserialize, Serialize};

use frostlang::{Nullable, Value, from_value, to_value};

/// The error for a `None` outside a struct field or Map entry.
const NONE_OUTSIDE_AN_ENTRY: &str =
    "`None` can only leave out a struct field or Map entry; for Null, use `Nullable`";

/// The error message of serializing `value`, which must fail.
fn serialize_error<T: Serialize>(value: &T) -> String {
    to_value(value)
        .expect_err("serializing fails")
        .message()
        .into_owned()
}

/// The error message of deserializing `value` as `T`, which must fail.
fn deserialize_error<T: DeserializeOwned + Debug>(value: Value) -> String {
    from_value::<T>(value)
        .expect_err("deserializing fails")
        .message()
        .into_owned()
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct WithOption {
    a: Option<i64>,
}

/// A field that tells absent, Null, and a value apart.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct WithOptionalNullable {
    a: Option<Nullable<i64>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct WithNullable {
    a: Nullable<i64>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Variants {
    Fields { a: Option<i64> },
    Tuple(i64, Option<i64>),
    Newtype(Option<i64>),
}

// ---- Deserializing an Option ----

#[test]
fn an_option_holds_a_present_value() {
    assert_eq!(from_value::<Option<i64>>(Value::Int(5)).unwrap(), Some(5));
    assert_eq!(
        from_value::<Option<String>>(Value::from("hi")).unwrap(),
        Some("hi".to_string())
    );
}

#[test]
fn an_option_gives_null_to_what_it_holds() {
    assert_eq!(
        deserialize_error::<Option<i64>>(Value::Null),
        "expected Int, got Null"
    );
    assert_eq!(
        from_value::<Option<Value>>(Value::Null).unwrap(),
        Some(Value::Null),
        "a Value takes Null"
    );
    assert_eq!(
        deserialize_error::<Option<Option<i64>>>(Value::Null),
        "expected Int, got Null",
        "an inner Option passes Null on in turn"
    );
    assert_eq!(
        from_value::<Option<Option<i64>>>(Value::Int(5)).unwrap(),
        Some(Some(5))
    );
}

#[test]
fn an_absent_field_is_none_and_a_null_one_is_present() {
    assert_eq!(
        from_value::<WithOption>(Value::map::<&str, 0>([])).unwrap(),
        WithOption { a: None }
    );
    assert_eq!(
        deserialize_error::<WithOption>(Value::map([("a", Value::Null)])),
        "expected Int, got Null (at `a`)"
    );
}

#[test]
fn a_struct_variant_field_follows_the_field_rule() {
    assert_eq!(
        from_value::<Variants>(Value::map([("Fields", Value::map::<&str, 0>([]))])).unwrap(),
        Variants::Fields { a: None }
    );
    assert_eq!(
        deserialize_error::<Variants>(Value::map([("Fields", Value::map([("a", Value::Null)]))])),
        "expected Int, got Null (at `Fields.a`)"
    );
}

#[test]
fn elements_and_entries_take_null_only_as_nullable() {
    let array = Value::array([Value::Int(1), Value::Null]);
    assert_eq!(
        from_value::<Vec<Nullable<i64>>>(array.clone()).unwrap(),
        vec![Nullable(Some(1)), Nullable(None)]
    );
    assert_eq!(
        deserialize_error::<Vec<Option<i64>>>(array),
        "expected Int, got Null (at `[1]`)"
    );
    let map = Value::map([("a", Value::Null)]);
    assert_eq!(
        from_value::<HashMap<String, Nullable<i64>>>(map.clone()).unwrap(),
        HashMap::from([("a".to_string(), Nullable(None))])
    );
    assert_eq!(
        deserialize_error::<HashMap<String, Option<i64>>>(map),
        "expected Int, got Null (at `a`)"
    );
}

// ---- Deserializing a Nullable ----

#[test]
fn a_nullable_takes_null_as_none() {
    assert_eq!(
        from_value::<Nullable<i64>>(Value::Null).unwrap(),
        Nullable(None)
    );
    assert_eq!(
        from_value::<Nullable<i64>>(Value::Int(5)).unwrap(),
        Nullable(Some(5))
    );
    assert_eq!(
        from_value::<Nullable<Value>>(Value::Null).unwrap(),
        Nullable(None),
        "even of a Value, which would take Null itself"
    );
    assert_eq!(
        deserialize_error::<Nullable<i64>>(Value::from("5")),
        "expected Int, got String"
    );
}

#[test]
fn an_optional_nullable_field_tells_absent_null_and_a_value_apart() {
    let read = |map: Value| from_value::<WithOptionalNullable>(map).unwrap().a;
    assert_eq!(read(Value::map::<&str, 0>([])), None);
    assert_eq!(read(Value::map([("a", Value::Null)])), Some(Nullable(None)));
    assert_eq!(
        read(Value::map([("a", Value::Int(5))])),
        Some(Nullable(Some(5)))
    );
}

#[test]
fn a_nullable_field_is_required() {
    assert_eq!(
        deserialize_error::<WithNullable>(Value::map::<&str, 0>([])),
        "missing field `a`"
    );
}

// ---- Serializing an Option ----

#[test]
fn a_none_entry_is_left_out() {
    assert_eq!(
        to_value(&WithOption { a: None }).unwrap(),
        Value::map::<&str, 0>([])
    );
    assert_eq!(
        to_value(&Variants::Fields { a: None }).unwrap(),
        Value::map([("Fields", Value::map::<&str, 0>([]))]),
        "a struct variant's field too"
    );
    assert_eq!(
        to_value(&HashMap::from([("a", Some(1)), ("b", None)])).unwrap(),
        Value::map([("a", Value::Int(1))]),
        "and a Map's entry"
    );
}

#[test]
fn a_flattened_none_field_is_left_out() {
    // serde writes a struct with a flattened field as a Map, so this is the
    // Map entry rule at work.
    #[derive(Serialize)]
    struct Outer {
        b: Option<i64>,
        #[serde(flatten)]
        inner: WithOption,
    }
    assert_eq!(
        to_value(&Outer {
            b: None,
            inner: WithOption { a: None }
        })
        .unwrap(),
        Value::map::<&str, 0>([])
    );
}

#[test]
fn a_none_outside_an_entry_is_an_error() {
    for (case, message) in [
        ("at the top level", serialize_error(&None::<i64>)),
        ("an element", serialize_error(&vec![Some(1), None])),
        ("in a tuple", serialize_error(&(1, None::<i64>))),
        (
            "in a tuple variant",
            serialize_error(&Variants::Tuple(1, None)),
        ),
        (
            "in a newtype variant",
            serialize_error(&Variants::Newtype(None)),
        ),
        ("in a Some", serialize_error(&Some(None::<i64>))),
        // A `None` nested in a `Some` or a `Nullable` is not itself an
        // entry, even inside one.
        (
            "in a Some, in an entry",
            serialize_error(&HashMap::from([("a", Some(None::<i64>))])),
        ),
        (
            "in a Nullable",
            serialize_error(&Nullable(Some(None::<i64>))),
        ),
    ] {
        assert_eq!(message, NONE_OUTSIDE_AN_ENTRY, "{case}");
    }
}

// ---- Serializing a Nullable ----

#[test]
fn a_nullable_gives_none_as_null() {
    assert_eq!(to_value(&Nullable(None::<i64>)).unwrap(), Value::Null);
    assert_eq!(to_value(&Nullable(Some(5))).unwrap(), Value::Int(5));
    assert_eq!(
        to_value(&vec![Nullable(Some(1)), Nullable(None)]).unwrap(),
        Value::array([Value::Int(1), Value::Null])
    );
    assert_eq!(
        to_value(&WithNullable { a: Nullable(None) }).unwrap(),
        Value::map([("a", Value::Null)]),
        "a field holding Null is present"
    );
    assert_eq!(
        to_value(&HashMap::from([("a", Nullable(None::<i64>))])).unwrap(),
        Value::map([("a", Value::Null)])
    );
}

#[test]
fn an_optional_nullable_field_writes_absent_null_and_a_value_apart() {
    let write = |a| to_value(&WithOptionalNullable { a }).unwrap();
    assert_eq!(write(None), Value::map::<&str, 0>([]));
    assert_eq!(
        write(Some(Nullable(None))),
        Value::map([("a", Value::Null)])
    );
    assert_eq!(
        write(Some(Nullable(Some(5)))),
        Value::map([("a", Value::Int(5))])
    );
}

#[test]
fn an_optional_nullable_field_round_trips() {
    for a in [None, Some(Nullable(None)), Some(Nullable(Some(5)))] {
        let original = WithOptionalNullable { a };
        let value = to_value(&original).unwrap();
        assert_eq!(from_value::<WithOptionalNullable>(value).unwrap(), original);
    }
}

#[test]
fn a_nullable_converts_from_and_to_an_option() {
    assert_eq!(Nullable::from(Some(5)), Nullable(Some(5)));
    let none: Nullable<i64> = None.into();
    assert_eq!(none, Nullable(None));
    assert_eq!(Option::from(Nullable(Some(5))), Some(5));
}

// ---- Through other formats ----

#[test]
fn a_nullable_is_another_format_s_option() {
    assert_eq!(
        serde_json::to_string(&Nullable(None::<i64>)).unwrap(),
        "null"
    );
    assert_eq!(serde_json::to_string(&Nullable(Some(5))).unwrap(), "5");
    assert_eq!(
        serde_json::from_str::<Nullable<i64>>("null").unwrap(),
        Nullable(None)
    );
    assert_eq!(
        serde_json::from_str::<Nullable<i64>>("5").unwrap(),
        Nullable(Some(5))
    );
    // A format that does not describe itself reads back what it wrote.
    let values = vec![Nullable(Some(1)), Nullable(None), Nullable(Some(2))];
    let bytes = postcard::to_allocvec(&values).unwrap();
    assert_eq!(
        postcard::from_bytes::<Vec<Nullable<i64>>>(&bytes).unwrap(),
        values
    );
}

#[test]
fn a_nullable_reads_from_a_deserializer_that_gives_a_newtype_s_value() {
    // As serde's own value deserializers do.
    let five: I64Deserializer<ValueError> = 5i64.into_deserializer();
    assert_eq!(
        Nullable::<i64>::deserialize(five).unwrap(),
        Nullable(Some(5))
    );
    let unit: UnitDeserializer<ValueError> = ().into_deserializer();
    assert_eq!(Nullable::<i64>::deserialize(unit).unwrap(), Nullable(None));
}

// ---- serde's buffered representations ----

#[test]
fn a_buffered_representation_reads_null_into_an_option_as_none() {
    // serde buffers these through an intermediate form of its own, which the
    // bridge never sees, so its rule cannot hold there.
    #[derive(Deserialize, Debug, PartialEq)]
    #[serde(untagged)]
    enum Untagged {
        Fields { a: Option<i64> },
    }
    assert_eq!(
        from_value::<Untagged>(Value::map([("a", Value::Null)])).unwrap(),
        Untagged::Fields { a: None }
    );

    #[derive(Deserialize, Debug, PartialEq)]
    #[serde(tag = "type")]
    enum Tagged {
        Fields { a: Option<i64> },
    }
    assert_eq!(
        from_value::<Tagged>(Value::map([
            ("type", Value::from("Fields")),
            ("a", Value::Null)
        ]))
        .unwrap(),
        Tagged::Fields { a: None }
    );

    #[derive(Deserialize, Debug, PartialEq)]
    struct Flattened {
        #[serde(flatten)]
        inner: WithOption,
    }
    assert_eq!(
        from_value::<Flattened>(Value::map([("a", Value::Null)])).unwrap(),
        Flattened {
            inner: WithOption { a: None }
        }
    );
}
