# To-Do

## Lamguage

- Recursive destructuring
    - `def [ {foo}, {bar: {baz: [wow, _, cool, ..._]}}, ...rest ] = [ { foo: 42 }, { baz: [ 1, 2, 3, 4 ] } ]`
    - `foo == 42 and wow == 1 and cool == 3 and rest == []`
- Pattern matching 
    - Still working on the design

## Stdlib

### Async

- User-created thread pool objects with `spawn` method, returns a future (like HTTP now)
- Async facilities like HTTP could just be passed in a thread pool to spawn a task on

### Collections

- **`unique` / `deduplicate`** — remove duplicates (achievable with `group_by` but awkward)
- **`zip_with(f, ...arrays)`** — zip with a combining function rather than producing pairs
- **`xprod_with(f, ...arrays)`** — Cartesian product equivalent to zip_with
