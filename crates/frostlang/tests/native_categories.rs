//! The argument types for Frost's type categories, and for Function: the types
//! each accepts, what each takes an argument as, and returning each, through
//! natives made with `#[frost_fn]`.

use frostlang::native::{
    Flat, FrostArg, Function, IntoNativeResult, Nonnull, Numeric, Primitive, Structured,
};
use std::fmt::Debug;

use frostlang::{
    Arity, FrostBytes, FrostError, FrostFloat, FrostResult, FrostType, NativeCtx, SpecialFloat,
    TypeSet, Value, from_value, frost_fn, to_value,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::script::Script;

fn run(function: Value, source: &str) -> Value {
    Script::new(source).capture("f", function).run()
}

fn raises(function: Value, source: &str) -> String {
    Script::new(source).capture("f", function).raises()
}

/// Every Frost type, as a Frost source literal, with its type.
const ONE_OF_EACH: [(&str, FrostType); 10] = [
    ("null", FrostType::Null),
    ("true", FrostType::Bool),
    ("1", FrostType::Int),
    ("1.5", FrostType::Float),
    ("'text'", FrostType::String),
    ("x'00ff'", FrostType::Bytes),
    ("[1]", FrostType::Array),
    ("{a: 1}", FrostType::Map),
    ("fn -> 1", FrostType::Function),
    ("opaque", FrostType::Opaque),
];

/// Asserts that `function`, a native named `name` taking one argument `x` of
/// a category accepting `types` and returning it, returns each value of those
/// types unchanged, and rejects each other value as `category` names it.
fn assert_accepts_exactly(name: &str, function: &Value, types: TypeSet, category: &str) {
    let opaque = Value::opaque(SpecialFloat::new(f64::INFINITY).expect("special"));
    for (literal, ty) in ONE_OF_EACH {
        // Compared in Frost, as a Function equals only itself.
        let source = format!(
            r"
                def value = {literal}
                f(value) == value
            "
        );
        let script = Script::new(&source)
            .capture("f", function.clone())
            .capture("opaque", opaque.clone());
        if types.contains(ty) {
            assert_eq!(
                script.run(),
                Value::Bool(true),
                "{name} returns {literal} unchanged, being one of {category}"
            );
        } else {
            assert_eq!(
                script.raises(),
                format!(
                    "Function {name} requires {category} as argument 1 (x), got {}",
                    ty.name()
                ),
                "{name} rejects {literal}"
            );
        }
    }
}

#[frost_fn(name = "numeric")]
fn numeric(x: Numeric) -> Numeric {
    x
}

#[frost_fn(name = "flat")]
fn flat(x: Flat) -> Flat {
    x
}

#[frost_fn(name = "structured")]
fn structured(x: Structured) -> Structured {
    x
}

#[frost_fn(name = "primitive")]
fn primitive(x: Primitive) -> Primitive {
    x
}

#[frost_fn(name = "nonnull")]
fn nonnull(x: Nonnull) -> Nonnull {
    x
}

#[frost_fn(name = "function")]
fn function(x: Function) -> Function {
    x
}

#[test]
fn each_type_declares_its_category() {
    let cases = [
        (<Numeric as FrostArg>::PARAM, FrostType::NUMERIC),
        (<Flat as FrostArg>::PARAM, FrostType::FLAT),
        (<Structured as FrostArg>::PARAM, FrostType::STRUCTURED),
        (<Primitive as FrostArg>::PARAM, FrostType::PRIMITIVE),
        (<Nonnull as FrostArg>::PARAM, FrostType::NONNULL),
        (<Function as FrostArg>::PARAM, FrostType::FUNCTION),
    ];
    for (param, types) in cases {
        assert_eq!(param.types(), types);
    }
}

#[test]
fn each_type_accepts_exactly_its_category_and_returns_it_unchanged() {
    assert_accepts_exactly("numeric", &numeric_frost(), FrostType::NUMERIC, "Numeric");
    assert_accepts_exactly("flat", &flat_frost(), FrostType::FLAT, "String or Bytes");
    assert_accepts_exactly(
        "structured",
        &structured_frost(),
        FrostType::STRUCTURED,
        "Structured",
    );
    assert_accepts_exactly(
        "primitive",
        &primitive_frost(),
        FrostType::PRIMITIVE,
        "Primitive",
    );
    assert_accepts_exactly("nonnull", &nonnull_frost(), FrostType::NONNULL, "Nonnull");
    assert_accepts_exactly(
        "function",
        &function_frost(),
        FrostType::FUNCTION,
        "Function",
    );
}

#[frost_fn(name = "describe_numeric")]
fn describe_numeric(x: Numeric) -> String {
    match x {
        Numeric::Int(i) => format!("Int {i}"),
        Numeric::Float(f) => format!("Float {}", f.get()),
    }
}

#[test]
fn numeric_keeps_an_int_distinct_from_a_float() {
    assert_eq!(run(describe_numeric_frost(), "f(2)"), Value::from("Int 2"));
    assert_eq!(
        run(describe_numeric_frost(), "f(2.0)"),
        Value::from("Float 2")
    );
}

#[test]
fn numeric_is_a_float_on_request() {
    assert_eq!(Numeric::Int(3).to_f64(), 3.0);
    let half = FrostFloat::new(0.5).expect("finite");
    assert_eq!(Numeric::Float(half).to_f64(), 0.5);
    assert_eq!(
        Numeric::Int(i64::MAX).to_f64(),
        9_223_372_036_854_775_807_f64,
        "rounded to the nearest float"
    );
}

#[test]
fn flat_gives_the_bytes_of_either() {
    assert_eq!(Flat::String("héllo".into()).as_bytes(), "héllo".as_bytes());
    assert_eq!(
        Flat::Bytes(FrostBytes::from(vec![0, 255])).as_bytes(),
        [0, 255]
    );
}

#[frost_fn(name = "describe_primitive")]
fn describe_primitive(x: Primitive) -> &'static str {
    match x {
        Primitive::Null => "Null",
        Primitive::Bool(_) => "Bool",
        Primitive::Int(_) => "Int",
        Primitive::Float(_) => "Float",
        Primitive::String(_) => "String",
        Primitive::Bytes(_) => "Bytes",
    }
}

#[frost_fn(name = "describe_structured")]
fn describe_structured(x: Structured) -> usize {
    match x {
        Structured::Array(a) => a.len(),
        Structured::Map(m) => m.len(),
    }
}

#[test]
fn an_enum_gives_each_accepted_type_its_variant() {
    for (literal, expected) in [
        ("null", "Null"),
        ("false", "Bool"),
        ("1", "Int"),
        ("1.0", "Float"),
        ("'a'", "String"),
        ("x'61'", "Bytes"),
    ] {
        assert_eq!(
            run(describe_primitive_frost(), &format!("f({literal})")),
            Value::from(expected)
        );
    }
    assert_eq!(run(describe_structured_frost(), "f([1, 2])"), Value::Int(2));
    assert_eq!(run(describe_structured_frost(), "f({a: 1})"), Value::Int(1));
}

#[frost_fn(name = "apply")]
fn apply(#[frost(ctx)] ctx: &mut NativeCtx<'_>, function: Function, value: Value) -> FrostResult {
    ctx.invoke(&function, [value])
}

#[test]
fn a_function_argument_is_called_through_its_value() {
    assert_eq!(run(apply_frost(), "f(fn n -> n * 2, 21)"), Value::Int(42));
    assert_eq!(
        raises(apply_frost(), "f(1, 2)"),
        "Function apply requires Function as argument 1 (function), got Int"
    );
}

/// One value of every Frost type.
fn one_value_of_each() -> Vec<Value> {
    vec![
        Value::Null,
        Value::Bool(true),
        Value::Int(1),
        Value::Float(FrostFloat::new(1.5).expect("finite")),
        Value::from("text"),
        Value::Bytes(FrostBytes::from(vec![0, 255])),
        Value::array([1]),
        Value::map([("a", Value::Int(1))]),
        Value::native("f", Arity::Exact(0), |_, _| Ok(Value::Null)),
        Value::opaque(SpecialFloat::new(f64::INFINITY).expect("special")),
    ]
}

/// Asserts that `T` converts from exactly the values of `types`, back into
/// the same value, and fails for any other naming `expected` and its type.
fn assert_converts_exactly<T>(types: TypeSet, expected: &str)
where
    T: TryFrom<Value, Error = FrostError> + Into<Value> + Debug,
{
    for value in one_value_of_each() {
        let ty = value.type_name();
        match T::try_from(value.clone()) {
            Ok(converted) => {
                assert!(types.contains(value.frost_type()), "{ty} is not {expected}");
                assert_eq!(converted.into(), value, "{ty} converts back unchanged");
            }
            Err(error) => {
                assert!(!types.contains(value.frost_type()), "{ty} is {expected}");
                assert_eq!(error.message(), format!("Expected {expected}, got {ty}"));
            }
        }
    }
}

#[test]
fn each_converts_from_exactly_its_category_s_values() {
    assert_converts_exactly::<Numeric>(FrostType::NUMERIC, "Numeric");
    assert_converts_exactly::<Flat>(FrostType::FLAT, "String or Bytes");
    assert_converts_exactly::<Structured>(FrostType::STRUCTURED, "Structured");
    assert_converts_exactly::<Primitive>(FrostType::PRIMITIVE, "Primitive");
    assert_converts_exactly::<Nonnull>(FrostType::NONNULL, "Nonnull");
    assert_converts_exactly::<Function>(FrostType::FUNCTION, "Function");
}

#[test]
fn each_converts_into_the_value_it_holds() {
    let half = FrostFloat::new(0.5).expect("finite");
    assert_eq!(Value::from(Numeric::Int(1)), Value::Int(1));
    assert_eq!(Value::from(Numeric::Float(half)), Value::Float(half));
    assert_eq!(Value::from(Flat::String("a".into())), Value::from("a"));
    assert_eq!(Value::from(Primitive::Null), Value::Null);
    assert_eq!(
        Primitive::Bool(true)
            .into_result("f")
            .expect("a Bool returns"),
        Value::Bool(true)
    );
}

// ============================================================
// Serde
// ============================================================

/// Asserts that `T` deserializes through `from_value` from exactly the values
/// of `types`, serializing back through `to_value` unchanged, and fails for any
/// other naming `expected` and its type.
fn assert_round_trips_exactly<T>(types: TypeSet, expected: &str)
where
    T: Serialize + DeserializeOwned + Debug,
{
    for value in one_value_of_each() {
        let ty = value.type_name();
        match from_value::<T>(value.clone()) {
            Ok(taken) => {
                assert!(types.contains(value.frost_type()), "{ty} is not {expected}");
                assert_eq!(
                    to_value(&taken).expect("a category value serializes"),
                    value,
                    "{ty} serializes back unchanged"
                );
            }
            Err(error) => {
                assert!(!types.contains(value.frost_type()), "{ty} is {expected}");
                assert_eq!(error.message(), format!("expected {expected}, got {ty}"));
            }
        }
    }
}

#[test]
fn each_round_trips_through_serde_exactly_its_category_s_values() {
    assert_round_trips_exactly::<Numeric>(FrostType::NUMERIC, "Numeric");
    assert_round_trips_exactly::<Flat>(FrostType::FLAT, "String or Bytes");
    assert_round_trips_exactly::<Structured>(FrostType::STRUCTURED, "Structured");
    assert_round_trips_exactly::<Primitive>(FrostType::PRIMITIVE, "Primitive");
    assert_round_trips_exactly::<Nonnull>(FrostType::NONNULL, "Nonnull");
    assert_round_trips_exactly::<Function>(FrostType::FUNCTION, "Function");
}

/// Options naming each kind of category field.
#[derive(Debug, Serialize, Deserialize)]
struct Shape {
    size: Numeric,
    label: Flat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    on_draw: Option<Function>,
}

#[test]
fn a_derived_struct_holds_category_fields() {
    let options = Value::map([("size", Value::Int(2)), ("label", Value::from("a"))]);
    let shape: Shape = from_value(options.clone()).expect("the options deserialize");
    assert!(matches!(shape.size, Numeric::Int(2)), "{shape:?}");
    assert_eq!(shape.label, Flat::String("a".into()));
    assert!(shape.on_draw.is_none(), "{shape:?}");
    assert_eq!(
        to_value(&shape).expect("the options serialize"),
        options,
        "the options serialize back unchanged"
    );
}

#[test]
fn a_category_field_of_another_type_fails_with_its_path() {
    let cases = [
        (
            Value::map([("size", Value::from("big")), ("label", Value::from("a"))]),
            "expected Numeric, got String (at `size`)",
        ),
        (
            Value::map([("size", Value::Int(2)), ("label", Value::array([1]))]),
            "expected String or Bytes, got Array (at `label`)",
        ),
        (
            Value::map([
                ("size", Value::Int(2)),
                ("label", Value::from("a")),
                ("on_draw", Value::Int(3)),
            ]),
            "expected Function, got Int (at `on_draw`)",
        ),
    ];
    for (options, message) in cases {
        let error = from_value::<Shape>(options).expect_err("the options are rejected");
        assert_eq!(error.message(), message);
    }
}

#[test]
fn a_category_works_with_other_serde_formats() {
    let shape: Shape =
        serde_json::from_str(r#"{"size": 2.5, "label": "a"}"#).expect("the JSON deserializes");
    assert!(
        matches!(shape.size, Numeric::Float(f) if f.get() == 2.5),
        "{shape:?}"
    );
    assert_eq!(
        serde_json::to_string(&shape).expect("the options serialize"),
        r#"{"size":2.5,"label":"a"}"#
    );
    let error = serde_json::from_str::<Shape>(r#"{"size": "big", "label": "a"}"#)
        .expect_err("a String size is rejected");
    assert!(
        error
            .to_string()
            .starts_with("expected Numeric, got String"),
        "{error}"
    );
}
