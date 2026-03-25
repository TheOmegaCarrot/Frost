# Streams

Pre-defined streams available without imports.
See [IO](./io.md) for file and string stream constructors.

## `stdin`

A pre-defined [reader](./io.md#reader) backed by standard input.
Supports `.read_line`, `.read_one`, `.read`.

### `.read`
`.read()`

Reads all remaining input from stdin as a `String`.

## `stderr`

A pre-defined [writer](./io.md#writer) backed by standard error.
Supports `.write`, `.writeln`.
