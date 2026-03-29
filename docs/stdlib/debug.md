# Debug

## `assert`
`assert(condition)`
`assert(condition, message)`

Produces a recoverable error if `condition` is falsy.
The optional `message` is included in the error.
Returns `condition` unchanged if the assertion passes.

## `debug_dump`
`debug_dump(value)`

Returns a string representation of `value` for debugging.
For non-function values, this is similar to [`to_string`](types.md#to_string), except that strings are quoted.
For functions, returns an AST dump of the function body.
