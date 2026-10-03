//! `std.math`, from Frost source.
//!
//! Each case runs with only `std.math` installed, bound as `math`. Every function
//! but `special_float` takes Int or Float arguments. One computed in floating
//! point returns a Float, and a result that would be NaN or infinite is an error.

mod source;

use frost_runtime::stdlib::RandomConfig;
use frost_runtime::{FrostFloat, ImporterBuilder, SpecialFloat, Stdlib, Value, stdlib};
use source::Script;
use source::assertions::{Library, library_assertions};

library_assertions!(Library::module(stdlib::math, "math"));

fn run(expression: &str) -> Value {
    script(expression).run()
}

fn float(f: f64) -> Value {
    Value::Float(FrostFloat::new(f).expect("a finite float"))
}

/// A one-argument floating-point function, the `f64` method it computes, and a
/// Float and an Int argument it is defined for.
type Unary = (&'static str, fn(f64) -> f64, f64, i64);

const UNARY: &[Unary] = &[
    ("sqrt", f64::sqrt, 0.5, 2),
    ("cbrt", f64::cbrt, -0.5, -27),
    ("exp", f64::exp, 0.5, 2),
    ("exp2", f64::exp2, 0.5, 10),
    ("expm1", f64::exp_m1, 0.5, 2),
    ("log", f64::ln, 0.5, 2),
    ("log1p", f64::ln_1p, 0.5, 2),
    ("log2", f64::log2, 0.5, 8),
    ("log10", f64::log10, 0.5, 1000),
    ("sin", f64::sin, 0.5, 2),
    ("cos", f64::cos, 0.5, 2),
    ("tan", f64::tan, 0.5, 2),
    ("asin", f64::asin, 0.5, 1),
    ("acos", f64::acos, 0.5, -1),
    ("atan", f64::atan, 0.5, 2),
    ("sinh", f64::sinh, 0.5, 2),
    ("cosh", f64::cosh, 0.5, 2),
    ("tanh", f64::tanh, 0.5, 2),
    ("asinh", f64::asinh, 0.5, 2),
    ("acosh", f64::acosh, 1.5, 2),
    ("atanh", f64::atanh, 0.5, 0),
];

// --- The module ---

#[test]
fn the_module_holds_its_functions_and_constants() {
    assert_values(&[
        (
            "sorted(keys(math))",
            "['abs', 'acos', 'acosh', 'asin', 'asinh', 'atan', 'atan2', 'atanh', 'cbrt', \
             'ceil', 'clamp', 'cos', 'cosh', 'exp', 'exp2', 'expm1', 'floor', 'hypot', \
             'lerp', 'log', 'log10', 'log1p', 'log2', 'max', 'min', 'nums', 'pow', 'round', \
             'sin', 'sinh', 'special_float', 'sqrt', 'tan', 'tanh', 'trunc']",
        ),
        (
            "sorted(keys(math.nums))",
            "['e', 'float_epsilon', 'golden_ratio', 'maxfloat', 'maxint', 'minfloat', \
             'minint', 'pi', 'tau', 'tinyfloat']",
        ),
    ]);
}

#[test]
fn the_module_is_contained() {
    let stdlib = Stdlib::contained(RandomConfig::default());
    let contained = ImporterBuilder::new().with_stdlib(stdlib).build();
    let result = Script::new("import('std.math').sqrt(4)")
        .importer(contained)
        .run();
    assert_eq!(result, float(2.0));
}

// --- Floating-point functions ---

#[test]
fn each_float_function_computes_its_f64_method() {
    for &(name, f, x, n) in UNARY {
        assert_eq!(
            run(&format!("math.{name}({x:?})")),
            float(f(x)),
            "{name}({x})"
        );
        assert_eq!(
            run(&format!("math.{name}({n})")),
            float(f(n as f64)),
            "{name}({n})"
        );
    }
}

#[test]
fn float_functions_return_a_float_for_an_int() {
    // Only IEEE-exact operations are compared exactly; the rest are checked
    // against their `f64` methods in `each_float_function_computes_its_f64_method`.
    assert_values(&[
        ("math.sqrt(4)", "2.0"),
        ("math.exp2(10)", "1024.0"),
        ("math.cos(0)", "1.0"),
    ]);
    for name in ["cbrt", "log2", "log10", "acosh"] {
        assert_values(&[(&format!("is_float(math.{name}(8))"), "true")]);
    }
}

#[test]
fn pow_and_atan2_compute_their_f64_methods() {
    for (a, b) in [
        (2.0, 0.5),
        (2.0, 10.0),
        (-8.0, 3.0),
        (0.0, 0.0),
        (1.5, -2.0),
    ] {
        assert_eq!(
            run(&format!("math.pow({a:?}, {b:?})")),
            float(f64::powf(a, b)),
            "pow({a}, {b})"
        );
        assert_eq!(
            run(&format!("math.atan2({a:?}, {b:?})")),
            float(f64::atan2(a, b)),
            "atan2({a}, {b})"
        );
    }
    assert_values(&[
        ("math.pow(2, 10)", "1024.0"),
        ("math.atan2(0, -1)", "math.nums.pi"),
    ]);
}

#[test]
fn a_result_that_is_not_finite_is_an_error() {
    for (call, args) in [
        ("sqrt(-1)", "-1"),
        ("log(0)", "0"),
        ("log(-1)", "-1"),
        ("log1p(-1)", "-1"),
        ("log2(0)", "0"),
        ("log10(-1)", "-1"),
        ("asin(2)", "2"),
        ("acos(-2)", "-2"),
        ("acosh(0.5)", "0.5"),
        ("atanh(1)", "1"),
        ("atanh(2)", "2"),
        ("exp(1000)", "1000"),
        ("exp2(2000)", "2000"),
        ("expm1(1000)", "1000"),
        ("sinh(1000)", "1000"),
        ("cosh(-1000)", "-1000"),
        ("pow(10, 1000)", "10, 1000"),
        ("pow(0, -1)", "0, -1"),
        ("pow(-8, 0.5)", "-8, 0.5"),
        (
            "hypot(math.nums.maxfloat, math.nums.maxfloat)",
            "math.nums.maxfloat, math.nums.maxfloat",
        ),
        ("lerp(0, 1e308, 10)", "0, 1e308, 10"),
    ] {
        let name = call.split('(').next().expect("a call has a name");
        let rendered: Vec<String> = args
            .split(", ")
            .map(|arg| run(arg).to_frost_string())
            .collect();
        assert_raises(&[(
            &format!("math.{call}"),
            &format!(
                "Function math.{name} has no finite result for {}",
                rendered.join(", ")
            ),
        )]);
    }
}

// --- Rounding ---

#[test]
fn rounding_functions_return_an_int() {
    assert_values(&[
        ("math.round(2.4)", "2"),
        ("math.round(2.5)", "3"),
        // Halves round away from zero.
        ("math.round(-2.5)", "-3"),
        ("math.round(-2.4)", "-2"),
        ("math.ceil(2.1)", "3"),
        ("math.ceil(-2.1)", "-2"),
        ("math.floor(2.9)", "2"),
        ("math.floor(-2.1)", "-3"),
        ("math.trunc(2.7)", "2"),
        ("math.trunc(-2.7)", "-2"),
        ("math.round(-0.4)", "0"),
        ("is_int(math.floor(1.0))", "true"),
    ]);
}

#[test]
fn rounding_returns_an_int_unchanged() {
    for function in ["round", "ceil", "floor", "trunc"] {
        assert_values(&[
            (&format!("math.{function}(7)"), "7"),
            (
                &format!("math.{function}(math.nums.maxint)"),
                "math.nums.maxint",
            ),
            (
                &format!("math.{function}(math.nums.minint)"),
                "math.nums.minint",
            ),
        ]);
    }
}

#[test]
fn rounding_past_the_int_range_is_an_error() {
    let max = run("math.nums.maxfloat").to_frost_string();
    let min = run("math.nums.minfloat").to_frost_string();
    for function in ["round", "ceil", "floor", "trunc"] {
        assert_raises(&[
            (
                &format!("math.{function}(math.nums.maxfloat)"),
                &format!("Function math.{function} has no Int result for {max}"),
            ),
            (
                &format!("math.{function}(math.nums.minfloat)"),
                &format!("Function math.{function} has no Int result for {min}"),
            ),
        ]);
    }
    // 2^63 is one past the largest Int; -2^63 is the smallest. The largest Float
    // below 2^63 is 2^63 - 1024.
    let past = float(9_223_372_036_854_775_808.0).to_frost_string();
    for function in ["round", "ceil", "floor", "trunc"] {
        assert_values(&[
            (
                &format!("math.{function}(9223372036854774784.0)"),
                "9223372036854774784",
            ),
            (
                &format!("math.{function}(-9223372036854775808.0)"),
                "math.nums.minint",
            ),
        ]);
        assert_raises(&[(
            &format!("math.{function}(9223372036854775808.0)"),
            &format!("Function math.{function} has no Int result for {past}"),
        )]);
    }
}

// --- abs, min, max, hypot, clamp, lerp ---

#[test]
fn abs_keeps_the_type() {
    assert_values(&[
        ("math.abs(-5)", "5"),
        ("math.abs(5)", "5"),
        ("math.abs(0)", "0"),
        ("math.abs(-2.5)", "2.5"),
        ("math.abs(math.nums.maxint)", "math.nums.maxint"),
    ]);
    assert_raises(&[(
        "math.abs(math.nums.minint)",
        "Function math.abs has no Int result for -9223372036854775808",
    )]);
}

#[test]
fn min_and_max_keep_an_int_of_two_ints() {
    assert_values(&[
        ("math.min(1, 2)", "1"),
        ("math.max(1, 2)", "2"),
        ("math.min(-3, -4)", "-4"),
        ("math.min(1, 2.5)", "1.0"),
        ("math.max(1.5, 2)", "2.0"),
        ("math.max(2.5, 1.5)", "2.5"),
        ("math.min(2, 2.0)", "2.0"),
    ]);
}

#[test]
fn hypot_takes_two_or_three_lengths() {
    assert_values(&[
        ("math.hypot(3, 4)", "5.0"),
        ("math.hypot(0, 0)", "0.0"),
        ("math.hypot(-3, 4)", "5.0"),
    ]);
    // The three-length norm is not exactly representable on every platform.
    assert_eq!(
        run("math.hypot(2, 3, 6)"),
        float(f64::hypot(f64::hypot(2.0, 3.0), 6.0))
    );
}

#[test]
fn clamp_keeps_a_value_within_bounds() {
    assert_values(&[
        ("math.clamp(5, 0, 3)", "3"),
        ("math.clamp(-1, 0, 3)", "0"),
        ("math.clamp(2, 0, 3)", "2"),
        ("math.clamp(2, 2, 2)", "2"),
        ("math.clamp(2.5, 0, 3)", "2.5"),
        ("math.clamp(5, 0, 3.5)", "3.5"),
        // Any Float argument makes the result a Float.
        ("math.clamp(1, 0, 3.0)", "1.0"),
    ]);
    assert_raises(&[(
        "math.clamp(1, 3, 0)",
        "Function math.clamp requires argument 2 (lo) to be at most argument 3 (hi), got 3 and 0",
    )]);
}

#[test]
fn clamp_compares_int_bounds_exactly() {
    // As Floats, both bounds would round to 2^63 and seem equal.
    assert_values(&[(
        "math.clamp(0, 9223372036854775806, 9223372036854775807)",
        "9223372036854775806",
    )]);
    assert_raises(&[(
        "math.clamp(0, 9223372036854775807, 9223372036854775806)",
        "Function math.clamp requires argument 2 (lo) to be at most argument 3 (hi), \
         got 9223372036854775807 and 9223372036854775806",
    )]);
}

#[test]
fn lerp_interpolates_and_extrapolates() {
    assert_values(&[
        ("math.lerp(0, 10, 0.5)", "5.0"),
        ("math.lerp(0, 10, 0)", "0.0"),
        ("math.lerp(0, 10, 1)", "10.0"),
        ("math.lerp(0, 10, 2)", "20.0"),
        ("math.lerp(0, 10, -0.5)", "-5.0"),
        ("math.lerp(10, 0, 0.25)", "7.5"),
        // The ends are exact.
        ("math.lerp(0.1, 0.7, 0)", "0.1"),
        ("math.lerp(0.1, 0.7, 1)", "0.7"),
    ]);
}

// --- nums ---

#[test]
fn nums_holds_the_constants() {
    for (name, expected) in [
        ("pi", float(std::f64::consts::PI)),
        ("tau", float(std::f64::consts::TAU)),
        ("e", float(std::f64::consts::E)),
        ("golden_ratio", float((1.0 + 5f64.sqrt()) / 2.0)),
        ("maxint", Value::Int(i64::MAX)),
        ("minint", Value::Int(i64::MIN)),
        ("maxfloat", float(f64::MAX)),
        ("minfloat", float(f64::MIN)),
        ("tinyfloat", float(f64::MIN_POSITIVE)),
        ("float_epsilon", float(f64::EPSILON)),
    ] {
        assert_eq!(run(&format!("math.nums.{name}")), expected, "{name}");
    }
}

// --- special_float ---

#[test]
fn special_float_makes_the_float_its_argument_spells() {
    // Any spelling Rust parses as a non-finite float.
    for (text, expected) in [
        ("NaN", f64::NAN),
        ("nan", f64::NAN),
        ("inf", f64::INFINITY),
        ("+inf", f64::INFINITY),
        ("Infinity", f64::INFINITY),
        ("-inf", f64::NEG_INFINITY),
        ("-INFINITY", f64::NEG_INFINITY),
    ] {
        let made = run(&format!("math.special_float('{text}')"));
        let held = made
            .downcast_opaque::<SpecialFloat>()
            .unwrap_or_else(|| panic!("{text:?} made {made:?}"))
            .get();
        if expected.is_nan() {
            assert!(held.is_nan(), "{text:?} made {held}");
        } else {
            assert_eq!(held, expected, "{text:?}");
        }
    }
}

#[test]
fn special_float_returns_null_for_any_other_string() {
    for text in ["1.5", "0", "1e999", "", "infinite", " inf", "nan!", "none"] {
        assert_eq!(
            run(&format!("math.special_float('{text}')")),
            Value::Null,
            "{text:?}"
        );
    }
}

#[test]
fn special_float_makes_an_opaque_equal_to_one_spelled_the_same() {
    assert_values(&[
        ("is_opaque(math.special_float('inf'))", "true"),
        ("type(math.special_float('inf'))", "'Opaque'"),
        (
            "math.special_float('inf') == math.special_float('Infinity')",
            "true",
        ),
        (
            "math.special_float('inf') == math.special_float('-inf')",
            "false",
        ),
        (
            "to_string(math.special_float('-inf'))",
            "'<SpecialFloat: -inf>'",
        ),
    ]);
}

#[test]
fn special_float_requires_a_string() {
    assert_raises(&[
        (
            "math.special_float(x'6e616e')",
            "Function math.special_float requires String as argument 1, got Bytes",
        ),
        (
            "math.special_float(1.5)",
            "Function math.special_float requires String as argument 1, got Float",
        ),
    ]);
    assert_arity("special_float", "1", &[0, 2]);
}

// --- Arguments ---

#[test]
fn every_function_requires_numbers() {
    for &(name, _, _, _) in UNARY {
        assert_raises(&[(
            &format!("math.{name}('1')"),
            &format!("Function math.{name} requires Numeric as argument 1, got String"),
        )]);
        assert_arity(name, "1", &[0, 2]);
    }
    for name in ["round", "ceil", "floor", "trunc", "abs"] {
        assert_raises(&[(
            &format!("math.{name}(null)"),
            &format!("Function math.{name} requires Numeric as argument 1, got Null"),
        )]);
        assert_arity(name, "1", &[0, 2]);
    }
    for name in ["pow", "atan2", "min", "max"] {
        assert_raises(&[
            (
                &format!("math.{name}(true, 1)"),
                &format!("Function math.{name} requires Numeric as argument 1, got Bool"),
            ),
            (
                &format!("math.{name}(1, [1])"),
                &format!("Function math.{name} requires Numeric as argument 2, got Array"),
            ),
        ]);
        assert_arity(name, "2", &[0, 1, 3]);
    }
    assert_raises(&[
        (
            "math.hypot('1', 2)",
            "Function math.hypot requires Numeric as argument 1, got String",
        ),
        (
            "math.hypot(1, '2')",
            "Function math.hypot requires Numeric as argument 2, got String",
        ),
        (
            "math.hypot(1, 2, '3')",
            "Function math.hypot requires Numeric as argument 3, got String",
        ),
        (
            "math.clamp('1', 0, 3)",
            "Function math.clamp requires Numeric as argument 1 (value), got String",
        ),
        (
            "math.clamp(1, [0], 3)",
            "Function math.clamp requires Numeric as argument 2 (lo), got Array",
        ),
        (
            "math.clamp(1, 0, '3')",
            "Function math.clamp requires Numeric as argument 3 (hi), got String",
        ),
        (
            "math.lerp(true, 1, 0)",
            "Function math.lerp requires Numeric as argument 1 (a), got Bool",
        ),
        (
            "math.lerp(0, {}, 0)",
            "Function math.lerp requires Numeric as argument 2 (b), got Map",
        ),
        (
            "math.lerp(0, 1, null)",
            "Function math.lerp requires Numeric as argument 3 (t), got Null",
        ),
    ]);
    assert_arity("hypot", "between 2 and 3", &[0, 1, 4]);
    assert_arity("clamp", "3", &[0, 2, 4]);
    assert_arity("lerp", "3", &[0, 2, 4]);
}
