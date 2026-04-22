# Regex

```frost
def regex = import('std.regex')
```

Regex syntax follows the Boost.Regex Perl-compatible dialect.

## `matches`
`matches(string, regex)`

Returns `true` if `regex` matches the entire `string`.

## `contains`
`contains(string, regex)`

Returns `true` if `regex` matches any substring of `string`.

## `replace`
`replace(string, regex, replacement)`

Replaces all non-overlapping matches of `regex` in `string` with `replacement`.
The replacement string supports the following substitutions:

| Syntax      | Replaced with                        |
|-------------|--------------------------------------|
| `$1`, `$2`  | Numbered capture group               |
| `$+{name}`  | Named capture group                  |
| `$&`        | The entire match                     |
| `` $` ``    | The substring preceding the match    |
| `$'`        | The substring following the match    |
| `$$`        | A literal `$`                        |

## `replace_first`
`replace_first(string, regex, replacement)`

Replaces the first match of `regex` in `string` with `replacement`.
Supports the same substitution syntax as `replace`.

## `split`
`split(string, regex)`

Splits `string` around matches of `regex` and returns an `Array` of the substrings between matches.
Returns an empty array for an empty input string.

```frost
regex.split('one,two,three', ',')     # => ['one', 'two', 'three']
regex.split('a1b2c3', R'(\d)')        # => ['a', 'b', 'c']
regex.split('a,,b,,,c', ',+')         # => ['a', 'b', 'c']
regex.split('hello', 'x')             # => ['hello']
```

## `scan_matches`
`scan_matches(string, regex)`

Returns a map describing all matches of `regex` in `string`:

```
{
    found:   Bool,
    count:   Int,
    matches: [
        {
            full:   String,   # the entire match (group 0)
            groups: [
                { matched: Bool, value: String | null, index: Int },
                ...
            ],
            named: {          # only present if the regex contains named groups
                name: { matched: Bool, value: String | null },
                ...
            }
        },
        ...
    ]
}
```
