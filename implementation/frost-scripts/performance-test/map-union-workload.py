#!/usr/bin/env python3

from functools import reduce

n = 2500
rows = list(map(lambda i: {i: i, "shared": i}, range(n)))

merged = reduce(lambda acc, m: acc | m, rows, {})
result = len(merged.keys())

assert result == 2501
print(result)
