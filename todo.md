# To-Do

## Refactors

- Replace thread local Backtrace state with threading context through call boundaries
    - Pretty big refactor, but will pay off in other ways (thread other context through too)

## Perf

- Probe: immutable maps for symbol tables (immer)
- Probe: immutable arrays (immer)

## Language

### Map destructuring: bind whole + fields

Pattern: destructure specific fields from a map while also binding the whole map.
Motivating use case: module imports where you want a few common items pulled out but also keep the full module reference.

```frost
# Strawman syntax using `as`:
def {ms} as dt = import('std.datetime')
dt.now() + 3 * ms.hour
```

Design notes:
- `...rest` in arrays means "the remainder" -- using it in maps for "the whole thing" would be an inconsistency
- `as` keyword feels like lowest friction, avoids overloading `...` semantics
- Could apply to any destructuring/matching context, not just imports
- Syntax still needs refining

## Stdlib

### Async

- User-created thread pool objects with `spawn` method, returns a future (like HTTP now)
- Async facilities like HTTP could just be passed in a thread pool to spawn a task on

### Collections

- **`unique` / `deduplicate`** — remove duplicates (achievable with `group_by` but awkward)

### Datetime

- formatted datetime, arithmetic, constants, backed by `std::chrono`

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
