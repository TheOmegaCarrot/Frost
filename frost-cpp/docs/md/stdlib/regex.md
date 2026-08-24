# Regex

```frost
def regex = import('std.regex')
```

Regular expression matching, replacement, and splitting. Regex syntax follows the Boost.Regex Perl-compatible dialect.

## `matches`

`regex.matches(string, regex)`

Returns `true` if `regex` matches the entire `string`.

## `contains`

`regex.contains(string, regex)`

Returns `true` if `regex` matches any substring of `string`.

## `replace`

`regex.replace(string, regex, replacement)`

Replaces all non-overlapping matches of `regex` in `string` with `replacement`.

The replacement string supports the following substitutions:

|  Syntax | Replaced with  |
| ---|--- |
|  `$1`, `$2` | Numbered capture group  |
|  `$+{name}` | Named capture group  |
|  `$&` | The entire match  |
|  `` $` `` | The substring preceding the match  |
|  `$'` | The substring following the match  |
|  `$$` | A literal `$`  |

## `replace_first`

`regex.replace_first(string, regex, replacement)`

Replaces the first match of `regex` in `string` with `replacement`. Supports the same substitution syntax as `replace`.

See also:
[`replace`](regex.md#replace)

## `replace_with`

`regex.replace_with(string, regex, callback)`

Replaces all full matches of `regex` in `string` by calling `callback` with each match. The callback receives the full match as a `String` and its return value is converted to a string with `to_string`.

```frost
regex.replace_with('hello world', R'(\w+)', to_upper)
# => "HELLO WORLD"

regex.replace_with('a1b2', R'(\d)', fn d -> to_int(d) * 10)
# => "a10b20"
```

## `split`

`regex.split(string, regex)`

Splits `string` around matches of `regex` and returns an `Array` of the substrings between matches. Returns an empty array for an empty input string.

```frost
regex.split('one,two,three', ',')     # => ['one', 'two', 'three']
regex.split('a1b2c3', R'(\d)')       # => ['a', 'b', 'c']
regex.split('a,,b,,,c', ',+')         # => ['a', 'b', 'c']
regex.split('hello', 'x')             # => ['hello']
```

## `scan_matches`

`regex.scan_matches(string, regex)`

Returns a map describing all matches of `regex` in `string`.

```frost
{
    found:   Bool,
    count:   Int,
    matches: [
        {
            full:   String,
            groups: [
                { matched: Bool, value: String or null, index: Int },
            ],
            named: { name: { matched: Bool, value: String or null } },
        },
    ]
}
```

