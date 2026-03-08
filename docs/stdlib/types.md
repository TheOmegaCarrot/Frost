# Types

## `is_null`
`is_null(value)`

Returns `true` if `value` is `null`.

## `is_int`
`is_int(value)`

Returns `true` if `value` is an `Int`.

## `is_float`
`is_float(value)`

Returns `true` if `value` is a `Float`.

## `is_bool`
`is_bool(value)`

Returns `true` if `value` is a `Bool`.

## `is_string`
`is_string(value)`

Returns `true` if `value` is a `String`.

## `is_array`
`is_array(value)`

Returns `true` if `value` is an `Array`.

## `is_map`
`is_map(value)`

Returns `true` if `value` is a `Map`.

## `is_function`
`is_function(value)`

Returns `true` if `value` is a `Function`.

## `is_nonnull`
`is_nonnull(value)`

Returns `true` if `value` is not `null`. Equivalent to `not is_null(value)`.

## `is_numeric`
`is_numeric(value)`

Returns `true` if `value` is an `Int` or `Float`.

## `is_primitive`
`is_primitive(value)`

Returns `true` if `value` is a `Null`, `Bool`, `Int`, `Float`, or `String`.

## `is_structured`
`is_structured(value)`

Returns `true` if `value` is an `Array` or `Map`.

## `type`
`type(value)`

Returns the type of `value` as a `String`. Possible values: `"Null"`, `"Int"`,
`"Float"`, `"Bool"`, `"String"`, `"Array"`, `"Map"`, `"Function"`.

## `to_string`
`to_string(value)`

Converts `value` to its display string — the same representation that `print`
would output. For strings, returns the string itself (unquoted). For arrays and
maps, returns a compact single-line representation.

## `pretty`
`pretty(value)`

Like `to_string`, but formats arrays and maps as indented, multi-line output.
Map keys that are valid identifiers are rendered without quotes.

## `to_int`
`to_int(value)`

Converts `value` to an `Int`. Strings are parsed as base-10 integers.

## `to_float`
`to_float(value)`

Converts `value` to a `Float`. Strings are parsed as base-10 floating-point
numbers.
