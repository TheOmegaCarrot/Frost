# To-Do

## Stdlib gaps

### Strings

- **`trim` / `strip`** — remove leading/trailing whitespace; would surprise most users to be missing
- **`replace(s, from, to)`** — plain string replacement (only regex replace exists)
- **`lines(s)`** — split by newline; so common in text processing it's almost as obvious as `join`
- **`repeat(s, n)`** — repeat a string n times

### Collections

- **`flatten`** — one level of nesting collapsed; natural companion to `zip`, `xprod`, and `chunk`
- **`find(arr, predicate)`** — first matching element, vs `select` which finds all; very common pattern
- **`unique` / `deduplicate`** — remove duplicates (achievable with `group_by` but awkward)
- **`sum` / `product`** — trivially `fold`, but so universally needed they usually get shortcuts
- **`flat_map`** — map then flatten; common FP operation
- **`zip_with(f, a, b)`** — zip with a combining function rather than producing pairs

### Math

- **`clamp(n, lo, hi)`** — extremely common, no clean way to express it without `min`/`max` nesting
- **`sign` / `signum`** — sign of a number
- **Mathematical constants (`pi`, `e`)** — unless these are already defined somewhere
