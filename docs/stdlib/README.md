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
| [Strings](./strings.md)             | Splitting, joining, case conversion, searching, and encoding               |
| [Operators](./operators.md)         | Arithmetic and comparison operators as first-class functions               |
| [Output](./output.md)               | Printing to stdout and string formatting                                   |
| [Streams](./streams.md)             | Reading and writing files, stdin, and stdout                               |
| [Functions](./functions.md)         | Error handling and higher-order utilities                                   |
| [Debug](./debug.md)                 | Assertions and value inspection                                            |
| [Mutable Cell](./mutable-cell.md)   | Mutable state                                                              |
| [OS](./os.md)                       | Interacting with the operating system                                      |
| [Filesystem](./filesystem.md)       | Reading, writing, and navigating the filesystem                            |

## Standard Modules

Built-in modules accessed via `import`.

| Module                           | Import                       | Description                              |
|----------------------------------|------------------------------|------------------------------------------|
| [JSON](./json.md)                | `import('std.json')`         | Encoding and decoding JSON               |
| [Math](./math.md)                | `import('std.math')`         | Arithmetic, rounding, logarithms, trig   |
| [Regex](./regex.md)              | `import('std.regex')`        | Pattern matching, searching, replacement |

## Extensions

Extensions are optional components enabled at build time.
They are not available in all Frost builds, but are enabled by default.

| Extension                  | Description                      | Dependencies              |
|----------------------------|----------------------------------|---------------------------|
| [HTTP](./http.md)          | Making HTTP requests             | Boost `url`, OpenSSL      |
| [SQLite](./sqlite.md)      | Embedded relational database     | None (vendored)           |
| [Unsafe](./unsafe.md)      | Bypass Frost's safety guarantees | None                      |
