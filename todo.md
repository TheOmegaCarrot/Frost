# To-Do

## Bugs

- Retry import after failed import falsly reports import cycle

## Stdlib

### Async

- User-created thread pool objects with `spawn` method, returns a future (like HTTP now)
- Async facilities like HTTP could just be passed in a thread pool to spawn a task on

### Collections

- **`unique` / `deduplicate`** — remove duplicates (achievable with `group_by` but awkward)

### Datetime

- formatted datetime, arithmetic, constants, backed by `std::chrono`

## Extensions

### MessagePack

- JSON-like binary format, should be straightforward

### Postgres

- Mimic SQLite API as close as possible
