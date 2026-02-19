#!/usr/bin/env python3

from functools import reduce

n = 2000
chunks = list(map(lambda i: [i, i + 1], range(n)))
merged = reduce(lambda acc, chunk: acc + chunk, chunks, [])

result = reduce(lambda acc, x: acc + x, merged, 0)
assert result == 4_000_000
print(result)
