//! `std.math`: roots, powers, logarithms, trigonometry, rounding, and numeric
//! constants.
//!
//! Every function but `special_float` takes Int or Float arguments. One computed
//! in floating point returns a Float, and a result that would be NaN or infinite
//! is an error: a Float is always finite.

use std::f64::consts;

use crate::native::{Numeric, Optional};
use crate::{
    FrostError, FrostFloat, FrostResult, FrostString, SpecialFloat, StdlibModule, Value,
    frost_closure, frost_fn,
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
            ("pow", pow_frost()),
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
            ("abs", abs_frost()),
            ("min", min_max("math.min", i64::min, f64::min)),
            ("max", min_max("math.max", i64::max, f64::max)),
            ("hypot", hypot_frost()),
            ("clamp", clamp_frost()),
            ("lerp", lerp_frost()),
            ("special_float", special_float_frost()),
            ("nums", nums()),
        ]),
    )
}

/// `result`, which `function` computed from `args`, as a Float, if it is finite.
fn finite(function: &str, result: f64, args: &[Numeric]) -> FrostResult {
    FrostFloat::new(result).map(Value::Float).map_err(|_| {
        let args: Vec<String> = args
            .iter()
            .map(|&arg| Value::from(arg).to_frost_string())
            .collect();
        FrostError::from_string(format!(
            "Function {function} has no finite result for {}",
            args.join(", ")
        ))
    })
}

/// A one-argument function computed in floating point.
fn float_function(name: &'static str, f: fn(f64) -> f64) -> Value {
    frost_closure!(name, move |#[frost(unnamed)] x: Numeric| {
        finite(name, f(x.to_f64()), &[x])
    })
}

/// A two-argument function computed in floating point.
fn binary_float_function(name: &'static str, f: fn(f64, f64) -> f64) -> Value {
    frost_closure!(
        name,
        move |#[frost(unnamed)] x: Numeric, #[frost(unnamed)] y: Numeric| {
            finite(name, f(x.to_f64(), y.to_f64()), &[x, y])
        }
    )
}

/// A function rounding to an Int by `round`; an Int is already rounded.
fn rounding_function(name: &'static str, round: fn(f64) -> f64) -> Value {
    frost_closure!(name, move |#[frost(unnamed)] x: Numeric| match x {
        Numeric::Int(i) => Ok(Value::Int(i)),
        Numeric::Float(f) => {
            let rounded = FrostFloat::new(round(f.get())).expect("rounding keeps a Float finite");
            rounded.to_exact_int().map(Value::Int).ok_or_else(|| {
                FrostError::from_string(format!(
                    "Function {name} has no Int result for {}",
                    Value::Float(f).to_frost_string()
                ))
            })
        }
    })
}

#[frost_fn(name = "math.abs")]
fn abs(#[frost(unnamed)] x: Numeric) -> FrostResult {
    match x {
        Numeric::Int(i) => i.checked_abs().map(Value::Int).ok_or_else(|| {
            FrostError::from_string(format!("Function math.abs has no Int result for {i}"))
        }),
        Numeric::Float(f) => finite("math.abs", f.get().abs(), &[x]),
    }
}

/// `pow`: an Int of an Int base and a non-negative Int exponent, else a Float.
#[frost_fn(name = "math.pow")]
fn pow(#[frost(unnamed)] base: Numeric, #[frost(unnamed)] exponent: Numeric) -> FrostResult {
    match (base, exponent) {
        (Numeric::Int(base), Numeric::Int(exponent)) if exponent >= 0 => {
            let result = match u32::try_from(exponent) {
                Ok(exponent) => base.checked_pow(exponent),
                // Past `u32`, every base but 0, 1, and -1 overflows.
                Err(_) => match base {
                    0 | 1 => Some(base),
                    -1 => Some(if exponent % 2 == 0 { 1 } else { -1 }),
                    _ => None,
                },
            };
            result.map(Value::Int).ok_or_else(|| {
                FrostError::from_string(format!(
                    "Function math.pow has no Int result for {base}, {exponent}"
                ))
            })
        }
        _ => finite(
            "math.pow",
            base.to_f64().powf(exponent.to_f64()),
            &[base, exponent],
        ),
    }
}

/// `min` or `max`: an Int of two Ints, else a Float.
fn min_max(name: &'static str, ints: fn(i64, i64) -> i64, floats: fn(f64, f64) -> f64) -> Value {
    frost_closure!(
        name,
        move |#[frost(unnamed)] a: Numeric, #[frost(unnamed)] b: Numeric| match (a, b) {
            (Numeric::Int(a), Numeric::Int(b)) => Ok(Value::Int(ints(a, b))),
            _ => finite(name, floats(a.to_f64(), b.to_f64()), &[a, b]),
        }
    )
}

#[frost_fn(name = "math.hypot")]
fn hypot(
    #[frost(unnamed)] a: Numeric,
    #[frost(unnamed)] b: Numeric,
    #[frost(unnamed)] c: Optional<Numeric>,
) -> FrostResult {
    let norm = a.to_f64().hypot(b.to_f64());
    match c {
        Optional(None) => finite("math.hypot", norm, &[a, b]),
        Optional(Some(c)) => finite("math.hypot", norm.hypot(c.to_f64()), &[a, b, c]),
    }
}

#[frost_fn(name = "math.clamp")]
fn clamp(value: Numeric, lo: Numeric, hi: Numeric) -> FrostResult {
    let (lo_value, hi_value) = (Value::from(lo), Value::from(hi));
    if lo_value.compare(&hi_value)?.is_gt() {
        return Err(FrostError::from_string(format!(
            "Function math.clamp requires argument 2 (lo) to be at most argument 3 (hi), \
             got {} and {}",
            lo_value.to_frost_string(),
            hi_value.to_frost_string()
        )));
    }
    match (value, lo, hi) {
        (Numeric::Int(value), Numeric::Int(lo), Numeric::Int(hi)) => {
            Ok(Value::Int(value.clamp(lo, hi)))
        }
        _ => finite(
            "math.clamp",
            value.to_f64().clamp(lo.to_f64(), hi.to_f64()),
            &[value, lo, hi],
        ),
    }
}

#[frost_fn(name = "math.lerp")]
fn lerp(a: Numeric, b: Numeric, t: Numeric) -> FrostResult {
    let [a_float, b_float, t_float] = [a, b, t].map(Numeric::to_f64);
    // Weighting both ends, rather than `a + (b - a) * t`, makes `t` of 0 and
    // 1 give exactly `a` and `b`.
    finite(
        "math.lerp",
        (1.0 - t_float) * a_float + t_float * b_float,
        &[a, b, t],
    )
}

/// The [`SpecialFloat`] Opaque that a String names as Rust parses floats (`NaN`,
/// `inf`, `-infinity`, in any case), or Null for any other String.
#[frost_fn(name = "math.special_float")]
fn special_float(#[frost(unnamed)] text: FrostString) -> Value {
    // A name has no digits; a numeral such as `1e999` parses as infinite too.
    let is_name = !text.contains(|c: char| c.is_ascii_digit());
    let special = text.parse::<f64>().ok().and_then(SpecialFloat::new);
    special
        .filter(|_| is_name)
        .map_or(Value::Null, Value::opaque)
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
