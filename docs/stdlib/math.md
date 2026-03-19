# Math

All math functions live in the `math` namespace map and are accessed via dot syntax (e.g. `math.sqrt`).
They accept `Int` or `Float` unless noted otherwise.
Functions that delegate to C floating-point operations always return `Float`.

## `math.abs`
`math.abs(n)`

Returns the absolute value of `n`.
Returns `Int` for `Int` input and `Float` for `Float` input.
Produces an error if `n` is the minimum `Int` value (which has no positive representation).

## `math.round`
`math.round(n)`

Rounds `n` to the nearest integer and returns an `Int`.
If `n` is already an `Int`, returns it unchanged.
Produces an error if the rounded value is out of `Int` range.

## `math.ceil`
`math.ceil(n)`

Rounds `n` up to the nearest integer and returns an `Int`.
If `n` is already an `Int`, returns it unchanged.

## `math.floor`
`math.floor(n)`

Rounds `n` down to the nearest integer and returns an `Int`.
If `n` is already an `Int`, returns it unchanged.

## `math.trunc`
`math.trunc(n)`

Returns `n` rounded toward zero as a `Float`.

## `math.sqrt`
`math.sqrt(n)`

Returns the square root of `n`.

## `math.cbrt`
`math.cbrt(n)`

Returns the cube root of `n`.

## `math.pow`
`math.pow(base, exponent)`

Returns `base` raised to `exponent`.

## `math.exp`
`math.exp(n)`

Returns e raised to the power `n`.

## `math.exp2`
`math.exp2(n)`

Returns 2 raised to the power `n`.

## `math.expm1`
`math.expm1(n)`

Returns e raised to the power `n`, minus 1.
More numerically accurate than `math.exp(n) - 1` for small values of `n`.

## `math.log`
`math.log(n)`

Returns the natural logarithm of `n`.

## `math.log1p`
`math.log1p(n)`

Returns the natural logarithm of `1 + n`.
More numerically accurate than `math.log(1 + n)` for small values of `n`.

## `math.log2`
`math.log2(n)`

Returns the base-2 logarithm of `n`.

## `math.log10`
`math.log10(n)`

Returns the base-10 logarithm of `n`.

## `math.sin`
`math.sin(n)`

Returns the sine of `n` (in radians).

## `math.cos`
`math.cos(n)`

Returns the cosine of `n` (in radians).

## `math.tan`
`math.tan(n)`

Returns the tangent of `n` (in radians).

## `math.asin`
`math.asin(n)`

Returns the arcsine of `n` in radians. Input must be in `[-1, 1]`.

## `math.acos`
`math.acos(n)`

Returns the arccosine of `n` in radians. Input must be in `[-1, 1]`.

## `math.atan`
`math.atan(n)`

Returns the arctangent of `n` in radians. Result is in `(-π/2, π/2)`.

## `math.atan2`
`math.atan2(y, x)`

Returns the angle in radians between the positive x-axis and the point `(x, y)`.
Result is in `(-π, π]`.

## `math.sinh`
`math.sinh(n)`

Returns the hyperbolic sine of `n`.

## `math.cosh`
`math.cosh(n)`

Returns the hyperbolic cosine of `n`.

## `math.tanh`
`math.tanh(n)`

Returns the hyperbolic tangent of `n`.

## `math.asinh`
`math.asinh(n)`

Returns the inverse hyperbolic sine of `n`.

## `math.acosh`
`math.acosh(n)`

Returns the inverse hyperbolic cosine of `n`. Input must be `>= 1`.

## `math.atanh`
`math.atanh(n)`

Returns the inverse hyperbolic tangent of `n`. Input must be in `(-1, 1)`.

## `math.min`
`math.min(a, b)`

Returns the smaller of `a` and `b` as a `Float`.

## `math.max`
`math.max(a, b)`

Returns the larger of `a` and `b` as a `Float`.

## `math.hypot`
`math.hypot(a, b)`
`math.hypot(a, b, c)`

Returns the Euclidean norm: `sqrt(a² + b²)` or `sqrt(a² + b² + c²)`.

## `math.clamp`
`math.clamp(value, lo, hi)`

Returns `value` constrained to the range `[lo, hi]`.
Returns `lo` if `value < lo`, `hi` if `value > hi`, otherwise `value` unchanged.
Returns an `Int` if all three arguments are `Int`; otherwise returns a `Float`.

## `math.nums`

A map of numeric constants, accessed as `math.nums`.

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

## `math.lerp`
`math.lerp(a, b, t)`

Linearly interpolates between `a` and `b` by `t`.
Returns `a` when `t` is `0` and `b` when `t` is `1`.
`t` is not clamped; values outside `[0, 1]` extrapolate beyond the `[a, b]` range.
