# Encoding

```frost
def enc = import('std.encoding')
```

## `fmt_int`
`fmt_int(number, base)`

Returns a lowercase string representation of `number` in the given `base`.
`base` must be in the range `[2, 36]`.

## `parse_int`
`parse_int(number, base)`

Parses `number` as an integer in the given `base` and returns an `Int`.
`base` must be in the range `[2, 36]`.
Produces an error if the string is not a valid integer in that base, or if the value is out of `Int` range.

## `to_bytes`
`to_bytes(s)`

Returns an array of `Int` values representing the raw byte values (0--255) of each character in `s`.

## `from_bytes`
`from_bytes(arr)`

Converts an array of `Int` values in the range `[0, 255]` to a string.
Produces an error if any element is not an `Int` or is out of range.

## `b64.encode`
`b64.encode(s)`

Encodes `s` using standard Base64 (RFC 4648) and returns the result as a string.

## `b64.decode`
`b64.decode(s)`

Decodes a standard Base64 (RFC 4648) string and returns the raw bytes as a string.
Produces an error on invalid input.

## `b64.urlencode`
`b64.urlencode(s)`

Encodes `s` using URL-safe Base64 and returns the result as a string.

## `b64.urldecode`
`b64.urldecode(s)`

Decodes a URL-safe Base64 string and returns the raw bytes as a string.
Produces an error on invalid input.
