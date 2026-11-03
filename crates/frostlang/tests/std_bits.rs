//! `std.bits`, from Frost source.
//!
//! Each case runs with only `std.bits` installed, bound as `bits`. Every
//! function takes an Int as 64 bits in two's complement, but `float_to_bits`,
//! which takes a Float.

use crate::script;

use frostlang::stdlib::StdlibConfig;
use frostlang::{FrostFloat, ImporterBuilder, Stdlib, Value, stdlib};
use script::Script;
use script::assertions::{Library, library_assertions};

library_assertions!(Library::module(stdlib::bits, "bits"));

fn run(expression: &str) -> Value {
    script(expression).run()
}

/// `int` as Frost source. `i64::MIN` has no literal: its magnitude is past
/// the Int range.
fn int_source(int: i64) -> String {
    match int {
        i64::MIN => format!("({} - 1)", i64::MIN + 1),
        _ => int.to_string(),
    }
}

/// Ints at and around the edges of the range, and a few bit patterns.
const SAMPLES: &[i64] = &[
    0,
    1,
    -1,
    2,
    -2,
    0x55,
    0x0123_4567_89ab_cdef,
    -0x0123_4567_89ab_cdef,
    i64::MAX,
    i64::MAX - 1,
    i64::MIN,
    i64::MIN + 1,
];

/// Assert `call`, a call with `{a}` and `{b}` holes, computes `expected` for
/// every pair of [`SAMPLES`].
fn assert_binary(call: &str, expected: fn(i64, i64) -> i64) {
    for &a in SAMPLES {
        for &b in SAMPLES {
            let source = call
                .replace("{a}", &int_source(a))
                .replace("{b}", &int_source(b));
            assert_eq!(run(&source), Value::Int(expected(a, b)), "{source}");
        }
    }
}

/// Assert `call`, a call with an `{x}` hole, computes `expected` for every
/// one of [`SAMPLES`].
fn assert_unary(call: &str, expected: fn(i64) -> i64) {
    for &x in SAMPLES {
        let source = call.replace("{x}", &int_source(x));
        assert_eq!(run(&source), Value::Int(expected(x)), "{source}");
    }
}

/// Assert `call`, a call with `{x}` and `{n}` holes, computes `expected` for
/// every one of [`SAMPLES`] and every count from 0 to 63.
fn assert_by_count(call: &str, expected: fn(i64, u32) -> i64) {
    for &x in SAMPLES {
        for n in 0..64 {
            let source = call
                .replace("{x}", &int_source(x))
                .replace("{n}", &n.to_string());
            assert_eq!(run(&source), Value::Int(expected(x, n)), "{source}");
        }
    }
}

fn float(f: f64) -> Value {
    Value::Float(FrostFloat::new(f).expect("a finite float"))
}

// --- The module ---

#[test]
fn the_module_holds_its_functions() {
    assert_values(&[(
        "sorted(keys(bits))",
        "['band', 'bnot', 'bor', 'bxor', 'float_from_bits', 'float_to_bits', \
         'leading_zeros', 'popcount', 'rotate_left', 'rotate_right', 'shift_left', \
         'shift_right_arithmetic', 'shift_right_logical', 'swap_bytes', 'trailing_zeros', \
         'wrapping_add', 'wrapping_mul', 'wrapping_neg', 'wrapping_sub']",
    )]);
}

#[test]
fn the_module_is_contained() {
    let stdlib = Stdlib::contained(StdlibConfig::default());
    let contained = ImporterBuilder::new().with_stdlib(stdlib).build();
    let result = Script::new("import('std.bits').band(6, 3)")
        .importer(contained)
        .run();
    assert_eq!(result, Value::Int(2));
}

// --- Bitwise operations ---

#[test]
fn band_bor_and_bxor_combine_two_ints_bit_by_bit() {
    assert_binary("bits.band({a}, {b})", |a, b| a & b);
    assert_binary("bits.bor({a}, {b})", |a, b| a | b);
    assert_binary("bits.bxor({a}, {b})", |a, b| a ^ b);
    assert_values(&[
        ("bits.band(0b1100, 0b1010)", "0b1000"),
        ("bits.bor(0b1100, 0b1010)", "0b1110"),
        // 1 exactly where the bits differ.
        ("bits.bxor(0b1100, 0b1010)", "0b0110"),
        ("bits.bxor(-1, 0b1010)", "bits.bnot(0b1010)"),
    ]);
}

#[test]
fn band_and_bor_fold_more_than_two_ints() {
    assert_values(&[
        ("bits.band(0b1110, 0b0111, 0b1101)", "0b0100"),
        ("bits.band(-1, -1, -1, 0b101)", "0b101"),
        ("bits.bor(0b0001, 0b0010, 0b1000)", "0b1011"),
        ("bits.bor(0, 0, 0, 0, 0)", "0"),
    ]);
}

#[test]
fn bnot_flips_every_bit() {
    assert_unary("bits.bnot({x})", |x| !x);
    assert_values(&[("bits.bnot(0)", "-1"), ("bits.bnot(-1)", "0")]);
}

// --- Shifts and rotations ---

#[test]
fn shift_left_drops_the_bits_shifted_out() {
    assert_by_count("bits.shift_left({x}, {n})", |x, n| x << n);
    let min = int_source(i64::MIN);
    assert_values(&[
        ("bits.shift_left(1, 62)", "0x4000_0000_0000_0000"),
        // Into the sign bit, and out past it, with no overflow error.
        ("bits.shift_left(1, 63)", &min),
        ("bits.shift_left(3, 63)", &min),
    ]);
}

#[test]
fn shift_right_arithmetic_fills_with_the_sign_bit() {
    assert_by_count("bits.shift_right_arithmetic({x}, {n})", |x, n| x >> n);
    assert_values(&[
        ("bits.shift_right_arithmetic(-16, 2)", "-4"),
        ("bits.shift_right_arithmetic(-1, 63)", "-1"),
        ("bits.shift_right_arithmetic(16, 2)", "4"),
    ]);
}

#[test]
fn shift_right_logical_fills_with_zeros() {
    assert_by_count("bits.shift_right_logical({x}, {n})", |x, n| {
        (x.cast_unsigned() >> n).cast_signed()
    });
    assert_values(&[
        ("bits.shift_right_logical(-1, 60)", "15"),
        ("bits.shift_right_logical(-1, 1)", "0x7fff_ffff_ffff_ffff"),
        ("bits.shift_right_logical(16, 2)", "4"),
    ]);
}

#[test]
fn rotations_carry_the_bits_shifted_out_around() {
    assert_by_count("bits.rotate_left({x}, {n})", i64::rotate_left);
    assert_by_count("bits.rotate_right({x}, {n})", i64::rotate_right);
    let min = int_source(i64::MIN);
    assert_values(&[
        (&format!("bits.rotate_left({min}, 1)"), "1"),
        ("bits.rotate_right(1, 1)", &min),
        // The top byte comes around to the bottom.
        ("bits.rotate_left(0x0100_0000_0000_0002, 8)", "0x0201"),
        ("bits.rotate_right(0x0201, 8)", "0x0100_0000_0000_0002"),
    ]);
}

#[test]
fn a_count_outside_0_to_63_is_an_error() {
    for name in [
        "shift_left",
        "shift_right_arithmetic",
        "shift_right_logical",
        "rotate_left",
        "rotate_right",
    ] {
        for count in [-1, 64, 65, i64::MAX, i64::MIN] {
            assert_raises(&[(
                &format!("bits.{name}(1, {})", int_source(count)),
                &format!(
                    "Function std.bits.{name} requires argument 2 (count) to be from 0 to 63, \
                     got {count}"
                ),
            )]);
        }
    }
}

// --- Counts and byte order ---

#[test]
fn bit_counts_count_ones_and_zeros_at_each_end() {
    assert_unary("bits.popcount({x})", |x| i64::from(x.count_ones()));
    assert_unary("bits.leading_zeros({x})", |x| i64::from(x.leading_zeros()));
    assert_unary("bits.trailing_zeros({x})", |x| {
        i64::from(x.trailing_zeros())
    });
    assert_values(&[
        ("bits.popcount(-1)", "64"),
        ("bits.leading_zeros(0)", "64"),
        ("bits.trailing_zeros(0)", "64"),
        ("bits.leading_zeros(-1)", "0"),
        ("bits.leading_zeros(1)", "63"),
    ]);
}

#[test]
fn swap_bytes_reverses_the_byte_order() {
    assert_unary("bits.swap_bytes({x})", i64::swap_bytes);
    assert_values(&[(
        "bits.swap_bytes(0x0102_0304_0506_0708)",
        "0x0807_0605_0403_0201",
    )]);
}

// --- Wrapping arithmetic ---

#[test]
fn wrapping_arithmetic_wraps_in_twos_complement_and_never_raises() {
    assert_binary("bits.wrapping_add({a}, {b})", i64::wrapping_add);
    assert_binary("bits.wrapping_sub({a}, {b})", i64::wrapping_sub);
    assert_binary("bits.wrapping_mul({a}, {b})", i64::wrapping_mul);
    assert_unary("bits.wrapping_neg({x})", i64::wrapping_neg);
    let min = int_source(i64::MIN);
    assert_values(&[
        ("bits.wrapping_add(0x7fff_ffff_ffff_ffff, 1)", &min),
        (
            &format!("bits.wrapping_sub({min}, 1)"),
            "0x7fff_ffff_ffff_ffff",
        ),
        (&format!("bits.wrapping_neg({min})"), &min),
        ("bits.wrapping_mul(0x7fff_ffff_ffff_ffff, 2)", "-2"),
    ]);
}

// --- A Float's bits ---

#[test]
fn float_to_bits_is_the_ieee_754_encoding() {
    for f in [
        0.0,
        -0.0,
        1.0,
        -1.5,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        std::f64::consts::PI,
    ] {
        let literal = float(f).to_frost_string();
        let source = format!("bits.float_to_bits({literal})");
        assert_eq!(
            run(&source),
            Value::Int(f.to_bits().cast_signed()),
            "{source}"
        );
        let round_trip = format!("bits.float_from_bits(bits.float_to_bits({literal}))");
        assert_eq!(run(&round_trip), float(f), "{round_trip}");
    }
    // A Float with its sign bit set is a negative Int.
    assert_values(&[
        ("bits.float_to_bits(1.0)", "0x3ff0_0000_0000_0000"),
        ("bits.float_to_bits(-0.0)", &int_source(i64::MIN)),
        (
            "bits.float_to_bits(-2.0)",
            &int_source(i64::MIN + 0x4000_0000_0000_0000),
        ),
    ]);
}

#[test]
fn float_from_bits_inverts_float_to_bits() {
    assert_values(&[
        ("bits.float_from_bits(0)", "0.0"),
        ("bits.float_from_bits(0x3ff0_0000_0000_0000)", "1.0"),
        // Signed zero survives the round trip; Float equality tells it apart.
        ("bits.float_from_bits(bits.float_to_bits(-0.0))", "-0.0"),
        ("bits.float_from_bits(bits.float_to_bits(-1.5))", "-1.5"),
    ]);
    // The least subnormal Float, and the greatest and least finite ones, just
    // short of the infinities.
    for (bits, f) in [
        (1, f64::from_bits(1)),
        (0x7fef_ffff_ffff_ffff, f64::MAX),
        (0xffef_ffff_ffff_ffff_u64.cast_signed(), f64::MIN),
    ] {
        let source = format!("bits.float_from_bits({})", int_source(bits));
        assert_eq!(run(&source), float(f), "{source}");
    }
}

#[test]
fn float_from_bits_is_null_for_a_nan_or_an_infinity() {
    for bits in [
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        f64::NAN.to_bits(),
        0x7ff0_0000_0000_0001,
        u64::MAX,
    ] {
        let source = format!("bits.float_from_bits({})", int_source(bits.cast_signed()));
        assert_eq!(run(&source), Value::Null, "{source}");
    }
}

// --- Arguments ---

#[test]
fn every_function_requires_ints() {
    for name in [
        "bnot",
        "popcount",
        "leading_zeros",
        "trailing_zeros",
        "swap_bytes",
        "wrapping_neg",
    ] {
        for (arg, type_name) in [("1.0", "Float"), ("'1'", "String"), ("null", "Null")] {
            assert_raises(&[(
                &format!("bits.{name}({arg})"),
                &format!("Function std.bits.{name} requires Int as argument 1, got {type_name}"),
            )]);
        }
        assert_arity(name, "1", &[0, 2]);
    }
    for name in ["bxor", "wrapping_add", "wrapping_sub", "wrapping_mul"] {
        assert_raises(&[
            (
                &format!("bits.{name}(1.0, 1)"),
                &format!("Function std.bits.{name} requires Int as argument 1, got Float"),
            ),
            (
                &format!("bits.{name}(1, true)"),
                &format!("Function std.bits.{name} requires Int as argument 2, got Bool"),
            ),
        ]);
        assert_arity(name, "2", &[0, 1, 3]);
    }
    for name in ["band", "bor"] {
        assert_raises(&[
            (
                &format!("bits.{name}(1.0, 1)"),
                &format!("Function std.bits.{name} requires Int as argument 1, got Float"),
            ),
            (
                &format!("bits.{name}(1, [1])"),
                &format!("Function std.bits.{name} requires Int as argument 2, got Array"),
            ),
            (
                &format!("bits.{name}(1, 2, 3, x'01')"),
                &format!("Function std.bits.{name} requires Int as argument 4, got Bytes"),
            ),
        ]);
        assert_arity(name, "at least 2", &[0, 1]);
    }
    for name in [
        "shift_left",
        "shift_right_arithmetic",
        "shift_right_logical",
        "rotate_left",
        "rotate_right",
    ] {
        assert_raises(&[
            (
                &format!("bits.{name}(1.0, 1)"),
                &format!("Function std.bits.{name} requires Int as argument 1 (value), got Float"),
            ),
            (
                &format!("bits.{name}(1, 1.0)"),
                &format!("Function std.bits.{name} requires Int as argument 2 (count), got Float"),
            ),
        ]);
        assert_arity(name, "2", &[0, 1, 3]);
    }
}

#[test]
fn float_to_bits_requires_a_float_and_float_from_bits_an_int() {
    assert_raises(&[
        (
            "bits.float_to_bits(1)",
            "Function std.bits.float_to_bits requires Float as argument 1, got Int",
        ),
        (
            "bits.float_to_bits('1.0')",
            "Function std.bits.float_to_bits requires Float as argument 1, got String",
        ),
        (
            "bits.float_from_bits(1.0)",
            "Function std.bits.float_from_bits requires Int as argument 1, got Float",
        ),
    ]);
    assert_arity("float_to_bits", "1", &[0, 2]);
    assert_arity("float_from_bits", "1", &[0, 2]);
}
