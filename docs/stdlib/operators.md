# Operators

Functional forms of the built-in infix operators. Useful for passing operators
as first-class values, e.g. to `fold` or `sorted`.

## `plus`
`plus(a, b)`

Equivalent to `a + b`.

## `minus`
`minus(a, b)`

Equivalent to `a - b`.

## `times`
`times(a, b)`

Equivalent to `a * b`.

## `divide`
`divide(a, b)`

Equivalent to `a / b`.

## `mod`
`mod(a, b)`

Equivalent to `a % b`.

## `equal`
`equal(a, b)`

Equivalent to `a == b`.

## `not_equal`
`not_equal(a, b)`

Equivalent to `a != b`.

## `less_than`
`less_than(a, b)`

Equivalent to `a < b`.

## `less_than_or_equal`
`less_than_or_equal(a, b)`

Equivalent to `a <= b`.

## `greater_than`
`greater_than(a, b)`

Equivalent to `a > b`.

## `greater_than_or_equal`
`greater_than_or_equal(a, b)`

Equivalent to `a >= b`.

## `deep_equal`
`deep_equal(a, b)`

Returns `true` if `a` and `b` are recursively structurally equal: arrays and
maps are compared element by element rather than by identity.
