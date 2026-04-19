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

```frost
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

```frost
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

## `includes`
`includes(array, value)`

Returns `true` if `array` contains an element equal to `value`.

## `index`
`index(k)`

Returns a function that indexes into its argument with key `k`.
Equivalent to `fn s -> s[k]`.
Useful in pipelines: `people @ transform(index("name"))`.

## `dig`
`dig(structure, ...keys)`

Indexes into nested structures, returning `null` if any intermediate value is `null` rather than producing an error.
Each key is applied in order, indexing into the result of the previous step.

```frost
def m = {a: {b: {c: 42}}}

m @ dig('a', 'b', 'c')   # => 42
m @ dig('a', 'x', 'c')   # => null (missing intermediate key)

def nested = {items: [{name: 'alice'}, {name: 'bob'}]}
nested @ dig('items', 0, 'name')   # => 'alice'
nested @ dig('items', 5, 'name')   # => null (out of bounds)
```

## `slice`

`slice(arr, start)`
`slice(arr, start, end)`

Returns a sub-array from `start` (inclusive) to `end` (exclusive).
If `end` is omitted, slices to the end of the array.
Negative indices count from the end.
Out-of-bounds indices are clamped.

```frost
slice([1, 2, 3, 4, 5], 1, 3)   # => [2, 3]
slice([1, 2, 3, 4, 5], 2)      # => [3, 4, 5]
slice([1, 2, 3, 4, 5], -2)     # => [4, 5]
slice([1, 2, 3, 4, 5], 1, -1)  # => [2, 3, 4]
```

## `stride`
`stride(arr, n)`

Returns every `n`th element of `arr` starting from index 0. `n` must be `> 0`.

```frost
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

```frost
slide([1, 2, 3, 4], 3)  # => [[1, 2, 3], [2, 3, 4]]
```

## `chunk`
`chunk(arr, n)`

Returns an array of non-overlapping chunks of size `n`.
The last chunk may be smaller than `n` if the array length is not evenly divisible.
`n` must be `> 0`.

```frost
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

```frost
chunk_by([1, 1, 2, 2, 1], fn (a, b) -> a == b)  # => [[1, 1], [2, 2], [1]]
```

## `flatten`
`flatten(arr)`
`flatten(arr, n)`

Flattens nested arrays.
Without `n`, recursively flattens all levels of nesting.
With `n`, flattens exactly `n` levels. `n` must be `>= 0`; `n=0` returns `arr` unchanged.
Non-array elements are always left untouched.

```frost
flatten([1, [2, [3, 4]], 5])      # => [1, 2, 3, 4, 5]
flatten([1, [2, [3, 4]], 5], 1)   # => [1, 2, [3, 4], 5]
flatten([1, [2, [3, 4]], 5], 0)   # => [1, [2, [3, 4]], 5]
```

## `zip`
`zip(...arrays)`

Combines two or more arrays element by element.
Returns an array of arrays, truncated to the length of the shortest input.

```frost
zip([1, 2, 3], ["a", "b", "c"])  # => [[1, "a"], [2, "b"], [3, "c"]]
```

## `zip_with`
`zip_with(f, ...arrays)`

Combines two or more arrays element-wise by applying `f` to corresponding elements.
The function receives one argument per array.
If the input arrays are of unequal length, all are truncated to the length of the shortest input.
(See the second example below.)

Equivalent to `map zip(...arrays) with` [`spread`](functions.md#spread)`(f)`, but without materializing the intermediate array of arrays.

```frost
zip_with(fn a, b -> a + b, [1, 2, 3], [10, 20, 30])
# => [11, 22, 33]

zip_with(fn a, b, c -> a * b + c, [2, 3, 4], [10, 20, 30], [1, 1])
# => [21, 61]
```

## `xprod`
`xprod(...arrays)`

Returns the cartesian product of two or more arrays.
Each element of the result is an array containing one element from each input.
Returns an empty array if any input is empty.

```frost
xprod([1, 2], ["a", "b"])  # => [[1, "a"], [1, "b"], [2, "a"], [2, "b"]]
```

## `xprod_with`
`xprod_with(f, ...arrays)`

Computes the cartesian product of two or more arrays, applying `f` to each element of the product.
The function receives one argument per array.
Returns an empty array if any input is empty.

Equivalent to `map xprod(...arrays) with` [`spread`](functions.md#spread)`(f)`, but without materializing the intermediate array of arrays.

```frost
xprod_with(fn a, b -> a + b, [10, 20], [1, 2])
# => [11, 12, 21, 22]

xprod_with(fn a, b -> a + b, ['a', 'b'], ['1', '2'])
# => ['a1', 'a2', 'b1', 'b2']
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

```frost
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

## `sort_by`
`sort_by(arr, projection)`

Returns a sorted copy of `arr`, using `projection` as a key function.
Each element is sorted by the value of `projection(element)` under the default `<` ordering.
The projection is called exactly once per element.
Uses a stable sort.
Combines well with [`index`](#index): `sort_by(users, index("name"))`.

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

## `find`
`find(arr, predicate)`

Returns the first element of `arr` for which `predicate` returns truthy, or `null` if no element matches.
Short-circuits on the first match.

## `group_by`
`group_by(arr, f)`

Returns a map where each key is a value returned by `f`, and each value is an array of all elements in `arr` that produced that key.

```frost
group_by([1, 2, 3, 4, 5], fn (x) -> x % 2)
# => { [0]: [2, 4], [1]: [1, 3, 5] }
```

## `count_by`
`count_by(arr, f)`

Returns a map where each key is a value returned by `f`, and each value is an `Int` count of how many elements produced that key.

```frost
count_by(["a", "b", "a", "c", "a"], id)
# => { ["a"]: 3, ["b"]: 1, ["c"]: 1 }
```

## `scan`
`scan(arr, f)`

Returns a running accumulation of `arr` using `f`.
The first element of the result is the first element of `arr` (used as the initial accumulator).
Each subsequent element is `f(previous_accumulator, current_element)`.
Returns an empty array for empty input.

```frost
scan([1, 2, 3, 4], fn(acc, x) -> acc + x)  # => [1, 3, 6, 10]
```

## `partition`
`partition(arr, f)`

Splits `arr` into two arrays based on predicate `f`.
Returns `{ pass: [...], fail: [...] }` where `pass` contains elements for which `f` returned truthy and `fail` contains the rest.

```frost
partition([1, 2, 3, 4, 5], fn (x) -> x % 2 == 1)
# => { pass: [1, 3, 5], fail: [2, 4] }
```

## `map_into`
`map_into(arr, f)`

Calls `f` on each element of `arr`. Each call must return a `Map`.
The results are merged left-to-right into a single `Map`.
Later entries overwrite earlier ones on key collision.

```frost
map_into(['a', 'b', 'c'], fn s -> { [s]: len(s) })
# => { a: 1, b: 1, c: 1 }

map_into(range(3), fn n -> { [n]: n * n })
# => { [0]: 0, [1]: 1, [2]: 4 }
```

## `to_entries`
`to_entries(m)`

Converts a map into an array of `{key: k, value: v}` entries.
The order of entries in the result is unspecified.

```frost
to_entries({a: 1, b: 2})
# => [{key: 'a', value: 1}, {key: 'b', value: 2}]  (order may vary)
```

## `from_entries`
`from_entries(arr)`

Converts an array of `{key: k, value: v}` maps into a single map.
Each element must be a map containing at least a `key` and a `value` entry.
Keys must be valid map keys (non-null primitives). Extra entries in the element maps are ignored.
On duplicate keys, the last entry wins.

```frost
from_entries([{key: 'a', value: 1}, {key: 'b', value: 2}])
# => {a: 1, b: 2}

from_entries([{key: 'x', value: 1}, {key: 'x', value: 2}])
# => {x: 2}
```

`to_entries` and `from_entries` are inverses: `from_entries(to_entries(m)) == m` for any map `m`.

## `dissoc`
`dissoc(m, ...keys)`

Returns a new map with the given keys removed.
Each key must be a valid map key (non-null primitive).
Missing keys are silently ignored.

```frost
dissoc({a: 1, b: 2, c: 3}, 'b')        # => {a: 1, c: 3}
dissoc({a: 1, b: 2, c: 3}, 'a', 'c')   # => {b: 2}
dissoc({a: 1}, 'missing')               # => {a: 1}

{a: 1, b: 2, c: 3} @ dissoc('b')       # => {a: 1, c: 3}
```
