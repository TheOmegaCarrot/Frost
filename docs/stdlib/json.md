# JSON

## `parse_json`
`parse_json(s)`

Parses a JSON string and returns the corresponding Frost value. `//` and `/* */`
comments and trailing commas are accepted as an extension. Type mapping: `null` → `null`, booleans →
`Bool`, integers → `Int`, floats → `Float`, strings → `String`, arrays →
`Array`, objects → `Map` with `String` keys. Produces an error on malformed
input or integer values out of `Int` range.

## `to_json`
`to_json(value)`

Serializes `value` to a JSON string. `Map` keys must be `String`. `Function`
values cannot be serialized. Produces an error on unsupported values.
