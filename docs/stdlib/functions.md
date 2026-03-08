# Functions

## `pack_call`
`pack_call(function, args)`

Calls `function` with the elements of the array `args` spread as individual
arguments.

## `try_call`
`try_call(function, args)`

Calls `function` with the elements of `args` spread as arguments. Returns
`{ ok: true, value: result }` on success, or `{ ok: false, error: message }`
if the call produces a recoverable error.

## `and_then`
`and_then(value, f)`

If `value` is `null`, returns `null`. Otherwise calls `f(value)` and returns
the result. Useful for chaining operations that may produce `null`.

## `or_else`
`or_else(value, f)`

If `value` is not `null`, returns `value`. If `value` is `null`, calls `f()`
with no arguments and returns the result. Useful for providing fallbacks.
