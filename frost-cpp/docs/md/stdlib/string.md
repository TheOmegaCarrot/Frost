# String

```frost
def str = import('std.string')
```

Additional string operations beyond the [global string builtins](strings.md).

## `index_of`

`str.index_of(s, substr)`

Returns the index of the first occurrence of `substr` in `s`, or `null` if not found.

```frost
str.index_of('hello world', 'world')
# => 6
```

```frost
str.index_of('hello', 'xyz')
# => null
```

## `last_index_of`

`str.last_index_of(s, substr)`

Returns the index of the last occurrence of `substr` in `s`, or `null` if not found.

```frost
str.last_index_of('abcabc', 'bc')
# => 4
```

## `count`

`str.count(s, substr)`

Counts the number of non-overlapping occurrences of `substr` in `s`. `substr` must not be empty.

```frost
str.count('banana', 'an')
# => 2
```

```frost
str.count('aaa', 'aa')
# => 1
```

## `chars`

`str.chars(s)`

Splits `s` into an `Array` of single-byte strings.

```frost
str.chars('abc')
# => [ "a", "b", "c" ]
```

```frost
str.chars('')
# => []
```

## `is_empty`

`str.is_empty(s)`

Returns `true` if `s` is the empty string.

## `is_ascii`

`str.is_ascii(s)`

Returns `true` if all bytes in `s` are ASCII (< 128). An empty string returns `true` (vacuous truth).

## `is_digit`

`str.is_digit(s)`

Returns `true` if all bytes in `s` are ASCII digits (`0`-`9`). An empty string returns `true` (vacuous truth).

## `is_alpha`

`str.is_alpha(s)`

Returns `true` if all bytes in `s` are ASCII letters (`a`-`z`, `A`-`Z`). An empty string returns `true`.

## `is_alphanumeric`

`str.is_alphanumeric(s)`

Returns `true` if all bytes in `s` are ASCII letters or digits. An empty string returns `true`.

## `is_whitespace`

`str.is_whitespace(s)`

Returns `true` if all bytes in `s` are ASCII whitespace. An empty string returns `true`.

## `is_uppercase`

`str.is_uppercase(s)`

Returns `true` if all ASCII letters in `s` are uppercase. Non-letter bytes are ignored.

## `is_lowercase`

`str.is_lowercase(s)`

Returns `true` if all ASCII letters in `s` are lowercase. Non-letter bytes are ignored.

## `pad_left`

`str.pad_left(s, width)`
`str.pad_left(s, width, fill)`

Pads `s` on the left to the given `width`. `fill` defaults to space. `fill` must be a single-byte string. If `s` is already at or beyond `width`, it is returned unchanged.

```frost
str.pad_left('42', 5, '0')
# => 00042
```

```frost
str.pad_left('42', 5)
# =>    42
```

## `pad_right`

`str.pad_right(s, width)`
`str.pad_right(s, width, fill)`

Pads `s` on the right to the given `width`. `fill` defaults to space. `fill` must be a single-byte string.

```frost
str.pad_right('hi', 5, '.')
# => hi...
```

```frost
str.pad_right('hi', 5)
# => hi   
```

## `center`

`str.center(s, width)`
`str.center(s, width, fill)`

Centers `s` within the given `width`. `fill` defaults to space. When the padding is odd, the extra byte goes on the right.

```frost
str.center('hi', 6, '-')
# => --hi--
```

## `repeat`

`str.repeat(s, n)`

Returns `s` repeated `n` times. `n` must not be negative.

```frost
str.repeat('ab', 3)
# => ababab
```

```frost
str.repeat('ha', 0)
# => 
```

