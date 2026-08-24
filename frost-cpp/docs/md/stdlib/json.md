# JSON

```frost
def json = import('std.json')
```

Encode and decode JSON.

## `decode`

`json.decode(s)`

Parses a JSON string and returns the corresponding Frost value. `//` and `/* */` comments and trailing commas are accepted as an extension.

Type mapping: `null` -> `null`, booleans -> `Bool`, integers -> `Int`, floats -> `Float`, strings -> `String`, arrays -> `Array`, objects -> `Map` with `String` keys. Produces an error on malformed input or integer values out of `Int` range.

## `encode`

`json.encode(value)`

Serializes `value` to a compact JSON string. `Map` keys must be `String`. `Function` values cannot be serialized. Produces an error on unsupported values.

## `encode_pretty`

`json.encode_pretty(value, indent)`

Serializes `value` to a pretty-printed JSON string with the given `indent` level (number of spaces per nesting level). `indent` must be a non-negative `Int`. Carries the same value restrictions as `encode`.

```frost
json.encode_pretty({name: 'Alice', scores: [10, 20]}, 2)
# {
#   "name": "Alice",
#   "scores": [
#     10,
#     20
#   ]
# }
```

