# INI

```frost
def ini = import('ext.ini')
```

Encode and decode INI configuration files. All keys and values are `String`.

## Document Structure

An INI document is represented as a `Map` with two keys:

- `globals`: a `Map` of key-value pairs that appear before any section header.
- `sections`: a `Map` of section names to `Map`s of key-value pairs.

```frost
def doc = ini.decode('''
                     app = myapp

                     [database]
                     host = localhost
                     port = 5432
                     ''')
doc.globals   # => {app: "myapp"}
doc.sections  # => {database: {host: "localhost", port: "5432"}}
```

The same structure is used for encoding:

```frost
ini.encode({
    globals: {app: 'myapp'},
    sections: {
        database: {host: 'localhost', port: '5432'},
    },
})
```

Result:

```ini
app = myapp

[database]
host = localhost
port = 5432
```

## Decode Options

`decode` accepts an optional `options` map with the following fields (all default to sensible values):

|  Key | Type | Default | Description  |
| ---|---|---|--- |
|  `strip_quotes` | `Bool` | `true` | Strip surrounding `"` or `'` from values.  |
|  `escapes` | `Bool` | `true` | Interpret backslash escapes (e.g. `\n`, `\t`).  |
|  `multiline_values` | `Bool` | `false` | Allow values to span multiple indented lines.  |

## Encode Options

`encode` accepts an optional `options` map:

|  Key | Type | Default | Description  |
| ---|---|---|--- |
|  `kv_separator` | `String` | `" = "` | Separator between keys and values. Common choices: `"="`, `" = "`, `": "`.  |
|  `line_separator` | `String` | `"LF"` | Line ending style: `"LF"` (`\n`) or `"CRLF"` (`\r\n`).  |

## Limitations

- Comments are stripped on decode and not preserved for round-tripping.
- When duplicate keys appear within a section, it is unspecified which key is preserved.
- All values are strings. The caller is responsible for type conversion.

## `decode`

`ini.decode(s)`
`ini.decode(s, options)`

Parses an INI document from `String` `s` and returns a `Map` with `globals` and `sections`. Produces an error on malformed input.

```frost
def config = ini.decode('
app=myapp

[server]
host=0.0.0.0
port=8080

[logging]
level=info
')
config.globals.app         # => "myapp"
config.sections.server.host  # => "0.0.0.0"
```

### Disabling escape interpretation

```frost
ini.decode('[s]\npath=C:\\Users\\me', {escapes: false})
```

See also:
[`encode`](ini.md#encode)

## `encode`

`ini.encode(doc)`
`ini.encode(doc, options)`

Serializes a `Map` with `globals` and `sections` to an INI `String`. All keys and values must be `String`. Non-string keys or values produce an error.

```frost
ini.encode({
    globals: {},
    sections: {
        database: {host: 'localhost', port: '5432'},
    },
})
```

### Custom separator

```frost
ini.encode(doc, {kv_separator: ': '})
```

Produces `key: value` instead of `key = value`.

See also:
[`decode`](ini.md#decode)

