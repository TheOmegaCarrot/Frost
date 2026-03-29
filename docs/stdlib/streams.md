# Streams

Pre-defined streams available without imports.
See [IO](./io.md) for file and string stream constructors.

## `stdin`

A pre-defined [reader](./io.md#reader) backed by standard input.
Supports `.read_line`, `.read_one`, `.read`, `.read_rest`.

### `.read`
`.read()`

Alias for `.read_rest()`.

### `.read_rest`
`.read_rest()`

Reads all remaining input from stdin as a `String`.

## `stdout`

A pre-defined [writer](./io.md#writer) backed by standard output.
Supports `.write`, `.writeln`.

## `stderr`

A pre-defined [writer](./io.md#writer) backed by standard error.
Supports `.write`, `.writeln`.
