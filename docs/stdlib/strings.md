# Strings

## `split`
`split(s, delimiter)`

Splits `s` on each occurrence of `delimiter` and returns an array of strings.
If `delimiter` is not found, returns a single-element array containing `s`. An
empty `delimiter` splits into individual bytes.

## `to_upper`
`to_upper(s)`

Returns `s` with all characters converted to uppercase.

## `to_lower`
`to_lower(s)`

Returns `s` with all characters converted to lowercase.

## `contains`
`contains(s, substring)`

Returns `true` if `substring` appears anywhere in `s`.

## `starts_with`
`starts_with(s, prefix)`

Returns `true` if `s` begins with `prefix`.

## `ends_with`
`ends_with(s, suffix)`

Returns `true` if `s` ends with `suffix`.

## `fmt_int`
`fmt_int(number, base)`

Returns a lowercase string representation of `number` in the given `base`.
`base` must be in the range `[2, 36]`.

## `parse_int`
`parse_int(number, base)`

Parses `number` as an integer in the given `base` and returns an `Int`. `base`
must be in the range `[2, 36]`. Produces an error if the string is not a valid
integer in that base, or if the value is out of `Int` range.

## `to_byte_array`
`to_byte_array(s)`

Returns an array of `Int` values representing the raw byte values (0–255) of
each character in `s`.

## `from_byte_array`
`from_byte_array(arr)`

Converts an array of `Int` values in the range `[0, 255]` to a string. Produces
an error if any element is not an `Int` or is out of range.

## `b64_encode`
`b64_encode(s)`

Encodes `s` using standard Base64 (RFC 4648) and returns the result as a string.

## `b64_decode`
`b64_decode(s)`

Decodes a standard Base64 (RFC 4648) string and returns the raw bytes as a
string. Produces an error on invalid input.

## `b64_urlencode`
`b64_urlencode(s)`

Encodes `s` using URL-safe Base64 and returns the result as a string.

## `b64_urldecode`
`b64_urldecode(s)`

Decodes a URL-safe Base64 string and returns the raw bytes as a string. Produces
an error on invalid input.
