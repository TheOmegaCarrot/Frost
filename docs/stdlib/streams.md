# Streams

## Reader

Not all reader streams support every method below.
Each function's entry specifies its supported subset.

### `.read_line`
`.read_line()`

### `.read_one`
`.read_one()`

### `.read_rest`
`.read_rest()`

### `.eof`
`.eof()`

### `.tell`
`.tell()`

### `.seek`
`.seek(pos)`

### `.is_open`
`.is_open()`

### `.close`
`.close()`

## Writer

Not all writer streams support every method below.
Each function's entry specifies its supported subset.

### `.write`
`.write(s)`

### `.writeln`
`.writeln(s)`

### `.tell`
`.tell()`

### `.seek`
`.seek(pos)`

### `.flush`
`.flush()`

### `.is_open`
`.is_open()`

### `.close`
`.close()`

---

## `open_read`
`open_read(path)`

Returns a [Reader](#reader) with full support for all reader methods.

## `open_trunc`
`open_trunc(path)`

Returns a [Writer](#writer) with full support for all writer methods.

## `open_append`
`open_append(path)`

Returns a [Writer](#writer) with full support for all writer methods.

## `stringreader`
`stringreader(s)`

Returns a [Reader](#reader). Supports `.read_line`, `.read_one`, `.read_rest`, `.eof`, `.tell`, `.seek`.

## `stringwriter`
`stringwriter()`

Returns a [Writer](#writer). Supports `.write`, `.writeln`, `.tell`, `.seek`, plus `.get`.

### `.get`
`.get()`

## `stdin`

A pre-defined reader. Supports `.read_line`, `.read_one`, `.read`.

### `.read`
`.read()`

## `stderr`

A pre-defined writer. Supports `.write`, `.writeln`.
