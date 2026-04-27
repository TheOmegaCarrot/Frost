# To-Do

## Language

### Comparison Chaining

- P3239-like comparison chaining
    - https://www.open-std.org/jtc1/sc22/wg21/docs/papers/2025/p3439r3.pdf
    - Maybe? Might not be good. I'll let this one sit a while.

## Stdlib

### Async

- User-created thread pool objects with `spawn` method, returns a future (like HTTP now)
- Async facilities like HTTP could just be passed in a thread pool to spawn a task on

### Collections

- **`unique` / `deduplicate`** — remove duplicates (achievable with `group_by` but awkward)

## Extensions

### HTTP enhancements

- redirects, cancellation
- running as a server
    - blocked by async
- websockets
    - blocked by async
    - maybe separate extension
        - maybe together if server supports them

### Postgres

- Mimic SQLite API as close as possible
- Add pub/sub (blocked by async)
