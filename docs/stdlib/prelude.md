# Prelude

The prelude is a set of functions written in Frost itself, loaded at startup.

## `inv`
`inv(f)`

Returns a new function that applies `f` and negates the result with `not`.
Equivalent to `fn ...args -> not f @ pack_call(args)`.

## `reject`
`reject(structure, pred)`

Returns the elements of `structure` for which `pred` returns falsy.
The inverse of `select`.

## `curry`
`curry(f, ...outer)`

Returns a function that, when called with additional arguments, calls `f` with the `outer` arguments prepended.
Equivalent to `fn ...inner -> f @ pack_call(outer + inner)`.

## `bcurry`
`bcurry(f, ...outer)`

Like `curry`, but appends the `outer` arguments at the end instead of prepending them.
Equivalent to `fn ...inner -> f @ pack_call(inner + outer)`.

## `collect`
`collect(...e)`

Returns all arguments as an array.

## `call`
`call(f, ...args)`

Calls `f` with `args`. Variadic equivalent of `pack_call`.

## `rev_args`
`rev_args(f)`

Returns a function that calls `f` with its arguments in reverse order.

## `tap`
`tap(a, f)`

Calls `f(a)` for its side effects, then returns `a` unchanged.
Useful in `@` pipelines for logging or debugging intermediate values without breaking the chain.

## `const`
`const(v)`

Returns a function that ignores all its arguments and always returns `v`.

## `index`
`index(k)`

Returns a function that indexes into its argument with key `k`.
Equivalent to `fn s -> s[k]`.

## `map_keys`
`map_keys(m, f)`

Returns a new map with every key transformed by `f`.
Values are unchanged.

## `map_values`
`map_values(m, f)`

Returns a new map with every value transformed by `f`.
Keys are unchanged.

## `compose`
`compose(f, g, ...rest)`

Returns a function that applies `f` first, then `g`, then each function in `rest` in order (left-to-right).
All arguments must be functions.
