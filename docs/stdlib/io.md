# IO

```frost
def io = import('std.io')
```

Streams for reading and writing files and strings.

## Reader

Not all reader streams support every method below.
Each stream's entry below specifies its supported subset.

### `.read_line`
`.read_line()`

Reads and returns one line as a `String`, not including the trailing newline.
Returns `null` at EOF.

### `.read_one`
`.read_one()`

Reads and returns a single character as a `String`.
Returns `null` at EOF.

### `.read_rest`
`.read_rest()`

Reads and returns all remaining content as a `String`.

### `.eof`
`.eof()`

Returns `true` if the stream is at end of file.

### `.tell`
`.tell()`

Returns the current byte position in the stream as an `Int`.

### `.seek`
`.seek(pos)`

Sets the current position to byte offset `pos`.

### `.is_open`
`.is_open()`

Returns `true` if the stream is open.

### `.close`
`.close()`

Closes the stream.
Streams are automatically closed once there is no reference to them.

---

## Writer

Not all writer streams support every method below.
Each stream's entry below specifies its supported subset.

### `.write`
`.write(s)`

Writes the string `s` to the stream.

### `.writeln`
`.writeln(s)`

Writes the string `s` followed by a newline to the stream.

### `.tell`
`.tell()`

Returns the current byte position in the stream as an `Int`.

### `.seek`
`.seek(pos)`

Sets the current position to byte offset `pos`.

### `.flush`
`.flush()`

Flushes any buffered output to the underlying resource.

### `.is_open`
`.is_open()`

Returns `true` if the stream is open.

### `.close`
`.close()`

Closes the stream.
Streams are automatically closed once there is no reference to them.

---

## `open_read`
`open_read(path)`

Opens the file at `path` for reading.
Returns `null` if the file cannot be opened.
Returns a [Reader](#reader) with full support for all reader methods.

## `open_trunc`
`open_trunc(path)`

Opens the file at `path` for writing, truncating it if it exists and creating it if it does not.
Returns `null` if the file cannot be opened.
Returns a [Writer](#writer) with full support for all writer methods.

## `open_append`
`open_append(path)`

Opens the file at `path` for appending.
New writes are added after existing content.
Returns `null` if the file cannot be opened.
Returns a [Writer](#writer) with full support for all writer methods.

## `stringreader`
`stringreader(s)`

Creates a reader backed by the string `s`.
Returns a [Reader](#reader).
Supports `.read_line`, `.read_one`, `.read_rest`, `.eof`, `.tell`, `.seek`.

## `stringwriter`
`stringwriter()`

Creates an in-memory writer.
Returns a [Writer](#writer).
Supports `.write`, `.writeln`, `.tell`, `.seek`, plus `.get`.

### `.get`
`.get()`

Returns the accumulated content as a `String`.

---

**Note:** `stdin`, `stdout`, and `stderr` are available as global builtins without importing this module.
See [Streams](./streams.md) for their documentation.
