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
