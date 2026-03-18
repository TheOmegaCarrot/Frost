# Math

All math functions accept `Int` or `Float` unless noted otherwise.
Functions that delegate to C floating-point operations always return `Float`.

## `abs`
`abs(n)`

Returns the absolute value of `n`.
Returns `Int` for `Int` input and `Float` for `Float` input.
Produces an error if `n` is the minimum `Int` value (which has no positive representation).

## `round`
`round(n)`

Rounds `n` to the nearest integer and returns an `Int`.
If `n` is already an `Int`, returns it unchanged.
Produces an error if the rounded value is out of `Int` range.

## `ceil`
`ceil(n)`

Rounds `n` up to the nearest integer and returns an `Int`.
If `n` is already an `Int`, returns it unchanged.

## `floor`
`floor(n)`

Rounds `n` down to the nearest integer and returns an `Int`.
If `n` is already an `Int`, returns it unchanged.

## `trunc`
`trunc(n)`

Returns `n` rounded toward zero as a `Float`.

## `sqrt`
`sqrt(n)`

Returns the square root of `n`.

## `cbrt`
`cbrt(n)`

Returns the cube root of `n`.

## `pow`
`pow(base, exponent)`

Returns `base` raised to `exponent`.

## `exp`
`exp(n)`

Returns e raised to the power `n`.

## `exp2`
`exp2(n)`

Returns 2 raised to the power `n`.

## `expm1`
`expm1(n)`

Returns e raised to the power `n`, minus 1.
More numerically accurate than `exp(n) - 1` for small values of `n`.

## `log`
`log(n)`

Returns the natural logarithm of `n`.

## `log1p`
`log1p(n)`

Returns the natural logarithm of `1 + n`.
More numerically accurate than `log(1 + n)` for small values of `n`.

## `log2`
`log2(n)`

Returns the base-2 logarithm of `n`.

## `log10`
`log10(n)`

Returns the base-10 logarithm of `n`.

## `sin`
`sin(n)`

Returns the sine of `n` (in radians).

## `cos`
`cos(n)`

Returns the cosine of `n` (in radians).

## `tan`
`tan(n)`

Returns the tangent of `n` (in radians).

## `asin`
`asin(n)`

Returns the arcsine of `n` in radians. Input must be in `[-1, 1]`.

## `acos`
`acos(n)`

Returns the arccosine of `n` in radians. Input must be in `[-1, 1]`.

## `atan`
`atan(n)`

Returns the arctangent of `n` in radians. Result is in `(-π/2, π/2)`.

## `atan2`
`atan2(y, x)`

Returns the angle in radians between the positive x-axis and the point `(x, y)`.
Result is in `(-π, π]`.

## `sinh`
`sinh(n)`

Returns the hyperbolic sine of `n`.

## `cosh`
`cosh(n)`

Returns the hyperbolic cosine of `n`.

## `tanh`
`tanh(n)`

Returns the hyperbolic tangent of `n`.

## `asinh`
`asinh(n)`

Returns the inverse hyperbolic sine of `n`.

## `acosh`
`acosh(n)`

Returns the inverse hyperbolic cosine of `n`. Input must be `>= 1`.

## `atanh`
`atanh(n)`

Returns the inverse hyperbolic tangent of `n`. Input must be in `(-1, 1)`.

## `min`
`min(a, b)`

Returns the smaller of `a` and `b` as a `Float`.

## `max`
`max(a, b)`

Returns the larger of `a` and `b` as a `Float`.

## `hypot`
`hypot(a, b)`
`hypot(a, b, c)`

Returns the Euclidean norm: `sqrt(a² + b²)` or `sqrt(a² + b² + c²)`.

## `clamp`
`clamp(value, lo, hi)`

Returns `value` constrained to the range `[lo, hi]`.
Returns `lo` if `value < lo`, `hi` if `value > hi`, otherwise `value` unchanged.
Returns an `Int` if all three arguments are `Int`; otherwise returns a `Float`.

## `nums`

A map of numeric constants.

| Key | Value |
|---|---|
| `pi` | π ≈ 3.14159 |
| `tau` | 2π ≈ 6.28318 |
| `e` | Euler's number ≈ 2.71828 |
| `golden_ratio` | φ ≈ 1.61803 |
| `maxint` | Largest `Int` value |
| `minint` | Smallest (most negative) `Int` value |
| `maxfloat` | Largest finite `Float` value |
| `minfloat` | Most negative finite `Float` value |
| `tinyfloat` | Smallest positive normalized `Float` value |
| `float_epsilon` | Difference between `1.0` and the next representable `Float` |

## `lerp`
`lerp(a, b, t)`

Linearly interpolates between `a` and `b` by `t`.
Returns `a` when `t` is `0` and `b` when `t` is `1`.
`t` is not clamped; values outside `[0, 1]` extrapolate beyond the `[a, b]` range.
