//! `std.bits`: an Int taken as 64 bits in two's complement rather than as a
//! number.

use crate::native::{Nullable, Rest};
use crate::{FrostError, FrostFloat, NativeCtx, StdlibModule, frost_fn};

/// The `std.bits` module: an Int as 64 bits in two's complement rather than as
/// a number, for bitwise logic, shifts, bit counts, and arithmetic that wraps
/// instead of raising on overflow.
///
/// It only computes: it reads and changes nothing outside the script.
pub fn bits() -> StdlibModule {
    StdlibModule::builder("bits")
        .function(band_frost())
        .function(bor_frost())
        .function(bxor_frost())
        .function(bnot_frost())
        .function(shift_left_frost())
        .function(shift_right_arithmetic_frost())
        .function(shift_right_logical_frost())
        .function(rotate_left_frost())
        .function(rotate_right_frost())
        .function(popcount_frost())
        .function(leading_zeros_frost())
        .function(trailing_zeros_frost())
        .function(swap_bytes_frost())
        .function(wrapping_add_frost())
        .function(wrapping_sub_frost())
        .function(wrapping_mul_frost())
        .function(wrapping_neg_frost())
        .function(float_to_bits_frost())
        .function(float_from_bits_frost())
        .build()
}

#[frost_fn]
fn band(
    #[frost(unnamed)] a: i64,
    #[frost(unnamed)] b: i64,
    #[frost(unnamed)] Rest(more): Rest<i64>,
) -> i64 {
    more.into_iter().fold(a & b, |all, x| all & x)
}

#[frost_fn]
fn bor(
    #[frost(unnamed)] a: i64,
    #[frost(unnamed)] b: i64,
    #[frost(unnamed)] Rest(more): Rest<i64>,
) -> i64 {
    more.into_iter().fold(a | b, |any, x| any | x)
}

#[frost_fn]
fn bxor(#[frost(unnamed)] a: i64, #[frost(unnamed)] b: i64) -> i64 {
    a ^ b
}

#[frost_fn]
fn bnot(#[frost(unnamed)] x: i64) -> i64 {
    !x
}

/// `count`, the number of bit positions `function` shifts or rotates by, if it
/// is from 0 to 63.
fn count_arg(function: &str, count: i64) -> Result<u32, FrostError> {
    match u32::try_from(count) {
        Ok(count @ 0..=63) => Ok(count),
        _ => Err(FrostError::from_string(format!(
            "Function {function} requires argument 2 (count) to be from 0 to 63, got {count}"
        ))),
    }
}

/// Shifts `value` left, dropping the bits shifted out past the top.
#[frost_fn]
fn shift_left(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: i64,
    count: i64,
) -> Result<i64, FrostError> {
    Ok(value << count_arg(ctx.name(), count)?)
}

/// Shifts `value` right, filling with its sign bit.
#[frost_fn]
fn shift_right_arithmetic(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: i64,
    count: i64,
) -> Result<i64, FrostError> {
    Ok(value >> count_arg(ctx.name(), count)?)
}

/// Shifts `value` right, filling with zeros.
#[frost_fn]
fn shift_right_logical(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: i64,
    count: i64,
) -> Result<i64, FrostError> {
    Ok((value.cast_unsigned() >> count_arg(ctx.name(), count)?).cast_signed())
}

#[frost_fn]
fn rotate_left(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: i64,
    count: i64,
) -> Result<i64, FrostError> {
    Ok(value.rotate_left(count_arg(ctx.name(), count)?))
}

#[frost_fn]
fn rotate_right(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    value: i64,
    count: i64,
) -> Result<i64, FrostError> {
    Ok(value.rotate_right(count_arg(ctx.name(), count)?))
}

#[frost_fn]
fn popcount(#[frost(unnamed)] x: i64) -> u32 {
    x.count_ones()
}

#[frost_fn]
fn leading_zeros(#[frost(unnamed)] x: i64) -> u32 {
    x.leading_zeros()
}

#[frost_fn]
fn trailing_zeros(#[frost(unnamed)] x: i64) -> u32 {
    x.trailing_zeros()
}

#[frost_fn]
fn swap_bytes(#[frost(unnamed)] x: i64) -> i64 {
    x.swap_bytes()
}

#[frost_fn]
fn wrapping_add(#[frost(unnamed)] a: i64, #[frost(unnamed)] b: i64) -> i64 {
    a.wrapping_add(b)
}

#[frost_fn]
fn wrapping_sub(#[frost(unnamed)] a: i64, #[frost(unnamed)] b: i64) -> i64 {
    a.wrapping_sub(b)
}

#[frost_fn]
fn wrapping_mul(#[frost(unnamed)] a: i64, #[frost(unnamed)] b: i64) -> i64 {
    a.wrapping_mul(b)
}

#[frost_fn]
fn wrapping_neg(#[frost(unnamed)] x: i64) -> i64 {
    x.wrapping_neg()
}

/// The IEEE 754 binary64 encoding of `x`.
#[frost_fn]
fn float_to_bits(#[frost(unnamed)] x: FrostFloat) -> i64 {
    x.get().to_bits().cast_signed()
}

/// The Float the IEEE 754 binary64 encoding `bits` encodes; Null for a NaN or
/// an infinity, which no Float holds.
#[frost_fn]
fn float_from_bits(#[frost(unnamed)] bits: i64) -> Nullable<FrostFloat> {
    Nullable(FrostFloat::new(f64::from_bits(bits.cast_unsigned())).ok())
}
