#!/usr/bin/env python3

n = 100_000
chunks = [[i, i + 1] for i in range(n)]
merged = [value for chunk in chunks for value in chunk]
result = sum(merged)
assert result == 10_000_000_000
print(result)
