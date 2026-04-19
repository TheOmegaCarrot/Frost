# String

```frost
def str = import('std.string')
```

Additional string operations beyond the [global string builtins](./strings.md).

## Substring operations

### `index_of`

`index_of(s, substr)`

Returns the index of the first occurrence of `substr` in `s`, or `null` if not found.

```frost
str.index_of("hello world", "world")  # => 6
str.index_of("hello", "xyz")          # => null
```

### `last_index_of`

`last_index_of(s, substr)`

Returns the index of the last occurrence of `substr` in `s`, or `null` if not found.

```frost
str.last_index_of("abcabc", "bc")  # => 4
```

### `count`

`count(s, substr)`

Counts the number of non-overlapping occurrences of `substr` in `s`.
`substr` must not be empty.

```frost
str.count("banana", "an")  # => 2
str.count("aaa", "aa")     # => 1
```

### `chars`

`chars(s)`

Splits `s` into an `Array` of single-byte strings.

```frost
str.chars("abc")  # => ["a", "b", "c"]
str.chars("")     # => []
```
