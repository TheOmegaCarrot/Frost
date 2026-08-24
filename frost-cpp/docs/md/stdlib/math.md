# Math

```frost
def math = import('std.math')
```

Mathematical functions and numeric constants. All functions are accessed via dot syntax (e.g. `math.sqrt`). They accept `Int` or `Float` unless noted otherwise. Functions that delegate to C floating-point operations always return `Float`.

## `abs`

`math.abs(n)`

Returns the absolute value of `n`. Returns `Int` for `Int` input and `Float` for `Float` input. Produces an error if `n` is the minimum `Int` value (which has no positive representation).

## `round`

`math.round(n)`

Rounds `n` to the nearest integer and returns an `Int`. If `n` is already an `Int`, returns it unchanged. Produces an error if the rounded value is out of `Int` range.

## `ceil`

`math.ceil(n)`

Rounds `n` up to the nearest integer and returns an `Int`. If `n` is already an `Int`, returns it unchanged.

## `floor`

`math.floor(n)`

Rounds `n` down to the nearest integer and returns an `Int`. If `n` is already an `Int`, returns it unchanged.

## `trunc`

`math.trunc(n)`

Returns `n` rounded toward zero as an `Int`. If `n` is already an `Int`, returns it unchanged.

## `sqrt`

`math.sqrt(n)`

Returns the square root of `n`.

## `cbrt`

`math.cbrt(n)`

Returns the cube root of `n`.

## `pow`

`math.pow(base, exponent)`

Returns `base` raised to `exponent`.

## `exp`

`math.exp(n)`

Returns e raised to the power `n`.

## `exp2`

`math.exp2(n)`

Returns 2 raised to the power `n`.

## `expm1`

`math.expm1(n)`

Returns e raised to the power `n`, minus 1. More numerically accurate than `math.exp(n) - 1` for small values of `n`.

## `log`

`math.log(n)`

Returns the natural logarithm of `n`.

## `log1p`

`math.log1p(n)`

Returns the natural logarithm of `1 + n`. More numerically accurate than `math.log(1 + n)` for small values of `n`.

## `log2`

`math.log2(n)`

Returns the base-2 logarithm of `n`.

## `log10`

`math.log10(n)`

Returns the base-10 logarithm of `n`.

## `sin`

`math.sin(n)`

Returns the sine of `n` (in radians).

## `cos`

`math.cos(n)`

Returns the cosine of `n` (in radians).

## `tan`

`math.tan(n)`

Returns the tangent of `n` (in radians).

## `asin`

`math.asin(n)`

Returns the arcsine of `n` in radians. Input must be in `[-1, 1]`.

## `acos`

`math.acos(n)`

Returns the arccosine of `n` in radians. Input must be in `[-1, 1]`.

## `atan`

`math.atan(n)`

Returns the arctangent of `n` in radians. Result is in `(-pi/2, pi/2)`.

## `atan2`

`math.atan2(y, x)`

Returns the angle in radians between the positive x-axis and the point `(x, y)`. Result is in `(-pi, pi]`.

## `sinh`

`math.sinh(n)`

Returns the hyperbolic sine of `n`.

## `cosh`

`math.cosh(n)`

Returns the hyperbolic cosine of `n`.

## `tanh`

`math.tanh(n)`

Returns the hyperbolic tangent of `n`.

## `asinh`

`math.asinh(n)`

Returns the inverse hyperbolic sine of `n`.

## `acosh`

`math.acosh(n)`

Returns the inverse hyperbolic cosine of `n`. Input must be `>= 1`.

## `atanh`

`math.atanh(n)`

Returns the inverse hyperbolic tangent of `n`. Input must be in `(-1, 1)`.

## `min`

`math.min(a, b)`

Returns the smaller of `a` and `b`. Returns an `Int` if both arguments are `Int`; otherwise returns a `Float`.

## `max`

`math.max(a, b)`

Returns the larger of `a` and `b`. Returns an `Int` if both arguments are `Int`; otherwise returns a `Float`.

## `hypot`

`math.hypot(a, b)`
`math.hypot(a, b, c)`

Returns the Euclidean norm: `sqrt(a^2 + b^2)` or `sqrt(a^2 + b^2 + c^2)`.

## `clamp`

`math.clamp(value, lo, hi)`

Returns `value` constrained to the range `[lo, hi]`. Returns `lo` if `value < lo`, `hi` if `value > hi`, otherwise `value` unchanged. Returns an `Int` if all three arguments are `Int`; otherwise returns a `Float`.

## `lerp`

`math.lerp(a, b, t)`

Linearly interpolates between `a` and `b` by `t`. Returns `a` when `t` is `0` and `b` when `t` is `1`. `t` is not clamped; values outside `[0, 1]` extrapolate beyond the `[a, b]` range.

## Numeric Constants

A map of numeric constants, accessed as `math.nums`.

### `nums.pi`

pi, approximately 3.14159.

### `nums.tau`

2*pi, approximately 6.28318.

### `nums.e`

Euler's number, approximately 2.71828.

### `nums.golden_ratio`

The golden ratio, approximately 1.61803.

### `nums.maxint`

Largest `Int` value.

### `nums.minint`

Smallest (most negative) `Int` value.

### `nums.maxfloat`

Largest finite `Float` value.

### `nums.minfloat`

Most negative finite `Float` value.

### `nums.tinyfloat`

Smallest positive normalized `Float` value.

### `nums.float_epsilon`

Difference between `1.0` and the next representable `Float`.

