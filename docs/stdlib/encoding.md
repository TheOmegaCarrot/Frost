# Encoding

```frost
def enc = import('std.encoding')
```

Number formatting, byte-level string manipulation, and common encoding schemes (hex, Base64, URL percent-encoding).

## `fmt_int`

`encoding.fmt_int(number, base)`

Returns a lowercase string representation of `number` in the given `base`. `base` must be in the range [2, 36].

- `number` -- The number to format
- `base` -- The numeric base (2-36)

```frost
encoding.fmt_int(255, 16)
# => ff
```

```frost
encoding.fmt_int(10, 2)
# => 1010
```

See also:
[`parse_int`](encoding.md#parse_int)

## `parse_int`

`encoding.parse_int(string, base)`

Parses `string` as an integer in the given `base` and returns an Int. `base` must be in the range [2, 36].

Produces an error if the string is not a valid integer in that base, or if the value is out of Int range.

- `string` -- The string to parse
- `base` -- The numeric base (2-36)

```frost
encoding.parse_int('ff', 16)
# => 255
```

```frost
encoding.parse_int('1010', 2)
# => 10
```

See also:
[`fmt_int`](encoding.md#fmt_int)

## `to_bytes`

`encoding.to_bytes(s)`

Returns an array of Int values representing the raw byte values (0-255) of each character in `s`.

```frost
encoding.to_bytes('ABC')
# => [65, 66, 67]
```

See also:
[`from_bytes`](encoding.md#from_bytes)

## `from_bytes`

`encoding.from_bytes(arr)`

Converts an array of Int values in the range [0, 255] to a string.

Produces an error if any element is not an Int or is out of range.

```frost
encoding.from_bytes([72, 105])
# => Hi
```

See also:
[`to_bytes`](encoding.md#to_bytes)

## b64

Base64 encoding and decoding (RFC 4648).

### `b64.encode`

`encoding.b64.encode(s)`

Encodes `s` using standard Base64 (RFC 4648) and returns the result as a string.

```frost
encoding.b64.encode('Hello')
# => SGVsbG8=
```

See also:
[`b64.decode`](encoding.md#b64decode)

### `b64.decode`

`encoding.b64.decode(s)`

Decodes a standard Base64 (RFC 4648) string and returns the raw bytes as a string.

Produces an error on invalid input.

```frost
encoding.b64.decode('SGVsbG8=')
# => Hello
```

See also:
[`b64.encode`](encoding.md#b64encode)

### `b64.urlencode`

`encoding.b64.urlencode(s)`

Encodes `s` using URL-safe Base64 and returns the result as a string.

See also:
[`b64.urldecode`](encoding.md#b64urldecode)

### `b64.urldecode`

`encoding.b64.urldecode(s)`

Decodes a URL-safe Base64 string and returns the raw bytes as a string.

Produces an error on invalid input.

See also:
[`b64.urlencode`](encoding.md#b64urlencode)

## hex

Hexadecimal encoding and decoding.

### `hex.encode`

`encoding.hex.encode(s)`

Encodes `s` as a lowercase hexadecimal string. Each byte becomes two hex characters.

```frost
encoding.hex.encode('Hi')
# => 4869
```

See also:
[`hex.decode`](encoding.md#hexdecode)

### `hex.decode`

`encoding.hex.decode(s)`

Decodes a hexadecimal string and returns the raw bytes as a string. Input length must be even. Produces an error on invalid input.

The result is a raw binary string that may contain non-printable bytes. Printing such strings directly may produce unexpected terminal output.

```frost
encoding.hex.decode('4869')
# => Hi
```

See also:
[`hex.encode`](encoding.md#hexencode)

## url

URL percent-encoding (RFC 3986).

### `url.encode`

`encoding.url.encode(s)`

Percent-encodes `s` per RFC 3986. Unreserved characters (A-Z, a-z, 0-9, `-`, `.`, `_`, `~`) are not encoded.

```frost
encoding.url.encode('hello world')
# => hello%20world
```

See also:
[`url.decode`](encoding.md#urldecode)

### `url.decode`

`encoding.url.decode(s)`

Decodes a percent-encoded string.

Produces an error on invalid input (e.g. `%GG`, truncated `%`).

```frost
encoding.url.decode('hello%20world')
# => hello world
```

See also:
[`url.encode`](encoding.md#urlencode)

