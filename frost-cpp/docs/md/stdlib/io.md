# IO

```frost
def io = import('std.io')
```

Convenience functions for simple file IO and stream-based readers/writers for more complex use cases.

[`stdin`](streams.md#stdin), [`stdout`](streams.md#stdout), and [`stderr`](streams.md#stderr) are available as global builtins without importing this module.

## Reader

A stream for reading data. Not all reader streams support every method below. Each stream constructor specifies its supported subset.

### `reader.read_line`

`reader.read_line()`

Reads and returns one line as a `String`, not including the trailing newline. Returns `null` at EOF.

### `reader.read_one`

`reader.read_one()`

Reads and returns a single character as a `String`. Returns `null` at EOF.

### `reader.read_rest`

`reader.read_rest()`

Reads and returns all remaining content as a `String`.

### `reader.eof`

`reader.eof()`

Returns `true` if the stream is at end of file.

### `reader.tell`

`reader.tell()`

Returns the current byte position in the stream as an `Int`.

### `reader.seek`

`reader.seek(pos)`

Sets the current position to byte offset `pos`.

### `reader.is_open`

`reader.is_open()`

Returns `true` if the stream is open.

### `reader.close`

`reader.close()`

Closes the stream. Streams are automatically closed once there is no reference to them.

## Writer

A stream for writing data. Not all writer streams support every method below. Each stream constructor specifies its supported subset.

### `writer.write`

`writer.write(s)`

Writes the string `s` to the stream.

### `writer.writeln`

`writer.writeln(s)`

Writes the string `s` followed by a newline to the stream.

### `writer.tell`

`writer.tell()`

Returns the current byte position in the stream as an `Int`.

### `writer.seek`

`writer.seek(pos)`

Sets the current position to byte offset `pos`.

### `writer.flush`

`writer.flush()`

Flushes any buffered output to the underlying resource.

### `writer.is_open`

`writer.is_open()`

Returns `true` if the stream is open.

### `writer.close`

`writer.close()`

Closes the stream. Streams are automatically closed once there is no reference to them.

## `read`

`io.read(path)`

Reads the entire file at `path` and returns its contents as a `String`. Produces an error if the file cannot be opened.

## `write`

`io.write(path, content)`

Writes `content` to the file at `path`, replacing any existing content. Creates the file if it does not exist. Returns `null`. Produces an error if the file cannot be opened.

## `append`

`io.append(path, content)`

Appends `content` to the end of the file at `path`. Creates the file if it does not exist. Returns `null`. Produces an error if the file cannot be opened.

## `open_read`

`io.open_read(path)`

Opens the file at `path` for reading. Returns a Reader with full support for all reader methods. Produces an error if the file cannot be opened.

See also:
[`open_trunc`](io.md#open_trunc)
[`open_append`](io.md#open_append)

## `open_trunc`

`io.open_trunc(path)`

Opens the file at `path` for writing, truncating it if it exists and creating it if it does not. Returns a Writer with full support for all writer methods. Produces an error if the file cannot be opened.

See also:
[`open_read`](io.md#open_read)
[`open_append`](io.md#open_append)

## `open_append`

`io.open_append(path)`

Opens the file at `path` for appending. New writes are added after existing content. Returns a Writer with full support for all writer methods. Produces an error if the file cannot be opened.

See also:
[`open_read`](io.md#open_read)
[`open_trunc`](io.md#open_trunc)

## `stringreader`

`io.stringreader(s)`

Creates a reader backed by the string `s`. Returns a Reader supporting `.read_line`, `.read_one`, `.read_rest`, `.eof`, `.tell`, `.seek`.

See also:
[`stringwriter`](io.md#stringwriter)

## `stringwriter`

`io.stringwriter()`

Creates an in-memory writer. Returns a Writer supporting `.write`, `.writeln`, `.tell`, `.seek`, plus `.get`.

### `stringwriter.get`

`stringwriter.get()`: returns the accumulated content as a `String`.

See also:
[`stringreader`](io.md#stringreader)

