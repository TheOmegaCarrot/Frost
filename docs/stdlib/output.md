# Output

## `print`
`print(value)`

Prints `value` to stdout followed by a newline. Uses the same display
representation as `to_string`.

## `mformat`
`mformat(format string, replacement map)`

Returns a copy of `format string` with `${key}` placeholders replaced by the
corresponding values from `replacement map`. Produces an error if a placeholder
key is not present in the map. The placeholder syntax is identical to
[language format strings](../language.md#string).

## `mprint`
`mprint(format string, replacement map)`

Equivalent to `print(mformat(format string, replacement map))`.
