# SQLite

```frost
def sqlite = import('ext.sqlite')
```

SQLite support can be disabled at build time with `-DWITH_SQLITE=NO`.
Frost vendors its own SQLite build, so no system SQLite installation is required.
The SQLAR extension requires zlib, which is linked from the system.

The vendored build enables the following extensions:

- [**FTS5**](https://www.sqlite.org/fts5.html) — full-text search
- [**Math functions**](https://www.sqlite.org/lang_mathfunc.html)
- [**CSV virtual table**](https://www.sqlite.org/csv.html)
- [**File I/O**](https://www.sqlite.org/cli.html#file_i_o_functions) — `readfile()`, `writefile()`, and the `fsdir` virtual table
- [**SQLAR**](https://www.sqlite.org/sqlar.html) — `sqlar_compress()` and `sqlar_uncompress()` for working with SQLite Archive files

The `sqlite` module provides access to SQLite databases.
Supported binding types are `Null`, `Int`, `Float`, `Bool`, and `String`.
`Bool` values are stored as integers (`true` → `1`, `false` → `0`).

### Positional bindings

Use `?` placeholders and pass an `Array` of values.

```
db.exec('INSERT INTO t VALUES (?, ?)', [1, 'alice'])
```

### Named bindings

Use `:name`, `@name`, or `$name` placeholders and pass a `Map` with string keys.
The keys in the map correspond to the parameter names without the prefix.

```
db.exec('INSERT INTO t VALUES (:id, :name)', {id: 1, name: 'alice'})
db.query('SELECT * FROM t WHERE id > @min', {min: 0})
```

All methods that accept bindings (`exec`, `query`, `each`, `collect`) support both forms.

There are no explicit prepared statements.
For repeated operations, wrap a parameterized call in a function:

```
defn insert_user(db, id, name) -> db.exec('INSERT INTO users VALUES (?, ?)', [id, name])

insert_user(db, 1, 'alice')
insert_user(db, 2, 'bob')
```

## `version`

A `String` containing the SQLite library version (e.g. `"3.51.3"`).

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

```
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

```
db.script('CREATE TABLE t (id INTEGER); INSERT INTO t VALUES (1); INSERT INTO t VALUES (2)')  # => 2
```

## `db.query`
`db.query(sql)`
`db.query(sql, bindings)`

Executes a single SQL statement and returns all result rows as an `Array` of `Map`s.
Each map has column names as keys.

```
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

```
db.each('SELECT * FROM src', fn row -> db.exec('INSERT INTO dst VALUES (?)', [row.x]))
```

## `db.collect`
`db.collect(sql, callback)`
`db.collect(sql, bindings, callback)`

Executes a query, calls `callback` with each row, and collects the return values into an `Array`.
Rows are processed one at a time, but the collected results are accumulated in memory.

```
db.collect('SELECT x FROM t', fn row -> row.x * 2)  # => [2, 4, 6]
```

## `db.last_insert_rowid`
`db.last_insert_rowid()`

Returns the rowid of the most recent successful `INSERT` on this connection as an `Int`.
If no `INSERT` has been performed, returns `0`.

```
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

```
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

```
def result = try_call(db.transaction, [fn tx -> {
    tx.exec('INSERT INTO t VALUES (?, ?)', [1, 'alice'])
    tx.exec('INSERT INTO t VALUES (?, ?)', [1, 'duplicate'])  # constraint violation
}])
# result.ok == false; both inserts are rolled back
```

## `db.close`
`db.close()`

Closes the database connection.
Produces an error if a transaction is active.
After closing, all operations on `db` produce an error.
