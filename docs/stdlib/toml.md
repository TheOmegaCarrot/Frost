# TOML

```frost
def toml = import('ext.toml')
```

Encode and decode [TOML v1.0.0](https://toml.io/en/v1.0.0) documents with full semantic round-tripping.

Build flag: `WITH_TOML` (default: `AUTO`). No external dependencies.

## Foreign Values

TOML has types that Frost does not: dates, times, datetimes, and the special floats `nan`/`inf`/`-inf`. When `decode` encounters one of these, it produces a [foreign value](foreign-values.md). Call it to get a `String` representation, or pass it through to `encode` to preserve the original TOML type.

```frost
def doc = toml.decode('created = 2024-03-16')
doc.created       # => <Function>
doc.created()     # => "2024-03-16"
toml.encode(doc)  # round-trips as a TOML date, not a string
```

You can also construct foreign values directly using `toml.date`, `toml.time`, `toml.date_time`, and `toml.special_float`. This lets you build TOML documents from scratch that include these types.

```frost
def doc = {
    created: toml.date('2024-03-16'),
    updated: toml.date_time('2024-03-16T10:30:00Z'),
    threshold: toml.special_float('inf'),
}
toml.encode(doc)
```

## `decode`

`toml.decode(s)`

Parses a TOML document from `String` `s` and returns a `Map`. All TOML types map to their natural Frost equivalents. Dates, times, datetimes, and special floats become foreign values.

```frost
def config = toml.decode('''
[server]
host = "localhost"
port = 8080
''')
config.server.host  # => "localhost"
```

## `encode`

`toml.encode(m)`

Serializes a `Map` to a TOML `String`. Only string keys are permitted. `null` values and plain `Function` values are rejected. Foreign values produced by `decode` or the constructor functions are serialized with their original TOML types.

```frost
toml.encode({
    title: 'My Config',
    database: {enabled: true, ports: [8001, 8002]},
})
```

The output is valid TOML, but may not be ideally formatted for all use cases. TOML allows many textual representations of the same data, and `encode` does not currently offer formatting options.

## `date`

`toml.date(s)`

Creates a TOML date foreign value from a `String` in `YYYY-MM-DD` format. See the [TOML date specification](https://toml.io/en/v1.0.0#local-date) for exact syntax.

```frost
toml.date('2024-03-16')()
# => 2024-03-16
```

## `time`

`toml.time(s)`

Creates a TOML time foreign value from a `String` in `HH:MM:SS` format, with optional fractional seconds. See the [TOML time specification](https://toml.io/en/v1.0.0#local-time) for exact syntax.

```frost
toml.time('14:30:00')()
# => 14:30:00
```

```frost
toml.time('08:23:22.000523000')()
# => 08:23:22.000523000
```

## `date_time`

`toml.date_time(s)`

Creates a TOML datetime foreign value from a `String`. Accepts any datetime format that TOML supports: local datetimes, offset datetimes with `Z` or `+HH:MM`/`-HH:MM`, and the space separator variant. See the [TOML datetime specification](https://toml.io/en/v1.0.0#offset-date-time) for exact syntax.

```frost
toml.date_time('1979-05-27T07:32:00Z')()
# => 1979-05-27T07:32:00Z
```

```frost
toml.date_time('1979-05-27T07:32:00')()
# => 1979-05-27T07:32:00
```

```frost
toml.date_time('1979-05-27T07:32:00+09:00')()
# => 1979-05-27T07:32:00+09:00
```

## `special_float`

`toml.special_float(s)`

Creates a TOML special float foreign value. `s` must be one of `"nan"`, `"inf"`, or `"-inf"` (lowercase, matching TOML syntax). Frost normally rejects these values at construction, but foreign values can carry them safely for round-tripping.

```frost
toml.special_float('nan')()
# => nan
```

```frost
toml.special_float('inf')()
# => inf
```

