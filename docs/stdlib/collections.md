# Collections

## `keys`
`keys(m)`

Returns an array of all keys in map `m`.
The order is unspecified, but consistent with `values(m)`: index `i` in `keys(m)` corresponds to index `i` in `values(m)`.

## `values`
`values(m)`

Returns an array of all values in map `m`.
The order is unspecified, but consistent with `keys(m)`: index `i` in `values(m)` corresponds to index `i` in `keys(m)`.

## `map_keys`
`map_keys(m, f)`

Returns a new map with every key transformed by `f`.
Values are unchanged.

## `map_values`
`map_values(m, f)`

Returns a new map with every value transformed by `f`.
Keys are unchanged.

## `len`
`len(s)`

Returns the number of elements in an `Array`, the number of entries in a `Map`, or the byte length of a `String`.

## `range`
`range(stop)`
`range(start, stop)`
`range(start, stop, step)`

Returns an `Array` of `Int` values.
`step` defaults to `1` and must not be `0`.

When `step > 0`, yields `start, start + step, ...` while the value is `< stop`.
When `step < 0`, yields `start, start + step, ...` while the value is `> stop`.
Returns an empty array if the range is already exhausted at `start`.

```
range(5)           # => [0, 1, 2, 3, 4]
range(2, 5)        # => [2, 3, 4]
range(0, 10, 2)    # => [0, 2, 4, 6, 8]
range(5, 0, -1)    # => [5, 4, 3, 2, 1]
range(5, 5)        # => []
range(0, 5, -1)    # => []
```

## `nulls`
`nulls(n)`

Returns an array of `n` `null` values. `n` must be `>= 0`.

## `repeat`
`repeat(value, n)`

Returns an array of `n` copies of `value`. `n` must be `>= 0`.
`value` may be of any type.

```
repeat(0, 3)      # => [0, 0, 0]
repeat("x", 4)    # => ["x", "x", "x", "x"]
repeat(null, 0)   # => []
```

## `id`
`id(value)`

Returns `value` unchanged.

## `has`
`has(structure, index)`

For a `Map`: returns `true` if `index` exists as a key.
For an `Array`: returns `true` if `index` (must be an `Int`) is a valid index.

## `index`
`index(k)`

Returns a function that indexes into its argument with key `k`.
Equivalent to `fn s -> s[k]`.
Useful in pipelines: `people @ transform(index("name"))`.

## `dig`
`dig(structure, ...keys)`

Indexes into nested structures, returning `null` if any intermediate value is `null` rather than producing an error.
Each key is applied in order, indexing into the result of the previous step.

```
def m = {a: {b: {c: 42}}}

m @ dig('a', 'b', 'c')   # => 42
m @ dig('a', 'x', 'c')   # => null (missing intermediate key)

def nested = {items: [{name: 'alice'}, {name: 'bob'}]}
nested @ dig('items', 0, 'name')   # => 'alice'
nested @ dig('items', 5, 'name')   # => null (out of bounds)
```

## `stride`
`stride(arr, n)`

Returns every `n`th element of `arr` starting from index 0. `n` must be `> 0`.

```
stride([1, 2, 3, 4, 5, 6], 2)  # => [1, 3, 5]
```

## `take`
`take(arr, n)`

Returns the first `n` elements of `arr`. `n` must be `>= 0`.

## `drop`
`drop(arr, n)`

Returns all but the first `n` elements of `arr`. `n` must be `>= 0`.

## `tail`
`tail(arr, n)`

Returns the last `n` elements of `arr`. `n` must be `>= 0`.

## `drop_tail`
`drop_tail(arr, n)`

Returns all but the last `n` elements of `arr`. `n` must be `>= 0`.

## `slide`
`slide(arr, n)`

Returns an array of overlapping windows of size `n`. `n` must be `> 0`.

```
slide([1, 2, 3, 4], 3)  # => [[1, 2, 3], [2, 3, 4]]
```

## `chunk`
`chunk(arr, n)`

Returns an array of non-overlapping chunks of size `n`.
The last chunk may be smaller than `n` if the array length is not evenly divisible.
`n` must be `> 0`.

```
chunk([1, 2, 3, 4, 5], 2)  # => [[1, 2], [3, 4], [5]]
```

## `reverse`
`reverse(arr)`

Returns `arr` in reverse order.

## `take_while`
`take_while(arr, f)`

Returns elements from the start of `arr` as long as `f` returns truthy, stopping at the first element for which `f` returns falsy.

## `drop_while`
`drop_while(arr, f)`

Drops elements from the start of `arr` as long as `f` returns truthy, then returns the remainder.

## `chunk_by`
`chunk_by(arr, f)`

Groups consecutive elements into chunks.
A new chunk begins whenever `f(previous, current)` returns falsy.

```
chunk_by([1, 1, 2, 2, 1], fn (a, b) -> a == b)  # => [[1, 1], [2, 2], [1]]
```

## `flatten`
`flatten(arr)`
`flatten(arr, n)`

Flattens nested arrays.
Without `n`, recursively flattens all levels of nesting.
With `n`, flattens exactly `n` levels. `n` must be `>= 0`; `n=0` returns `arr` unchanged.
Non-array elements are always left untouched.

```
flatten([1, [2, [3, 4]], 5])      # => [1, 2, 3, 4, 5]
flatten([1, [2, [3, 4]], 5], 1)   # => [1, 2, [3, 4], 5]
flatten([1, [2, [3, 4]], 5], 0)   # => [1, [2, [3, 4]], 5]
```

## `zip`
`zip(...arrays)`

Combines two or more arrays element by element.
Returns an array of arrays, truncated to the length of the shortest input.

```
zip([1, 2, 3], ["a", "b", "c"])  # => [[1, "a"], [2, "b"], [3, "c"]]
```

## `xprod`
`xprod(...arrays)`

Returns the cartesian product of two or more arrays.
Each element of the result is an array containing one element from each input.
Returns an empty array if any input is empty.

```
xprod([1, 2], ["a", "b"])  # => [[1, "a"], [1, "b"], [2, "a"], [2, "b"]]
```

## `transform`
`transform(structure, f)`

Functional form of the [`map` expression](../language.md#map).
Applies `f` to each element of an `Array`, or to each key-value pair `(k, v)` of a `Map`.
For `Map` input, `f` must return a `Map`; its entries are merged into the result, allowing keys to be remapped or entries to be expanded.

## `flat_map`
`flat_map(arr, f)`

Applies `f` to each element of `arr` and flattens the results one level.
Equivalent to `arr @ transform(f) @ flatten(1)`. See [`transform`](#transform) and [`flatten`](#flatten).

```
flat_map([1, 2, 3], fn x -> [x, x * 2])  # => [1, 2, 2, 4, 3, 6]
```

## `select`
`select(structure, f)`

Functional form of the [`filter` expression](../language.md#filter).
Returns elements of an `Array` for which `f` returns truthy, or entries of a `Map` for which `f(k, v)` returns truthy.

## `reject`
`reject(structure, pred)`

Returns the elements of `structure` for which `pred` returns falsy.
The inverse of [`select`](#select).

## `fold`
`fold(structure, f)`
`fold(structure, f, init)`

Functional form of the [`reduce` expression](../language.md#reduce).
Reduces `structure` to a single value by repeatedly applying `f`.
Without `init`, the first element is used as the initial accumulator.
For maps, `f` receives `(accumulator, key, value)`.

## `sorted`
`sorted(arr)`
`sorted(arr, f)`

Returns a sorted copy of `arr`.
Without `f`, uses the default `<` ordering.
With `f`, uses `f(a, b)` as the comparator, where `f` should return truthy if `a` should come before `b`.
Uses a stable sort.

## `any`
`any(arr)`
`any(arr, f)`

Returns `true` if any element of `arr` is truthy, or if `f` returns truthy for any element.

## `all`
`all(arr)`
`all(arr, f)`

Returns `true` if all elements of `arr` are truthy, or if `f` returns truthy for every element.

## `none`
`none(arr)`
`none(arr, f)`

Returns `true` if no element of `arr` is truthy, or if `f` returns falsy for every element.

## `group_by`
`group_by(arr, f)`

Returns a map where each key is a value returned by `f`, and each value is an array of all elements in `arr` that produced that key.

```
group_by([1, 2, 3, 4, 5], fn (x) -> x % 2)
# => { [0]: [2, 4], [1]: [1, 3, 5] }
```

## `count_by`
`count_by(arr, f)`

Returns a map where each key is a value returned by `f`, and each value is an `Int` count of how many elements produced that key.

```
count_by(["a", "b", "a", "c", "a"], id)
# => { ["a"]: 3, ["b"]: 1, ["c"]: 1 }
```

## `scan`
`scan(arr, f)`

Returns a running accumulation of `arr` using `f`.
The first element of the result is the first element of `arr` (used as the initial accumulator).
Each subsequent element is `f(previous_accumulator, current_element)`.
Returns an empty array for empty input.

```
scan([1, 2, 3, 4], fn(acc, x) -> acc + x)  # => [1, 3, 6, 10]
```

## `partition`
`partition(arr, f)`

Splits `arr` into two arrays based on predicate `f`.
Returns `{ pass: [...], fail: [...] }` where `pass` contains elements for which `f` returned truthy and `fail` contains the rest.

```
partition([1, 2, 3, 4, 5], fn (x) -> x % 2 == 1)
# => { pass: [1, 3, 5], fail: [2, 4] }
```
