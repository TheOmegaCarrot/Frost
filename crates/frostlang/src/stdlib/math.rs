//! `std.math`: roots, powers, logarithms, trigonometry, rounding, and numeric
//! constants.
//!
//! Every function but `special_float` takes Int or Float arguments. One computed
//! in floating point returns a Float, and a result that would be NaN or infinite
//! is an error: a Float is always finite.

use std::f64::consts;

use crate::{
    FrostError, FrostFloat, FrostResult, FrostType, Param, Params, SpecialFloat, StdlibModule,
    Value,
};

/// The `std.math` module: floating-point functions (roots, exponentials,
/// logarithms, trigonometric and hyperbolic functions), rounding to Int, `pow`,
/// `abs`, `min`, `max`, `clamp`, `lerp`, the numeric constants under `nums`, and
/// `special_float`, which makes a [`SpecialFloat`] from its name.
///
/// It only computes: it reads and changes nothing outside the script.
pub fn math() -> StdlibModule {
    StdlibModule::new(
        "math",
        Value::map([
            ("sqrt", float_function("math.sqrt", f64::sqrt)),
            ("cbrt", float_function("math.cbrt", f64::cbrt)),
            ("pow", pow()),
            ("exp", float_function("math.exp", f64::exp)),
            ("exp2", float_function("math.exp2", f64::exp2)),
            ("expm1", float_function("math.expm1", f64::exp_m1)),
            ("log", float_function("math.log", f64::ln)),
            ("log1p", float_function("math.log1p", f64::ln_1p)),
            ("log2", float_function("math.log2", f64::log2)),
            ("log10", float_function("math.log10", f64::log10)),
            ("sin", float_function("math.sin", f64::sin)),
            ("cos", float_function("math.cos", f64::cos)),
            ("tan", float_function("math.tan", f64::tan)),
            ("asin", float_function("math.asin", f64::asin)),
            ("acos", float_function("math.acos", f64::acos)),
            ("atan", float_function("math.atan", f64::atan)),
            ("atan2", binary_float_function("math.atan2", f64::atan2)),
            ("sinh", float_function("math.sinh", f64::sinh)),
            ("cosh", float_function("math.cosh", f64::cosh)),
            ("tanh", float_function("math.tanh", f64::tanh)),
            ("asinh", float_function("math.asinh", f64::asinh)),
            ("acosh", float_function("math.acosh", f64::acosh)),
            ("atanh", float_function("math.atanh", f64::atanh)),
            ("round", rounding_function("math.round", f64::round)),
            ("ceil", rounding_function("math.ceil", f64::ceil)),
            ("floor", rounding_function("math.floor", f64::floor)),
            ("trunc", rounding_function("math.trunc", f64::trunc)),
            ("abs", abs()),
            ("min", min_max("math.min", i64::min, f64::min)),
            ("max", min_max("math.max", i64::max, f64::max)),
            ("hypot", hypot()),
            ("clamp", clamp()),
            ("lerp", lerp()),
            ("special_float", special_float()),
            ("nums", nums()),
        ]),
    )
}

const ONE_NUMBER: Params = Params::new(&[Param::of(FrostType::NUMERIC)]);
const TWO_NUMBERS: Params =
    Params::new(&[Param::of(FrostType::NUMERIC), Param::of(FrostType::NUMERIC)]);

/// A type-checked Numeric argument, as a float.
fn float_arg(arg: &Value) -> f64 {
    match arg {
        Value::Int(i) => *i as f64,
        Value::Float(f) => f.get(),
        other => unreachable!("type-checked as Numeric, got {}", other.type_name()),
    }
}

/// `result`, which `function` computed from `args`, as a Float, if it is finite.
fn finite(function: &str, result: f64, args: &[Value]) -> FrostResult {
    FrostFloat::new(result).map(Value::Float).map_err(|_| {
        let args: Vec<String> = args.iter().map(Value::to_frost_string).collect();
        FrostError::from_string(format!(
            "Function {function} has no finite result for {}",
            args.join(", ")
        ))
    })
}

/// A one-argument function computed in floating point.
fn float_function(name: &'static str, f: fn(f64) -> f64) -> Value {
    Value::checked_native(name, ONE_NUMBER, move |_, args| {
        finite(name, f(float_arg(&args[0])), args)
    })
}

/// A two-argument function computed in floating point.
fn binary_float_function(name: &'static str, f: fn(f64, f64) -> f64) -> Value {
    Value::checked_native(name, TWO_NUMBERS, move |_, args| {
        finite(name, f(float_arg(&args[0]), float_arg(&args[1])), args)
    })
}

/// A function rounding to an Int by `round`; an Int is already rounded.
fn rounding_function(name: &'static str, round: fn(f64) -> f64) -> Value {
    Value::checked_native(name, ONE_NUMBER, move |_, args| match &args[0] {
        Value::Int(_) => Ok(args[0].take()),
        Value::Float(f) => {
            let rounded = FrostFloat::new(round(f.get())).expect("rounding keeps a Float finite");
            rounded.to_exact_int().map(Value::Int).ok_or_else(|| {
                FrostError::from_string(format!(
                    "Function {name} has no Int result for {}",
                    args[0].to_frost_string()
                ))
            })
        }
        other => unreachable!("type-checked as Numeric, got {}", other.type_name()),
    })
}

fn abs() -> Value {
    Value::checked_native("math.abs", ONE_NUMBER, |_, args| match &args[0] {
        Value::Int(i) => i.checked_abs().map(Value::Int).ok_or_else(|| {
            FrostError::from_string(format!("Function math.abs has no Int result for {i}"))
        }),
        Value::Float(f) => finite("math.abs", f.get().abs(), args),
        other => unreachable!("type-checked as Numeric, got {}", other.type_name()),
    })
}

/// `pow`: an Int of an Int base and a non-negative Int exponent, else a Float.
fn pow() -> Value {
    Value::checked_native("math.pow", TWO_NUMBERS, |_, args| {
        match (&args[0], &args[1]) {
            (Value::Int(base), Value::Int(exponent)) if *exponent >= 0 => {
                let result = match u32::try_from(*exponent) {
                    Ok(exponent) => base.checked_pow(exponent),
                    // Past `u32`, every base but 0, 1, and -1 overflows.
                    Err(_) => match base {
                        0 | 1 => Some(*base),
                        -1 => Some(if exponent % 2 == 0 { 1 } else { -1 }),
                        _ => None,
                    },
                };
                result.map(Value::Int).ok_or_else(|| {
                    FrostError::from_string(format!(
                        "Function math.pow has no Int result for {}, {}",
                        args[0].to_frost_string(),
                        args[1].to_frost_string()
                    ))
                })
            }
            (base, exponent) => finite("math.pow", float_arg(base).powf(float_arg(exponent)), args),
        }
    })
}

/// `min` or `max`: an Int of two Ints, else a Float.
fn min_max(name: &'static str, ints: fn(i64, i64) -> i64, floats: fn(f64, f64) -> f64) -> Value {
    Value::checked_native(name, TWO_NUMBERS, move |_, args| {
        match (&args[0], &args[1]) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(ints(*a, *b))),
            (a, b) => finite(name, floats(float_arg(a), float_arg(b)), args),
        }
    })
}

fn hypot() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::NUMERIC),
        Param::of(FrostType::NUMERIC),
        Param::of(FrostType::NUMERIC).optional(),
    ]);
    Value::checked_native("math.hypot", PARAMS, |_, args| {
        let norm = args
            .iter()
            .map(float_arg)
            .reduce(f64::hypot)
            .expect("hypot has at least two arguments");
        finite("math.hypot", norm, args)
    })
}

fn clamp() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::NUMERIC).named("value"),
        Param::of(FrostType::NUMERIC).named("lo"),
        Param::of(FrostType::NUMERIC).named("hi"),
    ]);
    Value::checked_native("math.clamp", PARAMS, |_, args| {
        if args[1].compare(&args[2])?.is_gt() {
            return Err(FrostError::from_string(format!(
                "Function math.clamp requires argument 2 (lo) to be at most argument 3 (hi), \
                 got {} and {}",
                args[1].to_frost_string(),
                args[2].to_frost_string()
            )));
        }
        match (&args[0], &args[1], &args[2]) {
            (Value::Int(value), Value::Int(lo), Value::Int(hi)) => {
                Ok(Value::Int((*value).clamp(*lo, *hi)))
            }
            (value, lo, hi) => finite(
                "math.clamp",
                float_arg(value).clamp(float_arg(lo), float_arg(hi)),
                args,
            ),
        }
    })
}

fn lerp() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::NUMERIC).named("a"),
        Param::of(FrostType::NUMERIC).named("b"),
        Param::of(FrostType::NUMERIC).named("t"),
    ]);
    Value::checked_native("math.lerp", PARAMS, |_, args| {
        let [a, b, t] = [&args[0], &args[1], &args[2]].map(float_arg);
        // Weighting both ends, rather than `a + (b - a) * t`, makes `t` of 0 and
        // 1 give exactly `a` and `b`.
        finite("math.lerp", (1.0 - t) * a + t * b, args)
    })
}

/// The [`SpecialFloat`] Opaque that a String names as Rust parses floats (`NaN`,
/// `inf`, `-infinity`, in any case), or Null for any other String.
fn special_float() -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::STRING)]);
    Value::checked_native("math.special_float", PARAMS, |_, args| {
        let text = args[0].as_str().expect("type-checked as String");
        // A name has no digits; a numeral such as `1e999` parses as infinite too.
        let is_name = !text.contains(|c: char| c.is_ascii_digit());
        let special = text.parse::<f64>().ok().and_then(SpecialFloat::new);
        Ok(special
            .filter(|_| is_name)
            .map_or(Value::Null, Value::opaque))
    })
}

/// The constants under `math.nums`.
fn nums() -> Value {
    let float = |f: f64| Value::Float(FrostFloat::new(f).expect("a numeric constant is finite"));
    Value::map([
        ("pi", float(consts::PI)),
        ("tau", float(consts::TAU)),
        ("e", float(consts::E)),
        // (1 + sqrt 5) / 2, to the nearest float.
        ("golden_ratio", float(1.618_033_988_749_895)),
        ("maxint", Value::Int(i64::MAX)),
        ("minint", Value::Int(i64::MIN)),
        ("maxfloat", float(f64::MAX)),
        ("minfloat", float(f64::MIN)),
        ("tinyfloat", float(f64::MIN_POSITIVE)),
        ("float_epsilon", float(f64::EPSILON)),
    ])
}
