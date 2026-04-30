# MessagePack

```frost
def mp = import('ext.msgpack')
```

Encode and decode [MessagePack](https://msgpack.org/) binary data with full type round-tripping.

MessagePack is a compact binary serialization format. Unlike JSON, it distinguishes binary data from strings and supports `null` as a first-class type.

Build flag: `WITH_MSGPACK` (default: `AUTO`). No system dependencies.

## Foreign Values

MessagePack has types that Frost does not natively support:

- Binary data: MessagePack distinguishes binary bytes from UTF-8 strings. Both are Frost Strings, so binary data is wrapped as a [foreign value](foreign-values.md) to preserve the distinction.
- Special floats: `nan`, `inf`, and `-inf` are valid MessagePack floats but rejected by Frost's `Float` type.

## `decode`

`mp.decode(data)`

Parses a MessagePack-encoded `String` and returns the corresponding Frost value. The top-level value can be any valid MessagePack type. Unsigned integers that exceed `Int` range are rejected. Extension types are not supported.

```frost
mp.decode(mp.encode(42))
# => 42
```

```frost
mp.decode(mp.encode('hi'))
# => hi
```

## `encode`

`mp.encode(value)`

Serializes any Frost value to a MessagePack-encoded `String`. Maps may use any key type (MessagePack does not restrict map keys to strings). Plain `Function` values are rejected. Foreign values (binary, special floats) are serialized with their original MessagePack types.

```frost
mp.encode({name: 'alice', age: 30})
mp.encode([1, 2, 3])
mp.encode(null)
```

## `binary`

`mp.binary(data)`

Creates a binary foreign value from a `String`. When encoded, this value uses the MessagePack binary type rather than string.

```frost
mp.binary('raw bytes')()
# => raw bytes
```

```frost
def b = mp.binary('raw bytes')
def rt = mp.decode(mp.encode(b))
rt()  # round-trips as binary, not string
```

## `special_float`

`mp.special_float(s)`

Creates a special float foreign value. `s` must be one of `"nan"`, `"inf"`, or `"-inf"`.

```frost
mp.special_float('nan')()
# => nan
```

