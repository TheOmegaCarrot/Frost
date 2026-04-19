# String

```frost
def str = import('std.string')
```

Additional string operations beyond the [global string builtins](./strings.md).

**Substring operations**

## `index_of`

`index_of(s, substr)`

Returns the index of the first occurrence of `substr` in `s`, or `null` if not found.

```frost
str.index_of("hello world", "world")  # => 6
str.index_of("hello", "xyz")          # => null
```

## `last_index_of`

`last_index_of(s, substr)`

Returns the index of the last occurrence of `substr` in `s`, or `null` if not found.

```frost
str.last_index_of("abcabc", "bc")  # => 4
```

## `count`

`count(s, substr)`

Counts the number of non-overlapping occurrences of `substr` in `s`.
`substr` must not be empty.

```frost
str.count("banana", "an")  # => 2
str.count("aaa", "aa")     # => 1
```

## `chars`

`chars(s)`

Splits `s` into an `Array` of single-byte strings.

```frost
str.chars("abc")  # => ["a", "b", "c"]
str.chars("")     # => []
```

**Character classification**

All classification functions take a `String` and return `Bool`.
An empty string returns `true` (vacuous truth).

## `is_empty`

`is_empty(s)`

Returns `true` if `s` is the empty string.

## `is_ascii`

`is_ascii(s)`

Returns `true` if all bytes in `s` are ASCII (< 128).

## `is_digit`

`is_digit(s)`

Returns `true` if all bytes in `s` are ASCII digits (`0`-`9`).

## `is_alpha`

`is_alpha(s)`

Returns `true` if all bytes in `s` are ASCII letters (`a`-`z`, `A`-`Z`).

## `is_alphanumeric`

`is_alphanumeric(s)`

Returns `true` if all bytes in `s` are ASCII letters or digits.

## `is_whitespace`

`is_whitespace(s)`

Returns `true` if all bytes in `s` are ASCII whitespace (space, tab, newline, etc.).

## `is_uppercase`

`is_uppercase(s)`

Returns `true` if all ASCII letters in `s` are uppercase.
Non-letter bytes are ignored.

```frost
str.is_uppercase("HELLO")     # => true
str.is_uppercase("HELLO123")  # => true
str.is_uppercase("Hello")     # => false
```

## `is_lowercase`

`is_lowercase(s)`

Returns `true` if all ASCII letters in `s` are lowercase.
Non-letter bytes are ignored.

```frost
str.is_lowercase("hello")     # => true
str.is_lowercase("hello123")  # => true
str.is_lowercase("Hello")     # => false
```
