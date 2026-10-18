//! `std.math`: roots, powers, logarithms, trigonometry, rounding, and numeric
//! constants.
//!
//! Every function but `special_float` takes Int or Float arguments. One computed
//! in floating point returns a Float, and a result that would be NaN or infinite
//! is an error: a Float is always finite.

use std::cmp::Ordering;
use std::f64::consts;

use crate::native::{Numeric, Optional};
use crate::{
    FrostError, FrostFloat, FrostResult, FrostString, NativeCtx, SpecialFloat, StdlibModule, Value,
    frost_closure, frost_fn,
};

/// The `std.math` module: floating-point functions (roots, exponentials,
/// logarithms, trigonometric and hyperbolic functions), rounding to Int, `pow`,
/// `abs`, `min`, `max`, `clamp`, `lerp`, the numeric constants under `nums`, and
/// `special_float`, which makes a [`SpecialFloat`] from its name.
///
/// It only computes: it reads and changes nothing outside the script.
pub fn math() -> StdlibModule {
    StdlibModule::builder("math")
        .value("sqrt", float_function("std.math.sqrt", f64::sqrt))
        .value("cbrt", float_function("std.math.cbrt", f64::cbrt))
        .function(pow_frost())
        .value("exp", float_function("std.math.exp", f64::exp))
        .value("exp2", float_function("std.math.exp2", f64::exp2))
        .value("expm1", float_function("std.math.expm1", f64::exp_m1))
        .value("log", float_function("std.math.log", f64::ln))
        .value("log1p", float_function("std.math.log1p", f64::ln_1p))
        .value("log2", float_function("std.math.log2", f64::log2))
        .value("log10", float_function("std.math.log10", f64::log10))
        .value("sin", float_function("std.math.sin", f64::sin))
        .value("cos", float_function("std.math.cos", f64::cos))
        .value("tan", float_function("std.math.tan", f64::tan))
        .value("asin", float_function("std.math.asin", f64::asin))
        .value("acos", float_function("std.math.acos", f64::acos))
        .value("atan", float_function("std.math.atan", f64::atan))
        .function(atan2_frost())
        .value("sinh", float_function("std.math.sinh", f64::sinh))
        .value("cosh", float_function("std.math.cosh", f64::cosh))
        .value("tanh", float_function("std.math.tanh", f64::tanh))
        .value("asinh", float_function("std.math.asinh", f64::asinh))
        .value("acosh", float_function("std.math.acosh", f64::acosh))
        .value("atanh", float_function("std.math.atanh", f64::atanh))
        .value("round", rounding_function("std.math.round", f64::round))
        .value("ceil", rounding_function("std.math.ceil", f64::ceil))
        .value("floor", rounding_function("std.math.floor", f64::floor))
        .value("trunc", rounding_function("std.math.trunc", f64::trunc))
        .function(abs_frost())
        .value("min", min_max("std.math.min", Ordering::Less))
        .value("max", min_max("std.math.max", Ordering::Greater))
        .function(hypot_frost())
        .function(clamp_frost())
        .function(lerp_frost())
        .function(special_float_frost())
        .value("nums", nums())
        .build()
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

#[frost_fn]
fn atan2(#[frost(ctx)] ctx: &mut NativeCtx<'_>, y: Numeric, x: Numeric) -> FrostResult {
    finite(ctx.name(), y.to_f64().atan2(x.to_f64()), &[y, x])
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

#[frost_fn]
fn abs(#[frost(ctx)] ctx: &mut NativeCtx<'_>, #[frost(unnamed)] x: Numeric) -> FrostResult {
    let name = ctx.name();
    match x {
        Numeric::Int(i) => i.checked_abs().map(Value::Int).ok_or_else(|| {
            FrostError::from_string(format!("Function {name} has no Int result for {i}"))
        }),
        Numeric::Float(f) => finite(name, f.get().abs(), &[x]),
    }
}

/// `pow`: an Int of an Int base and a non-negative Int exponent, else a Float.
#[frost_fn]
fn pow(#[frost(ctx)] ctx: &mut NativeCtx<'_>, base: Numeric, exponent: Numeric) -> FrostResult {
    let name = ctx.name();
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
                    "Function {name} has no Int result for {base}, {exponent}"
                ))
            })
        }
        _ => finite(
            name,
            base.to_f64().powf(exponent.to_f64()),
            &[base, exponent],
        ),
    }
}

/// `min` or `max`: an Int of two Ints, else a Float. The second argument wins only by
/// ordering `wins` against the first, so of two numbers of equal value, such as `0.0`
/// and `-0.0`, the first is returned.
fn min_max(name: &'static str, wins: Ordering) -> Value {
    frost_closure!(
        name,
        move |#[frost(unnamed)] a: Numeric, #[frost(unnamed)] b: Numeric| match (a, b) {
            (Numeric::Int(x), Numeric::Int(y)) => {
                Ok(Value::Int(if y.cmp(&x) == wins { y } else { x }))
            }
            _ => {
                let (x, y) = (a.to_f64(), b.to_f64());
                let chosen = if y.partial_cmp(&x) == Some(wins) {
                    y
                } else {
                    x
                };
                finite(name, chosen, &[a, b])
            }
        }
    )
}

#[frost_fn]
fn hypot(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    #[frost(unnamed)] a: Numeric,
    #[frost(unnamed)] b: Numeric,
    #[frost(unnamed)] c: Optional<Numeric>,
) -> FrostResult {
    let norm = a.to_f64().hypot(b.to_f64());
    match c {
        Optional(None) => finite(ctx.name(), norm, &[a, b]),
        Optional(Some(c)) => finite(ctx.name(), norm.hypot(c.to_f64()), &[a, b, c]),
    }
}

#[frost_fn]
fn clamp(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: Numeric,
    lo: Numeric,
    hi: Numeric,
) -> FrostResult {
    let name = ctx.name();
    let (lo_value, hi_value) = (Value::from(lo), Value::from(hi));
    if lo_value.compare(&hi_value)?.is_gt() {
        return Err(FrostError::from_string(format!(
            "Function {name} requires argument 2 (lo) to be at most argument 3 (hi), \
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
            name,
            value.to_f64().clamp(lo.to_f64(), hi.to_f64()),
            &[value, lo, hi],
        ),
    }
}

#[frost_fn]
fn lerp(#[frost(ctx)] ctx: &mut NativeCtx<'_>, a: Numeric, b: Numeric, t: Numeric) -> FrostResult {
    let [a_float, b_float, t_float] = [a, b, t].map(Numeric::to_f64);
    // Weighting both ends, rather than `a + (b - a) * t`, makes `t` of 0 and
    // 1 give exactly `a` and `b`.
    finite(
        ctx.name(),
        (1.0 - t_float) * a_float + t_float * b_float,
        &[a, b, t],
    )
}

/// The [`SpecialFloat`] Opaque that a String names as Rust parses floats (`NaN`,
/// `inf`, `-infinity`, in any case), or Null for any other String.
#[frost_fn]
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
