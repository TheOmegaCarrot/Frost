# SQLite

```frost
def sqlite = import('ext.sqlite')
```

Build flag: `WITH_SQLITE` (default: `AUTO`).
Frost vendors its own SQLite build, so no system SQLite installation is required.
The SQLar and zipfile extensions require zlib; if zlib is not found, these features are omitted.

The vendored build enables the following non-default compile options:

- [**DEFAULT_FOREIGN_KEYS=1**](https://www.sqlite.org/compile.html#default_foreign_keys) — foreign key enforcement is on by default
- [**DEFAULT_MEMSTATUS=0**](https://www.sqlite.org/compile.html#default_memstatus) — disables memory usage tracking for faster allocations
- [**LIKE_DOESNT_MATCH_BLOBS**](https://www.sqlite.org/compile.html#like_doesnt_match_blobs) — LIKE/GLOB return FALSE for BLOB operands
- [**MAX_EXPR_DEPTH=0**](https://www.sqlite.org/compile.html#max_expr_depth) — no limit on expression depth
- [**STAT4**](https://www.sqlite.org/compile.html#enable_stat4) — collects histogram data for improved query planner decisions
- [**TEMP_STORE=2**](https://www.sqlite.org/compile.html#temp_store) — temporary tables and indices default to memory
- [**USE_ALLOCA**](https://www.sqlite.org/compile.html#use_alloca) — uses stack allocations for temporaries

The vendored build also enables the following extensions:

- [**CSV virtual table**](https://www.sqlite.org/csv.html)
- [**Decimal**](https://www.sqlite.org/floatingpoint.html#the_decimal_c_extension) — exact decimal arithmetic functions (`decimal_add`, `decimal_mul`, `decimal_sum`, etc.)
- [**dbstat**](https://www.sqlite.org/dbstat.html) — virtual table for storage analysis
- [**FTS5**](https://www.sqlite.org/fts5.html) — full-text search
- [**Math functions**](https://www.sqlite.org/lang_mathfunc.html)
- [**Percentile**](https://www.sqlite.org/lang_aggfunc.html#percentile_disc) — `median()` and `percentile()` aggregate functions
- [**File I/O**](https://www.sqlite.org/cli.html#file_i_o_functions) — `readfile()`, `writefile()`, and the `fsdir` virtual table
- [**generate_series**](https://www.sqlite.org/series.html) — table-valued function for generating integer sequences
- [**REGEXP**](https://www.sqlite.org/lang_expr.html#regexp) — enables the `REGEXP` operator for pattern matching
- [**SHA3**](https://www.sqlite.org/sha3query.html) — `sha3()` and `sha3_query()` hash functions
- [**Soundex**](https://www.sqlite.org/lang_corefunc.html#soundex) — `soundex()` function for phonetic string matching
- [**SQLAR**](https://www.sqlite.org/sqlar.html) — `sqlar_compress()` and `sqlar_uncompress()` for working with SQLite Archive files
- [**UUID**](https://www.sqlite.org/src/file?name=ext/misc/uuid.c) — `uuid()`, `uuid_str()`, and `uuid_blob()` for generating and converting UUIDs
- [**Zipfile**](https://www.sqlite.org/zipfile.html) — virtual table for reading and writing ZIP archives

The `sqlite` module provides access to SQLite databases.
Supported binding types are `Null`, `Int`, `Float`, `Bool`, `String`, and BLOB foreign values.
`Bool` values are stored as integers (`true` → `1`, `false` → `0`).

SQLite types map to Frost types as follows: `INTEGER` → `Int`, `REAL` → `Float`, `TEXT` → `String`, `NULL` → `null`, and `BLOB` → foreign value (see below).

### BLOB handling

SQLite BLOBs are returned as [foreign values](./foreign-values.md).
Call the foreign value to get the raw bytes as a `String`.
Pass it through unchanged to preserve the BLOB type when binding.

```frost
def [row, ..._] = db.query('SELECT blob_col FROM t')
row.blob_col     # => <Function>
row.blob_col()   # => the raw bytes as a String

# Round-trip: re-inserting preserves the BLOB type
db.exec('INSERT INTO t2 VALUES (?)', [row.blob_col])
```

To create a BLOB from scratch, use `sqlite.blob()`:

```frost
def b = sqlite.blob('raw bytes here')
db.exec('INSERT INTO t VALUES (?)', [b])
```

A plain `String` always binds as TEXT. Use `sqlite.blob()` when you need BLOB binding specifically.

### Positional bindings

Use `?` placeholders and pass an `Array` of values.

```frost
db.exec('INSERT INTO t VALUES (?, ?)', [1, 'alice'])
```

### Named bindings

Use `:name`, `@name`, or `$name` placeholders and pass a `Map` with string keys.
The keys in the map correspond to the parameter names without the prefix.

```frost
db.exec('INSERT INTO t VALUES (:id, :name)', {id: 1, name: 'alice'})
db.query('SELECT * FROM t WHERE id > @min', {min: 0})
```

All methods that accept bindings (`exec`, `query`, `each`, `collect`) support both forms.

There are no explicit prepared statements.
For repeated operations, wrap a parameterized call in a function:

```frost
defn insert_user(db, id, name) -> db.exec('INSERT INTO users VALUES (?, ?)', [id, name])

insert_user(db, 1, 'alice')
insert_user(db, 2, 'bob')
```

## `version`

A `String` containing the SQLite library version (e.g. `"3.51.3"`).

## `blob`

`blob(data)`

Creates a BLOB foreign value from a `String`.
When bound as a parameter, this value uses `BLOB` affinity rather than `TEXT`.
Calling this does not copy the underlying string data.

```frost
def b = sqlite.blob('binary data')
db.exec('INSERT INTO t VALUES (?)', [b])
```

## `open`
`open(path)`

Opens a read-write SQLite database at `path`, or creates one if it does not exist.
Returns a database object, the methods of which are documented below as `db.*`.
Produces an error if the database cannot be opened.

## `open_readonly`
`open_readonly(path)`

Opens an existing SQLite database at `path` in read-only mode.
Returns a database object.
Produces an error if the file does not exist.
Write operations (`INSERT`, `UPDATE`, `DELETE`, DDL) will fail.

## `open_memory`
`open_memory()`

Opens a private in-memory database.
Returns a database object.
The database exists only for the lifetime of the returned object.

## `db.exec`
`db.exec(sql)`
`db.exec(sql, bindings)`

Executes a single SQL statement and returns the number of rows affected (`Int`).
DDL statements return `0`.
Multi-statement SQL is rejected.

```frost
db.exec('CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)')
db.exec('INSERT INTO t VALUES (?, ?)', [1, 'alice'])                    # => 1
db.exec('UPDATE t SET name = :name WHERE id = :id', {name: 'x', id: 1}) # => 1
```

## `db.script`
`db.script(sql)`

Executes one or more SQL statements separated by semicolons.
Returns the total number of rows affected across all statements.
Does not accept parameter bindings.

Without a wrapping [`db.transaction`](#dbtransaction), each statement is committed individually.
If a statement fails partway through a script, earlier statements will have already committed.

```frost
db.script('CREATE TABLE t (id INTEGER); INSERT INTO t VALUES (1); INSERT INTO t VALUES (2)')  # => 2
```

## `db.query`
`db.query(sql)`
`db.query(sql, bindings)`

Executes a single SQL statement and returns all result rows as an `Array` of `Map`s.
Each map has column names as keys.

```frost
db.query('SELECT * FROM t WHERE id > ?', [0])
# => [{id: 1, name: 'alice'}, {id: 2, name: 'bob'}]
```

SQLite types map to Frost types: `INTEGER` → `Int`, `REAL` → `Float`, `TEXT` → `String`, `BLOB` → `String`, `NULL` → `null`.

All result rows are loaded into memory at once.
For large result sets, prefer [`db.each`](#dbeach) or [`db.collect`](#dbcollect) which process one row at a time.
Note that even with row-at-a-time processing, each individual column value is fully materialized in memory.
Databases containing very large `BLOB` or `TEXT` values (e.g. SQLAR archives) should be handled with care.

## `db.each`
`db.each(sql, callback)`
`db.each(sql, bindings, callback)`

Executes a query and calls `callback` with each row.
Only one row is in memory at a time, making this suitable for large result sets.
Returns `null`.

```frost
db.each('SELECT * FROM src', fn row -> db.exec('INSERT INTO dst VALUES (?)', [row.x]))
```

## `db.collect`
`db.collect(sql, callback)`
`db.collect(sql, bindings, callback)`

Executes a query, calls `callback` with each row, and collects the return values into an `Array`.
Rows are processed one at a time, but the collected results are accumulated in memory.

```frost
db.collect('SELECT x FROM t', fn row -> row.x * 2)  # => [2, 4, 6]
```

## `db.last_insert_rowid`
`db.last_insert_rowid()`

Returns the rowid of the most recent successful `INSERT` on this connection as an `Int`.
If no `INSERT` has been performed, returns `0`.

```frost
db.exec('CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)')
db.exec('INSERT INTO t (name) VALUES (?)', ['alice'])
db.last_insert_rowid()  # => 1
db.exec('INSERT INTO t (name) VALUES (?)', ['bob'])
db.last_insert_rowid()  # => 2
```

Also available as `tx.last_insert_rowid()` inside a transaction.

## `db.transaction`
`db.transaction(callback)`

Executes `callback` inside a SQLite transaction.
The callback receives a transaction object `tx` with the same data methods as `db` (`exec`, `query`, `each`, `collect`, `script`, `last_insert_rowid`).

On normal return, the transaction is committed and `db.transaction` returns the total number of rows affected (`Int`).
If the callback produces an error, the transaction is rolled back and the error propagates.

```frost
db.transaction(fn tx -> {
    tx.exec('INSERT INTO t VALUES (?, ?)', [1, 'alice'])
    tx.exec('INSERT INTO t VALUES (?, ?)', [2, 'bob'])
})
# => 2
```

While a transaction is active, the `db` object cannot be used for data operations.
All data methods on `db` will produce an error until the transaction completes.
Use the `tx` object for all database access within the callback.
The `tx` object is only valid for the duration of the callback.
Immutable bindings prevent accidental misuse, but these safeguards should not be circumvented.

### `tx.exec`, `tx.query`, `tx.each`, `tx.collect`, `tx.script`, `tx.last_insert_rowid`

These have the same signatures and behavior as their `db` counterparts, but operate within the transaction.
Queries through `tx` see uncommitted changes made earlier in the same transaction.
Note that callbacks passed to `tx.each` and `tx.collect` should also use `tx` for any database operations, not `db`.

### Nested transactions

Nested transactions are not supported.
Calling `db.transaction` while a transaction is already active produces an error.

### Errors during a transaction

Any error inside the callback — whether from Frost code or from SQLite (e.g. a constraint violation) — causes an automatic rollback.
The error then propagates to the caller of `db.transaction`.

```frost
def result = try_call(db.transaction, [fn tx -> {
    tx.exec('INSERT INTO t VALUES (?, ?)', [1, 'alice'])
    tx.exec('INSERT INTO t VALUES (?, ?)', [1, 'duplicate'])  # constraint violation
}])
# result.ok == false; both inserts are rolled back
```

## `db.create_function`
`db.create_function(name, function)`

Registers a Frost function as a scalar SQL function with the given name.
The function can accept any number of arguments.
SQLite values are converted to Frost types (`INTEGER` → `Int`, `REAL` → `Float`, `TEXT` → `String`, `NULL` → `null`), and the return value is converted back.

If the Frost function produces an error, it becomes a SQL error, which is then propagated back to the calling Frost code as a recoverable error.

```frost
db.create_function('double', fn x -> x * 2)
db.query('SELECT double(21)')  # => [{["double(21)"]: 42}]

db.create_function('is_even', fn x -> x % 2 == 0)
db.query('SELECT val FROM t WHERE is_even(val)')
```

Registering a function with the same name as an existing one replaces it.

Functions that return `Array`, `Map`, or `Function` values are not supported and will produce a SQL error.

## `db.create_aggregate`
`db.create_aggregate(name, init, step)`
`db.create_aggregate(name, init, step, finalize)`

Registers a user-defined aggregate SQL function with the given name.
The aggregate follows the fold/reduce pattern:

- `init` -- the initial accumulator value (any Frost value)
- `step(acc, ...args)` -- called once per row; receives the current accumulator as the first argument, followed by the SQL column arguments. Returns the new accumulator.
- `finalize(acc)` -- optional; called once after all rows; receives the final accumulator and returns the aggregate result. If omitted, the final accumulator is returned directly as the SQL result.

When no rows match the query, `finalize` is called on `init` (or `init` is returned directly if `finalize` is omitted).

The accumulator can be any Frost value, including `Array` or `Map`, but the value that becomes the SQL result must be a primitive type (`Null`, `Int`, `Float`, `Bool`, or `String`).

Errors in `step` or `finalize` are surfaced as errors. An error in `step` stops processing remaining rows.

Registering an aggregate with the same name as an existing function or aggregate replaces it.

```frost
# Simple sum
db.create_aggregate('sum2', 0, fn (acc, x) -> acc + x)
db.query('SELECT sum2(val) as total FROM t')

# Custom average using an Array accumulator
db.create_aggregate('avg2', [0, 0],
    fn (acc, x) -> [acc[0] + x, acc[1] + 1],
    fn acc -> acc[0] / acc[1])
db.query('SELECT avg2(score) as mean FROM students')
```

## `db.trace`
`db.trace(callback)`
`db.trace(null)`

Registers a callback that is called with the SQL text of each statement before it executes.
Bound parameter values are expanded into the SQL string (e.g., `INSERT INTO t VALUES (42)` rather than `INSERT INTO t VALUES (?)`).
Pass `null` to disable tracing.

Only one trace callback can be active at a time. Registering a new one replaces the previous.

Errors produced by the callback are reported to stderr but do not interrupt SQL execution.
Fatal errors (`fatal`) terminate the program with less grace than usual.

```frost
db.trace(fn sql -> print(sql))
db.exec('INSERT INTO t VALUES (?)', [42])
# prints: INSERT INTO t VALUES (42)

db.trace(null)  # disable tracing
```

## `db.close`
`db.close()`

Closes the database connection.
Produces an error if a transaction is active.
After closing, all operations on `db` produce an error.
