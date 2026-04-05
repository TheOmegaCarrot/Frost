# Functions

## `call`
`call(function)`
`call(function, args)`

Calls `function` with no arguments, or with the elements of the array `args` spread as individual arguments.

## `try_call`
`try_call(function)`
`try_call(function, args)`

Calls `function` with no arguments, or with the elements of `args` spread as arguments.
Returns `{ ok: true, value: result }` on success, or `{ ok: false, error: message }` if the call produces an error.

## `error`
`error(message)`

Produces a normal error with the given message.
The error can be caught by `try_call`.

## `fatal`
`fatal(message)`

Produces a fatal error with the given message.
The error cannot be caught by `try_call` and will terminate the program gracefully.

## `and_then`
`and_then(value, f)`

If `value` is `null`, returns `null`.
Otherwise calls `f(value)` and returns the result.
Useful for chaining operations that may produce `null`.

## `or_else`
`or_else(value, f)`

If `value` is not `null`, returns `value`.
If `value` is `null`, calls `f()` with no arguments and returns the result.
Useful for providing fallbacks.

## `inv`
`inv(f)`

Returns a new function that applies `f` and negates the result with `not`.
Returns `fn ...args -> not f @ call(args)`.

## `curry`
`curry(f, ...outer)`

Returns a function that, when called with additional arguments, calls `f` with the `outer` arguments prepended.
Returns `fn ...inner -> f @ call(outer + inner)`.

## `bcurry`
`bcurry(f, ...outer)`

Like `curry`, but appends the `outer` arguments at the end instead of prepending them.
Returns `fn ...inner -> f @ call(inner + outer)`.

## `collect`
`collect(...e)`

Returns all arguments as an array.

## `spread`
`spread(f)`

Returns a function that takes a single array and calls `f` with its elements spread as individual arguments.
Returns `fn arr -> call(f, arr)`.

```frost
def names = ['Alice', 'Bob']
def ages = [30, 25]
foreach zip(names, ages) with spread(fn (name, age) -> print(name + ' is ' + to_string(age)))
```

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

## `compose`
`compose(f, g, ...rest)`

Returns a function that applies `f` first, then `g`, then each function in `rest` in order (left-to-right).
All arguments must be functions.
