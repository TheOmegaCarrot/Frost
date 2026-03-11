# To-Do

## Stdlib gaps

### Collections

- **`find(arr, predicate)`** — first matching element, vs `select` which finds all; very common pattern
- **`unique` / `deduplicate`** — remove duplicates (achievable with `group_by` but awkward)
- **`sum` / `product`** — trivially `fold`, but so universally needed they usually get shortcuts
- **`zip_with(f, a, b)`** — zip with a combining function rather than producing pairs
