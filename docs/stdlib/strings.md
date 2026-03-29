# Strings

## `split`
`split(s, delimiter)`

Splits `s` on each occurrence of `delimiter` and returns an array of strings.
If `delimiter` is not found, returns a single-element array containing `s`.
An empty `delimiter` splits into individual bytes.

## `lines`
`lines(s)`

Splits `s` on each newline (`\n`) and returns an array of strings.
Equivalent to [`split`](#split)`(s, "\n")`.

## `join`
`join(arr, separator)`

Joins an array of strings into a single string, with `separator` inserted between each element.
Returns an empty string for an empty array.

## `replace`
`replace(s, find, replacement)`

Returns `s` with every non-overlapping occurrence of `find` replaced by `replacement`.
Returns `s` unchanged if `find` is not present.

## `trim`
`trim(s)`

Returns `s` with leading and trailing whitespace removed.

## `trim_left`
`trim_left(s)`

Returns `s` with leading whitespace removed.

## `trim_right`
`trim_right(s)`

Returns `s` with trailing whitespace removed.

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

