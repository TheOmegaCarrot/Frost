# CSV

```frost
def csv = import('ext.csv')
```

Read and write CSV (comma-separated values) data from files and strings.

All field values are `String`. The caller is responsible for type conversion (e.g. `to_int`).

## Read Options

Read functions take a required `options` map:

- `headers` (`Bool`, required): when `true`, the first row is treated as column names and data rows are returned as `Map`s. When `false`, all rows are returned as `Array`s.
- `delim` (`String`, optional): field delimiter. Must be a single byte. Defaults to `","`.

## Write Options

Write functions take an optional `options` map:

- `headers` (`Array` of `String`, optional): column names. When provided, a header row is written immediately. Enables `Map`-based row writing.
- `delim` (`String`, optional): field delimiter. Must be a single byte. Defaults to `","`.

## Callback Mode

Read functions accept an optional callback as the last argument. When provided, the callback is called with each row and the results are collected:

```frost
csv.read_str("val\n10\n20\n30", {headers: true}, fn row -> to_int(row.val))
```

Result: `[10, 20, 30]`.

## CSV Writer

A writer object returned by `write_file` or `write_str`. File writers have `row` and `flush`. String writers have `row` and `get`.

### `writer.row`

`writer.row(values)`

Writes a single row. `values` may be an `Array` of primitives (positional) or, when headers are defined, a `Map` keyed by header names. All values are stringified. Non-primitive values produce an error.

### `writer.flush`

`writer.flush()`

Flushes buffered data to the file. Only on file writers. The writer remains usable.

### `writer.get`

`writer.get()`

Returns the accumulated CSV data as a `String`. Only on string writers. The writer remains usable; subsequent rows append.

## `read_file`

`csv.read_file(path, options)`
`csv.read_file(path, options, callback)`

Reads CSV data from the file at `path`. Returns an `Array` of rows. With a callback, returns an `Array` of callback results. Produces an error if the file cannot be opened.

```frost
csv.read_file('./data.csv', {headers: true})
csv.read_file('./data.csv', {headers: true}, fn row -> row.name)
```

See also:
[`read_str`](csv.md#read_str)

## `read_str`

`csv.read_str(text, options)`
`csv.read_str(text, options, callback)`

Parses CSV data from the string `text`. Returns an `Array` of rows. With a callback, returns an `Array` of callback results.

```frost
csv.read_str("name,age\nAlice,30", {headers: true})
```

Result: `[{name: "Alice", age: "30"}]`.

See also:
[`read_file`](csv.md#read_file)

## `write_file`

`csv.write_file(path)`
`csv.write_file(path, options)`

Creates a CSV file at `path` and returns a writer object. The file is created or truncated. If `headers` is specified in options, the header row is written immediately. Produces an error if the file cannot be opened.

```frost
def w = csv.write_file('./out.csv', {headers: ['name', 'age']})
w.row(['Alice', '30'])
w.row({name: 'Bob', age: '25'})
w.flush()
```

See also:
[`write_str`](csv.md#write_str)

## `write_str`

`csv.write_str()`
`csv.write_str(options)`

Creates an in-memory CSV writer and returns a writer object. If `headers` is specified in options, the header row is written immediately.

```frost
def w = csv.write_str({headers: ['x', 'y']})
w.row(['1', '2'])
w.get()
```

Result: `"x,y\n1,2\n"`.

See also:
[`write_file`](csv.md#write_file)

