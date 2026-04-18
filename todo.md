# To-Do

## Refactors

- Replace thread local Backtrace state with threading context through call boundaries
    - Pretty big refactor, but will pay off in other ways (thread other context through too)

## Perf

- Probe: immutable maps for symbol tables
- Probe: immutable arrays

## Stdlib

### Async

- User-created thread pool objects with `spawn` method, returns a future (like HTTP now)
- Async facilities like HTTP could just be passed in a thread pool to spawn a task on

### Collections

- **`unique` / `deduplicate`** — remove duplicates (achievable with `group_by` but awkward)

### Datetime

- formatted datetime, arithmetic, constants, backed by `std::chrono`

### Strings

- Still missing a lot of string operations (substr, find, etc)

## Extensions

### HTTP enhancements

- redirects, cancellation
- running as a server (blocked by async)
- websockets (maybe separate extension? maybe together if server supports them)

### MessagePack

- JSON-like binary format, should be straightforward

### TOML

- Datetime handling ambiguity (no clear type mapping)

### Postgres

- Mimic SQLite API as close as possible
- Add pub/sub (blocked by async)

### Hash

- xxhash
- File hashing (without having to load full file contents)
