# Standard Library

An [alphabetical index of all functions](./index.md) is available.

## Parameter Name Conventions

Many functions in the standard library are sufficiently generic that parameter names beyond the type carry little additional meaning.
In these cases, short placeholder names are used in signatures rather than inventing names that would imply unwarranted specificity.

| Name       | Meaning                        |
|------------|--------------------------------|
| `n`        | Any numeric value              |
| `s`        | A `String`                     |
| `arr`      | An `Array`                     |
| `m`        | A `Map`                        |
| `f`        | A `Function`                   |
| `value`    | Any value                      |
| `path`     | A filesystem path (`String`)   |
| `pattern`  | A regex pattern (`String`)     |

## Core

Available in every Frost program without imports.

| Module                              | Description                                                                |
|-------------------------------------|----------------------------------------------------------------------------|
| [Collections](./collections.md)     | Slicing, grouping, sorting, searching, and transforming arrays and maps    |
| [Types](./types.md)                 | Type checking, conversion, and value serialization                         |
| [Strings](./strings.md)             | Splitting, joining, case conversion, and searching                         |
| [Operators](./operators.md)         | Arithmetic and comparison operators as first-class functions               |
| [Output](./output.md)               | Printing to stdout and string formatting                                   |
| [Functions](./functions.md)         | Error handling and higher-order utilities                                   |
| [Streams](./streams.md)             | Pre-defined `stdin` and `stderr` streams                                   |
| [Debug](./debug.md)                 | Assertions and value inspection                                            |
| [Mutable Cell](./mutable-cell.md)   | Mutable state                                                              |

## Standard Modules

Built-in modules accessed via `import`.

| Module                           | Import                       | Description                              |
|----------------------------------|------------------------------|------------------------------------------|
| [Encoding](./encoding.md)        | `import('std.encoding')`     | Binary and text encoding utilities        |
| [Filesystem](./fs.md)            | `import('std.fs')`           | Reading, writing, and navigating the filesystem |
| [Hash](./hash.md)                | `import('std.hash')`         | Cryptographic hash functions (MD5, SHA, BLAKE2, etc.) |
| [IO](./io.md)                    | `import('std.io')`           | Reading and writing files and streams    |
| [JSON](./json.md)                | `import('std.json')`         | Encoding and decoding JSON               |
| [Math](./math.md)                | `import('std.math')`         | Arithmetic, rounding, logarithms, trig   |
| [OS](./os.md)                    | `import('std.os')`           | Environment variables, process control   |
| [Regex](./regex.md)              | `import('std.regex')`        | Pattern matching, searching, replacement |

## Extensions

Extensions are optional components enabled at build time.
They are not available in all Frost builds, but are enabled by default.

| Extension                  | Import                       | Description                      | Extra Dependencies              |
|----------------------------|------------------------------|----------------------------------|---------------------------|
| [HTTP](./http.md)          | `import('ext.http')`         | Making HTTP requests             | None                      |
| [SQLite](./sqlite.md)      | `import('ext.sqlite')`       | Embedded relational database     | SQLite (vendored), zlib   |
| [Unsafe](./unsafe.md)      | `import('ext.unsafe')`       | Bypass Frost's safety guarantees | None                      |
