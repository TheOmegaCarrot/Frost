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

## `hex.encode`
`hex.encode(s)`

Encodes `s` as a lowercase hexadecimal string. Each byte becomes two hex characters.

## `hex.decode`
`hex.decode(s)`

Decodes a hexadecimal string and returns the raw bytes as a string.
Input length must be even. Produces an error on invalid input.
The result is a raw binary string that may contain non-printable bytes.
Printing such strings directly may produce unexpected terminal output.

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

## `url.encode`
`url.encode(s)`

Percent-encodes `s` per RFC 3986. Unreserved characters (`A-Z a-z 0-9 - . _ ~`) are not encoded.

## `url.decode`
`url.decode(s)`

Decodes a percent-encoded string. Produces an error on invalid input (e.g. `%GG`, truncated `%`).
