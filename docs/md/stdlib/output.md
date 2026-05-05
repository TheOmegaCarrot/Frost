# Output

Printing and message formatting. Available without importing.

## `print`

`print(value)`

Prints `value` to stdout followed by a newline. Uses the same display representation as [`to_string`](types.md#to_string).

## `mformat`

`mformat(format_string, replacement_map)`

Returns a copy of `format_string` with `${key}` placeholders replaced by the corresponding values from `replacement_map`. Uses the same `${key}` delimiter syntax as [language format strings](../language.md#types-and-literals), but only performs key lookup in the map (not arbitrary expression evaluation). Produces an error if a placeholder key is not present in the map.

## `mprint`

`mprint(format_string, replacement_map)`

Equivalent to `print(mformat(format_string, replacement_map))`.

