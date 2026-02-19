#!/usr/bin/env python3

from functools import reduce

n = 2000
rows = list(map(lambda i: {"k": i, "v": i * i}, range(n)))

result = reduce(lambda acc, m: acc + m["k"] + m["v"], rows, 0)
assert result == 2_666_666_000
print(result)
