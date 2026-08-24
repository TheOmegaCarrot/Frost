# Streams

Pre-defined streams available without imports. See [`IO`](io.md) for file and string stream constructors.

## `stdin`

A pre-defined [`Reader`](io.md#reader) backed by standard input. Supports `.read_line`, `.read_one`, `.read`, `.read_rest`.

### `stdin.read`

`stdin.read()`: alias for `.read_rest()`.

### `stdin.read_rest`

`stdin.read_rest()`: reads all remaining input from stdin as a `String`.

## `stdout`

A pre-defined [`Writer`](io.md#writer) backed by standard output. Supports `.write`, `.writeln`.

## `stderr`

A pre-defined [`Writer`](io.md#writer) backed by standard error. Supports `.write`, `.writeln`.

