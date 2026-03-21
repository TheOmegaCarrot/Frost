# Unsafe

The `unsafe` module provides functions that bypass Frost's usual safety guarantees.
These are opt-in and intended for advanced use cases such as performance-critical code, debugging, and interop patterns that require unrestricted mutability or pointer-level introspection.

The `unsafe` module is optional and can be disabled at build time with `-DWITH_UNSAFE=NO`.

The precise behavior of these functions may change between versions.
Internal implementation details such as value sharing and allocation are not part of Frost's stability guarantees, so results from pointer-level operations like `same` and `identity` may differ across releases.

## `unsafe.same`
`unsafe.same(a, b)`

Returns `true` if `a` and `b` are the exact same object in memory (pointer equality), `false` otherwise.
This is distinct from `==`, which compares by value.

```
def a = [1, 2, 3]
def b = [1, 2, 3]
def c = a

a == b            # => true  (equal by value)
unsafe.same(a, b) # => false (distinct objects)
unsafe.same(a, c) # => true  (same object)
```

## `unsafe.identity`
`unsafe.identity(value)`

Returns the memory address of `value` as an `Int`.
Two calls return the same `Int` if and only if `unsafe.same` would return `true` for the two values.

## `unsafe.mutable_cell`
`unsafe.mutable_cell()`
`unsafe.mutable_cell(initial)`

Creates a mutable cell with no restrictions on what it can hold.
Unlike the standard `mutable_cell`, this cell accepts functions, other mutable cells, and structures containing them.

This makes it possible to create reference cycles, which will leak memory.
Regular Frost code guarantees that reference cycles are impossible, and thus the implementation has no cycle detection.
An unsafe mutable cell allows the user to violate this guarantee.

The interface is identical to `mutable_cell`:

### `.get`
`.get()`

Returns the current value of the cell.

### `.exchange`
`.exchange(value)`

Sets the cell to `value` and returns the previous value.
