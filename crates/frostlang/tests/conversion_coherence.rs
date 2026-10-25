//! One set of conversion rules: a Rust type taken as a native's argument
//! converts as it does through `from_value` (`De`), and one returned as a
//! native's result converts as it does through `to_value` (`Ser`).

use std::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;

use frostlang::native::{
    Args, De, Flat, FromArg, FrostArg, Function, IntoNativeResult, Nonnull, Nullable, Numeric,
    Opaque, Primitive, Ser, Structured,
};
use frostlang::{FrostFloat, FrostOpaque, Params, SpecialFloat, Value, frost_closure};

/// The payload of the samples' Opaques.
#[derive(Debug, PartialEq)]
struct Point;

impl FrostOpaque for Point {
    fn type_name() -> &'static str {
        "Point"
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

/// A value of every type, plus values on either side of each edge a
/// conversion draws within a type, such as an integer type's bounds.
fn samples() -> Vec<Value> {
    let float = |f: f64| Value::Float(FrostFloat::new(f).unwrap());
    let mut samples = vec![
        Value::Null,
        Value::Bool(true),
        float(1.5),
        float(2.0),
        float(-0.0),
        float(1e39),
        Value::opaque(SpecialFloat::new(f64::NAN).unwrap()),
        Value::opaque(SpecialFloat::new(f64::INFINITY).unwrap()),
        Value::from(""),
        Value::from("a"),
        Value::from("ab"),
        Value::from("e\u{301}"),
        Value::from(&b"a"[..]),
        Value::array::<i64, 0>([]),
        Value::array([1, 2]),
        Value::array([Value::Int(1), Value::Null]),
        Value::array([Value::Int(1), Value::from("a")]),
        // A Function and an Opaque nested in an Array, which `from_value`
        // passes across whole, as a native argument holds them.
        Value::array([
            frost_closure!("g", || Value::Null),
            Value::from(Opaque::new(Point)),
        ]),
        Value::map::<&str, 0>([]),
        Value::map([("a", Value::Int(1))]),
        frost_closure!("f", || Value::Null),
        Value::from(Opaque::new(Point)),
    ];
    // Each integer type's bounds, and one past each, within the Int range.
    let bounds: [(i128, i128); 6] = [
        (i8::MIN.into(), i8::MAX.into()),
        (u8::MIN.into(), u8::MAX.into()),
        (i16::MIN.into(), i16::MAX.into()),
        (u16::MIN.into(), u16::MAX.into()),
        (i32::MIN.into(), i32::MAX.into()),
        (u32::MIN.into(), u32::MAX.into()),
    ];
    for (min, max) in bounds {
        for int in [min - 1, min, max, max + 1] {
            samples.push(Value::Int(i64::try_from(int).unwrap()));
        }
    }
    samples.extend([Value::Int(i64::MIN), Value::Int(i64::MAX)]);
    samples
}

/// `value` taken as the sole argument of a native `f` taking a `T`, after the
/// type check its call makes: `None` if refused.
fn taken<T: FrostArg>(value: Value) -> Option<T> {
    let params = Params::new(const { &[<T as FrostArg>::PARAM] });
    if !value.fits(<T as FrostArg>::PARAM.types()) {
        return None;
    }
    Args::new("f", params, &mut [value]).take().ok()
}

/// Asserts that every sample is taken as a `T` alike as an argument and
/// through `from_value`: refused by both, or taken by both as equal values.
/// Some sample must be taken, so that refusing everything cannot pass.
fn assert_taken_alike<T>(name: &str)
where
    T: FromArg + DeserializeOwned + Debug + PartialEq,
{
    let mut any_taken = false;
    for value in samples() {
        let as_argument = taken::<T>(value.clone());
        let through_bridge = taken::<De<T>>(value.clone()).map(|De(taken)| taken);
        any_taken |= as_argument.is_some();
        assert_eq!(
            as_argument,
            through_bridge,
            "{name} from {}: as an argument, then through `from_value`",
            value.to_debug_string()
        );
    }
    assert!(any_taken, "{name} takes some sample");
}

#[test]
fn an_argument_converts_as_from_value_does() {
    assert_taken_alike::<Value>("Value");
    assert_taken_alike::<bool>("bool");
    assert_taken_alike::<i8>("i8");
    assert_taken_alike::<i16>("i16");
    assert_taken_alike::<i32>("i32");
    assert_taken_alike::<i64>("i64");
    assert_taken_alike::<i128>("i128");
    assert_taken_alike::<isize>("isize");
    assert_taken_alike::<u8>("u8");
    assert_taken_alike::<u16>("u16");
    assert_taken_alike::<u32>("u32");
    assert_taken_alike::<u64>("u64");
    assert_taken_alike::<u128>("u128");
    assert_taken_alike::<usize>("usize");
    assert_taken_alike::<f64>("f64");
    assert_taken_alike::<FrostFloat>("FrostFloat");
    assert_taken_alike::<char>("char");
    assert_taken_alike::<String>("String");
    assert_taken_alike::<Vec<i64>>("Vec<i64>");
    assert_taken_alike::<Vec<Value>>("Vec<Value>");
    assert_taken_alike::<Vec<Nullable<i64>>>("Vec<Nullable<i64>>");
    assert_taken_alike::<Nullable<i64>>("Nullable<i64>");
    assert_taken_alike::<Nullable<String>>("Nullable<String>");
    assert_taken_alike::<Numeric>("Numeric");
    assert_taken_alike::<Flat>("Flat");
    assert_taken_alike::<Structured>("Structured");
    assert_taken_alike::<Primitive>("Primitive");
    assert_taken_alike::<Nonnull>("Nonnull");
}

#[test]
fn an_opaque_or_function_argument_converts_as_from_value_does() {
    // These compare what each conversion takes as a `Value`, which compares an
    // Opaque or a Function by identity: both must take the very same one.
    let (mut any_opaque, mut any_function) = (false, false);
    for value in samples() {
        let as_argument = taken::<Opaque<Point>>(value.clone()).map(Value::from);
        let through_bridge = taken::<De<Opaque<Point>>>(value.clone()).map(|De(p)| Value::from(p));
        any_opaque |= as_argument.is_some();
        assert_eq!(
            as_argument,
            through_bridge,
            "Opaque<Point> from {}",
            value.to_debug_string()
        );

        let as_argument = taken::<Function>(value.clone()).map(Value::from);
        let through_bridge = taken::<De<Function>>(value.clone()).map(|De(f)| Value::from(f));
        any_function |= as_argument.is_some();
        assert_eq!(
            as_argument,
            through_bridge,
            "Function from {}",
            value.to_debug_string()
        );
    }
    assert!(any_opaque, "Opaque<Point> takes some sample");
    assert!(any_function, "Function takes some sample");
}

/// `result` returned by a native `f`: `None` if refused.
fn returned(result: impl IntoNativeResult) -> Option<Value> {
    result.into_result("f").ok()
}

/// Asserts that each of `results` returns alike as itself and through
/// `to_value`: refused by both, or the same value from both.
fn assert_returned_alike<T>(name: &str, results: impl IntoIterator<Item = T>)
where
    T: IntoNativeResult + Serialize + Clone + Debug,
{
    for result in results {
        assert_eq!(
            returned(result.clone()),
            returned(Ser(result.clone())),
            "{name} {result:?}: as itself, then through `to_value`"
        );
    }
}

#[test]
fn a_result_converts_as_to_value_does() {
    assert_returned_alike("Value", samples());
    assert_returned_alike("()", [()]);
    assert_returned_alike("bool", [false, true]);
    assert_returned_alike("i8", [i8::MIN, i8::MAX]);
    assert_returned_alike("i16", [i16::MIN, i16::MAX]);
    assert_returned_alike("i32", [i32::MIN, i32::MAX]);
    assert_returned_alike("i64", [i64::MIN, i64::MAX]);
    assert_returned_alike("isize", [isize::MIN, isize::MAX]);
    assert_returned_alike("u8", [0, u8::MAX]);
    assert_returned_alike("u16", [0, u16::MAX]);
    assert_returned_alike("u32", [0, u32::MAX]);
    assert_returned_alike(
        "i128",
        [i128::MIN, i64::MIN.into(), i64::MAX.into(), i128::MAX],
    );
    assert_returned_alike("u64", [0, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX]);
    assert_returned_alike("u128", [0, i64::MAX as u128, u128::MAX]);
    assert_returned_alike("usize", [0, usize::MAX]);
    assert_returned_alike("f64", [0.0, -0.0, 1.5, f64::MAX, f64::NAN, f64::INFINITY]);
    assert_returned_alike("FrostFloat", [FrostFloat::new(1.5).unwrap()]);
    assert_returned_alike("char", ['a', '\u{1F600}']);
    assert_returned_alike("String", [String::new(), "a".to_string()]);
    assert_returned_alike("&str", ["", "a"]);
    assert_returned_alike("Vec<i64>", [vec![], vec![1, 2]]);
    assert_returned_alike("Vec<f64>", [vec![1.0, f64::NAN]]);
    assert_returned_alike("Nullable<i64>", [Nullable(None), Nullable(Some(1))]);
    assert_returned_alike("Nullable<f64>", [Nullable(Some(f64::NAN))]);
    assert_returned_alike("Opaque<Point>", [Opaque::new(Point)]);
    // Each category, holding each sample of its types.
    assert_returned_alike("Numeric", members::<Numeric>());
    assert_returned_alike("Flat", members::<Flat>());
    assert_returned_alike("Structured", members::<Structured>());
    assert_returned_alike("Primitive", members::<Primitive>());
    assert_returned_alike("Nonnull", members::<Nonnull>());
    assert_returned_alike("Function", members::<Function>());
}

/// Each sample that converts into the category `C`, as one.
fn members<C: TryFrom<Value>>() -> Vec<C> {
    let members: Vec<C> = samples()
        .into_iter()
        .filter_map(|value| C::try_from(value).ok())
        .collect();
    assert!(
        !members.is_empty(),
        "some sample is a {}",
        std::any::type_name::<C>()
    );
    members
}
