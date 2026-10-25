//! `native::Opaque<T>`: host data of one payload type taken as an argument,
//! returned as a result, and converted through the serde bridge, without
//! copying the payload.

use std::fmt::Debug;

use serde::de::value::{Error as PlainError, F64Deserializer};
use serde::de::{DeserializeOwned, IntoDeserializer};
use serde::{Deserialize, Serialize};

use frostlang::native::{Args, De, FrostArg, Nullable, Opaque, Optional};
use frostlang::{
    FrostError, FrostOpaque, FrostType, Params, SpecialFloat, Value, from_value, frost_fn, to_value,
};

use crate::script::Script;

#[derive(Debug, PartialEq)]
struct Point(i64, i64);

impl FrostOpaque for Point {
    fn type_name() -> &'static str {
        "Point"
    }

    fn try_to_string(&self) -> Option<String> {
        Some(format!("{}, {}", self.0, self.1))
    }
}

#[frost_fn]
fn point(x: i64, y: i64) -> Opaque<Point> {
    Opaque::new(Point(x, y))
}

#[frost_fn]
fn sum(point: Opaque<Point>) -> i64 {
    point.0 + point.1
}

#[frost_fn]
fn describe(point: Optional<Nullable<Opaque<Point>>>) -> String {
    match point {
        Optional(None) => "omitted".to_string(),
        Optional(Some(Nullable(None))) => "null".to_string(),
        Optional(Some(Nullable(Some(point)))) => format!("{}, {}", point.0, point.1),
    }
}

/// A script with `point`, `sum`, and `describe`, and a SpecialFloat Opaque as
/// `special`.
fn script(source: &str) -> Script {
    let special = SpecialFloat::new(f64::NAN).expect("NaN is special");
    Script::new(source)
        .capture("point", point_frost().build())
        .capture("sum", sum_frost().build())
        .capture("describe", describe_frost().build())
        .capture("special", Value::opaque(special))
}

#[test]
fn opaque_declares_an_opaque_parameter() {
    assert_eq!(
        <Opaque<Point> as FrostArg>::PARAM.types(),
        FrostType::OPAQUE
    );
}

#[test]
fn an_opaque_returned_by_one_native_is_taken_by_another() {
    assert_eq!(script("sum(point(3, 4))").run(), Value::Int(7));
    assert_eq!(
        script("to_string(point(3, 4))").run(),
        Value::from("<Point: 3, 4>")
    );
}

#[test]
fn an_opaque_of_another_payload_type_is_rejected_naming_both() {
    assert_eq!(
        script("sum(special)").raises(),
        "Function sum requires argument 1 (point) to be an Opaque Point, \
         got an Opaque SpecialFloat"
    );
}

#[test]
fn a_value_of_another_type_is_a_type_error() {
    assert_eq!(
        script("sum(1)").raises(),
        "Function sum requires Opaque as argument 1 (point), got Int"
    );
}

#[test]
fn opaque_wraps_as_optional_and_nullable() {
    assert_eq!(script("describe()").run(), Value::from("omitted"));
    assert_eq!(script("describe(null)").run(), Value::from("null"));
    assert_eq!(script("describe(point(1, 2))").run(), Value::from("1, 2"));
    assert_eq!(
        script("describe(1)").raises(),
        "Function describe requires Null or Opaque as argument 1 (point), got Int"
    );
}

/// `values` taken as an `Opaque<Point>`, the sole parameter of `f`.
fn take(mut values: Vec<Value>) -> Opaque<Point> {
    let params = Params::new(const { &[<Opaque<Point> as FrostArg>::PARAM] });
    Args::new("f", params, &mut values)
        .take()
        .expect("a Point is taken")
}

#[test]
fn taking_an_opaque_shares_its_payload_without_copying() {
    // The argument is the payload's only holder, so taking it leaves the
    // taken Opaque the only one, and the payload comes out whole.
    let unique = take(vec![Value::from(Opaque::new(Point(1, 2)))]);
    assert_eq!(unique.try_into_inner().expect("unshared"), Point(1, 2));

    // Another holder keeps the payload shared, so it stays in the Opaque.
    let held = Opaque::new(Point(3, 4));
    let shared = take(vec![Value::from(held.clone())]);
    let shared = shared
        .try_into_inner()
        .expect_err("the payload is shared with `held`");
    assert_eq!(*shared, Point(3, 4));
    drop(held);
    assert_eq!(shared.try_into_inner().expect("unshared now"), Point(3, 4));
}

// `Point` keeps the default `equals`, so its `==` is handle identity, which no
// optimization may change: a handle stays one value however it is reached, and
// handles to equal payloads stay distinct. Each script runs under every
// optimization permutation.

#[test]
fn an_opaque_handle_stays_one_value_however_it_is_reached() {
    let source = r"
        def p = point(1, 2)
        def get = fn -> p
        def ps = map([1, 2], fn _ -> p)
        def q = p
        [p == p, get() == get(), ps[0] == ps[1], q == p, id(p) == p, [p] == [p]]
        ";
    assert_eq!(
        script(source).run(),
        script("[true, true, true, true, true, true]").run(),
        "a handle equals itself wherever it flows"
    );
}

#[test]
fn opaque_handles_to_equal_payloads_are_never_merged() {
    let source = r"
        def make = fn -> point(1, 2)
        def ps = map([1, 2], fn _ -> point(1, 2))
        [point(1, 2) == point(1, 2), make() == make(), ps[0] == ps[1]]
        ";
    assert_eq!(
        script(source).run(),
        script("[false, false, false]").run(),
        "each call makes its own handle, never equal to another's"
    );
}

#[test]
fn an_opaque_with_its_own_equals_compares_by_it_under_every_optimization() {
    // A NaN `SpecialFloat` equals another handle to one with the same bits.
    let other_nan = SpecialFloat::new(f64::NAN).expect("NaN is special");
    let source = r"
        def get = fn -> other_nan
        [special == other_nan, get() == special, [special] == [other_nan]]
        ";
    assert_eq!(
        script(source)
            .capture("other_nan", Value::opaque(other_nan))
            .run(),
        script("[true, true, true]").run(),
        "two handles compare by the payloads' own `equals`"
    );
}

// --- Through the serde bridge ---

/// A struct of Opaque fields, as a host would take or give options.
#[derive(Debug, Serialize, Deserialize)]
struct Shape {
    origin: Opaque<Point>,
    corner: Option<Opaque<Point>>,
    center: Nullable<Opaque<Point>>,
}

/// The error message of deserializing `value` as `T`, which must fail.
fn deserialize_error<T: DeserializeOwned + Debug>(value: Value) -> String {
    from_value::<T>(value)
        .expect_err("deserializing fails")
        .message()
        .into_owned()
}

/// An Opaque of a `Point` at `x`, `y`.
fn point_of(x: i64, y: i64) -> Opaque<Point> {
    Opaque::new(Point(x, y))
}

#[test]
fn from_value_takes_an_opaque_sharing_its_payload() {
    let held = point_of(1, 2);
    let taken = from_value::<Opaque<Point>>(Value::from(held.clone())).unwrap();
    let taken = taken
        .try_into_inner()
        .expect_err("the payload is shared with `held`");
    assert_eq!(*taken, Point(1, 2));
}

#[test]
fn an_opaque_field_round_trips_as_the_same_handle() {
    let (origin, center) = (Value::from(point_of(1, 2)), Value::from(point_of(3, 4)));
    let value = Value::map([("origin", origin.clone()), ("center", center)]);
    let shape = from_value::<Shape>(value.clone()).unwrap();
    assert!(shape.corner.is_none(), "an absent field is None");
    let back = to_value(&shape).unwrap();
    // A Point compares by identity, so equal Values hold the same handles.
    assert_eq!(back, value);

    let null_center = Value::map([("origin", origin), ("center", Value::Null)]);
    let shape = from_value::<Shape>(null_center.clone()).unwrap();
    assert!(shape.center.0.is_none(), "a Nullable takes Null");
    assert_eq!(to_value(&shape).unwrap(), null_center);
}

#[test]
fn from_value_refuses_an_opaque_of_another_payload_type_naming_both() {
    let special = Value::opaque(SpecialFloat::new(f64::NAN).unwrap());
    assert_eq!(
        deserialize_error::<Opaque<Point>>(special.clone()),
        "expected an Opaque Point, got an Opaque SpecialFloat"
    );
    assert_eq!(
        deserialize_error::<Shape>(Value::map([("origin", special)])),
        "expected an Opaque Point, got an Opaque SpecialFloat (at `origin`)"
    );
}

#[test]
fn from_value_refuses_another_type() {
    assert_eq!(
        deserialize_error::<Opaque<Point>>(Value::Int(1)),
        "expected Opaque, got Int"
    );
    assert_eq!(
        deserialize_error::<Opaque<Point>>(Value::Null),
        "expected Opaque, got Null",
        "only a Nullable takes Null"
    );
}

/// `value` taken as a `T`, the sole parameter of `f`, named `x`.
fn take_as<T: FrostArg>(value: Value) -> Result<T, FrostError> {
    let params = Params::new(const { &[<T as FrostArg>::PARAM.named("x")] });
    Args::new("f", params, &mut [value]).take()
}

/// The error message of taking `value` as a `T`, which must fail.
fn take_error<T: FrostArg + Debug>(value: Value) -> String {
    take_as::<T>(value)
        .expect_err("taking fails")
        .message()
        .into_owned()
}

#[test]
fn de_takes_an_opaque() {
    let De(taken) = take_as::<De<Opaque<Point>>>(Value::from(point_of(5, 6))).unwrap();
    assert_eq!(*taken, Point(5, 6));
    let De(Nullable(taken)) = take_as::<De<Nullable<Opaque<Point>>>>(Value::Null).unwrap();
    assert!(taken.is_none(), "a Nullable takes Null");
}

#[test]
fn de_refuses_another_type_as_a_wrong_argument_type() {
    assert_eq!(
        take_error::<De<Opaque<Point>>>(Value::Int(1)),
        "Function f requires Opaque as argument 1 (x), got Int"
    );
    assert_eq!(
        take_error::<De<Nullable<Opaque<Point>>>>(Value::Int(1)),
        "Function f requires Null or Opaque as argument 1 (x), got Int"
    );
}

#[test]
fn de_refuses_an_opaque_of_another_payload_type_as_invalid() {
    let special = Value::opaque(SpecialFloat::new(f64::NAN).unwrap());
    assert_eq!(
        take_error::<De<Opaque<Point>>>(special.clone()),
        "Function f requires argument 1 (x) to be valid: \
         expected an Opaque Point, got an Opaque SpecialFloat"
    );
    assert_eq!(
        take_error::<De<Vec<Opaque<Point>>>>(Value::array([special])),
        "Function f requires argument 1 (x) to be valid: \
         expected an Opaque Point, got an Opaque SpecialFloat (at `[0]`)"
    );
}

#[test]
fn a_foreign_format_refuses_an_opaque() {
    assert_eq!(
        serde_json::to_string(&point_of(1, 2))
            .expect_err("refused")
            .to_string(),
        "cannot serialize Opaque"
    );
    assert_eq!(
        serde_json::from_str::<Opaque<Point>>("1")
            .expect_err("refused")
            .to_string(),
        "expected Opaque, got Int"
    );
}

#[test]
fn an_opaque_special_float_crosses_a_foreign_format_as_its_float() {
    // Both ways, as a Value holding a SpecialFloat does.
    let infinity = Opaque::new(SpecialFloat::new(f64::INFINITY).unwrap());
    assert_eq!(
        postcard::to_allocvec(&infinity).unwrap(),
        postcard::to_allocvec(&f64::INFINITY).unwrap(),
        "it serializes as its float"
    );
    // A format's decoder presenting the float.
    let decoder: F64Deserializer<PlainError> = f64::INFINITY.into_deserializer();
    let back = Opaque::<SpecialFloat>::deserialize(decoder).unwrap();
    assert_eq!(back.get(), f64::INFINITY);
}

#[test]
fn an_opaque_converts_into_a_value_naming_its_payload_type() {
    let value = Value::from(Opaque::new(Point(5, 6)));
    let handle = value.as_opaque().expect("an Opaque value");
    assert_eq!(handle.type_name(), "Point");
    assert_eq!(handle.downcast_ref::<Point>(), Some(&Point(5, 6)));
}
