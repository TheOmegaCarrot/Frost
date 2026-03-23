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

Note that results may sometimes be surprising.
Because Frost values are immutable, the implementation can often re-use the same exact object.
The cases where this optimization can be applied are largely undocumented, and may change between versions without being considered a breaking change.

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

This makes it possible to create reference cycles, which will leak memory with no warning and no guardrails.
Regular Frost code guarantees that reference cycles are impossible, and thus the implementation has no cycle detection.
An unsafe mutable cell allows the user to violate this guarantee.

The interface is identical to `mutable_cell`:

### `.get`
`.get()`

Returns the current value of the cell.

### `.exchange`
`.exchange(value)`

Sets the cell to `value` and returns the previous value.

## `unsafe.weaken`
`unsafe.weaken(value)`

Creates a weak reference to `value`.
Returns a map with a single method:

### `.get`
`.get()`

Returns the original value if it is still alive, or `null` if it has been freed.
A value is freed when all strong references to it are dropped.

```
defn make_weak() -> do {
    def val = {data: [1, 2, 3]}
    def weak = unsafe.weaken(val)
    assert(weak.get() != null)   # val is in scope
    weak
}

def weak = make_weak()
assert(weak.get() == null)       # val went out of scope
```

Note that a weak reference to `null` is indistinguishable from an expired weak reference, since both return `null` from `.get()`.

Because Frost values are immutable, the implementation is free to share identical values behind the scenes.
A weak reference may appear to stay alive longer than expected if the same value happens to be re-used elsewhere.
This sharing behavior is not stable and may change between versions.
