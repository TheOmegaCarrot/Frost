//! Taking a native's arguments as Rust types, with `frostlang::native`: the
//! `Param` each argument type declares, the conversions `Args` performs, and
//! the errors it words for content an argument type cannot take.
//!
//! Most cases drive `Args` directly, over arguments already of the declared
//! types, as `checked_native` would hand them over. The last section runs
//! natives built this way from Frost source.

use std::collections::BTreeMap;
use std::fmt::Debug;

use frostlang::native::{Args, De, FromArg, FrostArg, Nullable, Optional};
use frostlang::{
    FrostArray, FrostBytes, FrostError, FrostFloat, FrostMap, FrostString, FrostType, Params, Value,
};
use serde::Deserialize;

use crate::script::Script;

/// The spec of a native taking one argument, named `x`, as `T`.
fn params<T: FrostArg>() -> Params {
    Params::new(const { &[<T as FrostArg>::PARAM.named("x")] })
}

/// `values` taken as `T`, the sole parameter of a native named `f`.
fn take<T: FrostArg>(mut values: Vec<Value>) -> Result<T, FrostError> {
    Args::new("f", params::<T>(), &mut values).take()
}

/// `value` taken as `T`, which must succeed.
fn taken<T: FrostArg>(value: Value) -> T {
    take(vec![value]).unwrap_or_else(|err| panic!("the argument is taken, but: {err}"))
}

/// The error message of taking `value` as `T`, which must fail.
fn rejection<T: FrostArg + Debug>(value: Value) -> String {
    match take::<T>(vec![value]) {
        Ok(taken) => panic!("the argument is rejected, but is taken as {taken:?}"),
        Err(err) => err.message().into_owned(),
    }
}

// --- Params from argument types ---

#[test]
fn each_argument_type_declares_its_types() {
    let cases = [
        (<Value as FrostArg>::PARAM, FrostType::ANY),
        (<bool as FrostArg>::PARAM, FrostType::BOOL),
        (<i64 as FrostArg>::PARAM, FrostType::INT),
        (<u8 as FrostArg>::PARAM, FrostType::INT),
        (<f64 as FrostArg>::PARAM, FrostType::NUMERIC),
        (<FrostFloat as FrostArg>::PARAM, FrostType::FLOAT),
        (<String as FrostArg>::PARAM, FrostType::STRING),
        (<FrostString as FrostArg>::PARAM, FrostType::STRING),
        (<FrostBytes as FrostArg>::PARAM, FrostType::BYTES),
        (<FrostArray as FrostArg>::PARAM, FrostType::ARRAY),
        (<FrostMap as FrostArg>::PARAM, FrostType::MAP),
        (<Vec<String> as FrostArg>::PARAM, FrostType::ARRAY),
        (<De<Options> as FrostArg>::PARAM, FrostType::ANY),
        (
            <Nullable<String> as FrostArg>::PARAM,
            FrostType::String | FrostType::Null,
        ),
        (<Optional<String> as FrostArg>::PARAM, FrostType::STRING),
    ];
    for (i, (param, types)) in cases.into_iter().enumerate() {
        assert_eq!(param.types(), types, "case {i}");
        assert_eq!(param.name(), None, "case {i} is unnamed");
    }
}

#[test]
fn only_optional_declares_an_optional_parameter() {
    assert!(<Optional<u8> as FrostArg>::PARAM.is_optional());
    assert!(<Optional<Nullable<u8>> as FrostArg>::PARAM.is_optional());
    assert!(!<u8 as FrostArg>::PARAM.is_optional());
    assert!(!<Nullable<u8> as FrostArg>::PARAM.is_optional());
    assert!(!<Vec<u8> as FrostArg>::PARAM.is_optional());
}

#[test]
fn a_spec_built_from_argument_types_is_const() {
    const PARAMS: Params = Params::new(&[
        <String as FrostArg>::PARAM.named("text"),
        <Optional<Nullable<u32>> as FrostArg>::PARAM.named("width"),
    ]);
    assert_eq!(PARAMS.arity(), frostlang::Arity::Between(1, 2));
}

// --- Plain values ---

#[test]
fn value_takes_any_argument_as_is() {
    for value in [
        Value::Null,
        Value::Int(1),
        Value::from("text"),
        Value::array([1, 2]),
    ] {
        assert_eq!(taken::<Value>(value.clone()), value);
    }
}

#[test]
fn each_type_takes_its_value() {
    assert!(taken::<bool>(Value::Bool(true)));
    assert_eq!(
        taken::<FrostFloat>(Value::Float(FrostFloat::new(2.5).unwrap())).get(),
        2.5
    );
    assert_eq!(taken::<String>(Value::from("text")), "text");
    assert_eq!(taken::<FrostString>(Value::from("text")).as_str(), "text");
    assert_eq!(
        taken::<FrostBytes>(Value::from(&b"\x00\xff"[..])).as_slice(),
        b"\x00\xff"
    );
    assert_eq!(
        Value::from(taken::<FrostArray>(Value::array([1, 2]))),
        Value::array([1, 2])
    );
    let map = Value::map([("a", Value::Int(1))]);
    assert_eq!(Value::from(taken::<FrostMap>(map.clone())), map);
}

#[test]
fn f64_takes_a_float_or_an_int() {
    assert_eq!(
        taken::<f64>(Value::Float(FrostFloat::new(2.5).unwrap())),
        2.5
    );
    assert_eq!(taken::<f64>(Value::Int(-3)), -3.0);
    // An Int beyond 2^53 becomes the nearest f64.
    assert_eq!(
        taken::<f64>(Value::Int(i64::MAX)),
        9_223_372_036_854_775_808.0
    );
}

// --- Integers ---

#[test]
fn each_integer_type_takes_every_int_in_its_range() {
    assert_eq!(taken::<u8>(Value::Int(0)), 0);
    assert_eq!(taken::<u8>(Value::Int(255)), 255);
    assert_eq!(taken::<i8>(Value::Int(-128)), -128);
    assert_eq!(taken::<i8>(Value::Int(127)), 127);
    assert_eq!(taken::<u16>(Value::Int(65_535)), 65_535);
    assert_eq!(taken::<i16>(Value::Int(-32_768)), -32_768);
    assert_eq!(taken::<u32>(Value::Int(4_294_967_295)), 4_294_967_295);
    assert_eq!(taken::<i32>(Value::Int(-2_147_483_648)), -2_147_483_648);
    assert_eq!(taken::<u64>(Value::Int(i64::MAX)), i64::MAX as u64);
    assert_eq!(taken::<usize>(Value::Int(7)), 7);
    assert_eq!(taken::<u128>(Value::Int(i64::MAX)), i64::MAX as u128);
    for int in [i64::MIN, -1, 0, 1, i64::MAX] {
        assert_eq!(taken::<i64>(Value::Int(int)), int);
        assert_eq!(taken::<i128>(Value::Int(int)), i128::from(int));
        assert_eq!(taken::<isize>(Value::Int(int)), int as isize);
    }
}

#[test]
fn an_int_out_of_range_is_rejected_with_the_range() {
    let cases = [
        (rejection::<u8>(Value::Int(256)), "from 0 to 255, got 256"),
        (rejection::<u8>(Value::Int(-1)), "from 0 to 255, got -1"),
        (
            rejection::<i8>(Value::Int(128)),
            "from -128 to 127, got 128",
        ),
        (
            rejection::<i8>(Value::Int(-129)),
            "from -128 to 127, got -129",
        ),
        (
            rejection::<u16>(Value::Int(65_536)),
            "from 0 to 65535, got 65536",
        ),
        (
            rejection::<i16>(Value::Int(-32_769)),
            "from -32768 to 32767, got -32769",
        ),
        (
            rejection::<u32>(Value::Int(4_294_967_296)),
            "from 0 to 4294967295, got 4294967296",
        ),
        (
            rejection::<i32>(Value::Int(2_147_483_648)),
            "from -2147483648 to 2147483647, got 2147483648",
        ),
        // A range reaching past an Int's states only the bound an Int can pass.
        (rejection::<u64>(Value::Int(-1)), "at least 0, got -1"),
        (rejection::<usize>(Value::Int(-1)), "at least 0, got -1"),
        (
            rejection::<u128>(Value::Int(i64::MIN)),
            "at least 0, got -9223372036854775808",
        ),
    ];
    for (message, requirement) in cases {
        assert_eq!(
            message,
            format!("Function f requires argument 1 (x) to be {requirement}")
        );
    }
}

// --- Optional and Nullable ---

#[test]
fn optional_is_none_only_when_the_argument_is_omitted() {
    assert_eq!(take::<Optional<u8>>(vec![]).unwrap(), Optional(None));
    assert_eq!(taken::<Optional<u8>>(Value::Int(5)), Optional(Some(5)));
    // Null is an argument: it reaches the inner type, which takes it as is.
    assert_eq!(
        taken::<Optional<Value>>(Value::Null),
        Optional(Some(Value::Null))
    );
}

#[test]
fn nullable_is_none_only_for_null() {
    assert_eq!(taken::<Nullable<u8>>(Value::Null), Nullable(None));
    assert_eq!(taken::<Nullable<u8>>(Value::Int(5)), Nullable(Some(5)));
}

#[test]
fn optional_nullable_tells_omitted_from_null() {
    assert_eq!(
        take::<Optional<Nullable<u8>>>(vec![]).unwrap(),
        Optional(None)
    );
    assert_eq!(
        taken::<Optional<Nullable<u8>>>(Value::Null),
        Optional(Some(Nullable(None)))
    );
    assert_eq!(
        taken::<Optional<Nullable<u8>>>(Value::Int(5)),
        Optional(Some(Nullable(Some(5))))
    );
}

#[test]
fn optional_and_nullable_pass_on_the_inner_rejection() {
    let expected = "Function f requires argument 1 (x) to be from 0 to 255, got 300";
    assert_eq!(rejection::<Optional<u8>>(Value::Int(300)), expected);
    assert_eq!(rejection::<Nullable<u8>>(Value::Int(300)), expected);
    assert_eq!(
        rejection::<Optional<Nullable<u8>>>(Value::Int(300)),
        expected
    );
}

// --- Arrays ---

#[test]
fn vec_takes_each_element() {
    assert_eq!(taken::<Vec<String>>(Value::array(["a", "b"])), ["a", "b"]);
    assert_eq!(taken::<Vec<u8>>(Value::array([0, 255])), [0, 255]);
    assert_eq!(
        taken::<Vec<u8>>(Value::array::<i64, 0>([])),
        Vec::<u8>::new()
    );
    assert_eq!(
        taken::<Vec<Vec<u8>>>(Value::array([Value::array([1]), Value::array([2, 3])])),
        [vec![1], vec![2, 3]]
    );
}

#[test]
fn vec_rejects_an_element_of_the_wrong_type_as_check_args_would() {
    assert_eq!(
        rejection::<Vec<String>>(Value::array([Value::from("a"), Value::Int(1)])),
        "Function f requires String as element 1 of argument 1 (x), got Int"
    );
    assert_eq!(
        rejection::<Vec<Nullable<u8>>>(Value::array([Value::from("a")])),
        "Function f requires Null or Int as element 0 of argument 1 (x), got String"
    );
}

#[test]
fn vec_rejects_an_element_with_content_its_type_cannot_take() {
    assert_eq!(
        rejection::<Vec<u8>>(Value::array([1, 300])),
        "Function f requires element 1 of argument 1 (x) to be from 0 to 255, got 300"
    );
    assert_eq!(
        rejection::<Vec<Vec<u8>>>(Value::array([Value::array([1]), Value::array([2, -3])])),
        "Function f requires element 1 of element 1 of argument 1 (x) to be from 0 to 255, got -3"
    );
}

// --- Deserialized ---

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct Options {
    width: u32,
    label: Option<String>,
}

#[test]
fn de_deserializes_the_argument() {
    let options = Value::map([("width", Value::Int(3)), ("label", Value::from("a"))]);
    assert_eq!(
        taken::<De<Options>>(options),
        De(Options {
            width: 3,
            label: Some("a".to_string()),
        })
    );
    assert_eq!(
        taken::<De<BTreeMap<String, i64>>>(Value::map([("a", Value::Int(1))])),
        De(BTreeMap::from([("a".to_string(), 1)]))
    );
}

#[test]
fn de_rejects_what_does_not_deserialize_with_the_reason() {
    let cases = [
        (
            Value::map([("wdth", Value::Int(3))]),
            "unknown field `wdth`, expected `width` or `label` (at a key)",
        ),
        (
            Value::map([("width", Value::from("3"))]),
            "expected Int, got String (at `width`)",
        ),
    ];
    for (value, reason) in cases {
        assert_eq!(
            rejection::<De<Options>>(value),
            format!("Function f requires argument 1 (x) to be valid: {reason}")
        );
    }
}

// --- Several parameters ---

#[test]
fn args_takes_each_parameter_in_turn_and_names_it_in_errors() {
    const PARAMS: Params = Params::new(&[
        <String as FrostArg>::PARAM.named("text"),
        <u8 as FrostArg>::PARAM,
        <Optional<u8> as FrostArg>::PARAM.named("pad"),
    ]);
    let mut values = vec![Value::from("a"), Value::Int(2)];
    let mut args = Args::new("g", PARAMS, &mut values);
    assert_eq!(args.take::<String>().unwrap(), "a");
    assert_eq!(args.take::<u8>().unwrap(), 2);
    assert_eq!(args.take::<Optional<u8>>().unwrap(), Optional(None));

    // An unnamed parameter is named by position alone.
    let mut values = vec![Value::from("a"), Value::Int(-1)];
    let mut args = Args::new("g", PARAMS, &mut values);
    args.take::<String>().unwrap();
    assert_eq!(
        args.take::<u8>().unwrap_err().message(),
        "Function g requires argument 2 to be from 0 to 255, got -1"
    );

    let mut values = vec![Value::from("a"), Value::Int(1), Value::Int(256)];
    let mut args = Args::new("g", PARAMS, &mut values);
    args.take::<String>().unwrap();
    args.take::<u8>().unwrap();
    assert_eq!(
        args.take::<Optional<u8>>().unwrap_err().message(),
        "Function g requires argument 3 (pad) to be from 0 to 255, got 256"
    );
}

#[test]
fn args_takes_each_argument_leaving_null() {
    let mut values = vec![Value::array([1, 2])];
    let mut args = Args::new("f", params::<FrostArray>(), &mut values);
    args.take::<FrostArray>().unwrap();
    assert_eq!(values, [Value::Null]);
}

#[test]
#[should_panic(expected = "Function f has 1 parameters, all taken already")]
fn taking_past_the_last_parameter_panics() {
    let mut values = vec![Value::Int(1)];
    let mut args = Args::new("f", params::<u8>(), &mut values);
    args.take::<u8>().unwrap();
    let _ = args.take::<u8>();
}

#[test]
#[should_panic(expected = "argument 1 (x) of f is String, which its Rust type cannot take")]
fn taking_an_argument_of_a_type_the_params_exclude_panics() {
    let _ = take::<u8>(vec![Value::from("text")]);
}

#[test]
#[should_panic(expected = "argument 1 (x) of f is required, but its arity let the call omit it")]
fn taking_an_omitted_required_argument_panics() {
    let _ = take::<u8>(vec![]);
}

// --- A custom argument type ---

/// A compass direction, taken from its name.
#[derive(Debug, PartialEq)]
enum Direction {
    North,
    South,
}

impl FromArg for Direction {
    const TYPES: frostlang::EnumSet<FrostType> = FrostType::STRING;

    fn from_arg(value: Value, site: &frostlang::native::ArgSite<'_>) -> Result<Self, FrostError> {
        match value.as_str() {
            Some("north") => Ok(Direction::North),
            Some("south") => Ok(Direction::South),
            Some(other) => Err(site.requires(format_args!("'north' or 'south', got '{other}'"))),
            None => unreachable!("type-checked as a String"),
        }
    }
}

#[test]
fn a_custom_type_takes_arguments_and_words_errors_alike() {
    assert_eq!(taken::<Direction>(Value::from("south")), Direction::South);
    assert_eq!(
        taken::<Optional<Direction>>(Value::from("north")),
        Optional(Some(Direction::North))
    );
    assert_eq!(
        rejection::<Direction>(Value::from("up")),
        "Function f requires argument 1 (x) to be 'north' or 'south', got 'up'"
    );
    assert_eq!(
        rejection::<Vec<Direction>>(Value::array(["north", "west"])),
        "Function f requires element 1 of argument 1 (x) to be 'north' or 'south', got 'west'"
    );
}

// --- From Frost ---

/// `pad(text, width, fill?)`: `text` padded on the left to `width` characters
/// with `fill`, a space when omitted, or nothing when Null.
fn pad() -> Value {
    const PARAMS: Params = Params::new(&[
        <String as FrostArg>::PARAM.named("text"),
        <u16 as FrostArg>::PARAM.named("width"),
        <Optional<Nullable<String>> as FrostArg>::PARAM.named("fill"),
    ]);
    Value::checked_native("pad", PARAMS, |ctx, args| {
        let mut args = Args::new(ctx.name(), PARAMS, args);
        let text: String = args.take()?;
        let width: u16 = args.take()?;
        let fill = match args.take()? {
            Optional(None) => " ".to_string(),
            Optional(Some(Nullable(None))) => return Ok(text.into()),
            Optional(Some(Nullable(Some(fill)))) => fill,
        };
        let missing = usize::from(width).saturating_sub(text.chars().count());
        Ok(format!("{}{text}", fill.repeat(missing)).into())
    })
}

#[test]
fn a_native_taking_args_runs_from_frost() {
    let run = |source: &str| Script::new(source).capture("pad", pad()).run();
    assert_eq!(run("pad('ab', 4)"), Value::from("  ab"));
    assert_eq!(run("pad('ab', 4, '0')"), Value::from("00ab"));
    assert_eq!(run("pad('ab', 4, null)"), Value::from("ab"));
    assert_eq!(run("pad('abcdef', 4)"), Value::from("abcdef"));
}

#[test]
fn a_native_taking_args_words_type_and_content_errors_alike() {
    let raises = |source: &str| Script::new(source).capture("pad", pad()).raises();
    assert_eq!(
        raises("pad('ab', '4')"),
        "Function pad requires Int as argument 2 (width), got String"
    );
    assert_eq!(
        raises("pad('ab', 4, 0)"),
        "Function pad requires Null or String as argument 3 (fill), got Int"
    );
    assert_eq!(
        raises("pad('ab', 70000)"),
        "Function pad requires argument 2 (width) to be from 0 to 65535, got 70000"
    );
    assert_eq!(
        raises("pad('ab')"),
        "Function pad expects between 2 and 3 arguments, but was called with 1"
    );
}
