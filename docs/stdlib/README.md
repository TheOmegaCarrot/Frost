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
| [CLI](./cli.md)                  | `import('std.cli')`          | Building command-line tools              |
| [Encoding](./encoding.md)        | `import('std.encoding')`     | Binary and text encoding utilities        |
| [Filesystem](./fs.md)            | `import('std.fs')`           | Reading, writing, and navigating the filesystem |
| [IO](./io.md)                    | `import('std.io')`           | Reading and writing files and streams    |
| [JSON](./json.md)                | `import('std.json')`         | Encoding and decoding JSON               |
| [Math](./math.md)                | `import('std.math')`         | Arithmetic, rounding, logarithms, trig   |
| [OS](./os.md)                    | `import('std.os')`           | Environment variables, process control   |
| [Random](./random.md)            | `import('std.random')`       | Pseudo-random numbers, shuffling |
| [Regex](./regex.md)              | `import('std.regex')`        | Pattern matching, searching, replacement |

## Extensions

Extensions are optional components that are auto-detected at build time.
Each extension has a `WITH_<NAME>` CMake setting with three values:

- **`AUTO`** (default) -- enabled if dependencies are found, skipped otherwise
- **`ON`** -- enabled, hard error if dependencies are missing
- **`OFF`** -- disabled entirely

The configure output shows colored status for each extension and its dependencies.
Each extension's documentation describes exactly how it handles its dependencies.

| Extension                        | Import                        | Description                         | Dependencies              |
|----------------------------------|-------------------------------|-------------------------------------|---------------------------|
| [Compression](./compression.md)  | `import('ext.compression')`   | Compression and decompression       | zlib, bz2, brotli, zstd   |
| [Hash](./hash.md)                | `import('ext.hash')`          | Hash functions, checksums, and HMAC | OpenSSL, zlib             |
| [HTTP](./http.md)                | `import('ext.http')`          | Making HTTP requests                | OpenSSL                   |
| [SQLite](./sqlite.md)            | `import('ext.sqlite')`        | Embedded relational database        | SQLite (vendored), zlib   |
| [Unsafe](./unsafe.md)            | `import('ext.unsafe')`        | Bypass Frost's safety guarantees    | None                      |
