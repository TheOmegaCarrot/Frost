# To-Do

## Stdlib

### Async

- User-created thread pool objects with `spawn` method, returns a future (like HTTP now)
- Async facilities like HTTP could just be passed in a thread pool to spawn a task on

### Archives (lower priority)

- **libarchive** as an `ext` module — tar, cpio, 7z, etc. with compression support built in
- Pairs with gzip/zlib for `.tar.gz` workflows

### Collections

- **`unique` / `deduplicate`** — remove duplicates (achievable with `group_by` but awkward)
