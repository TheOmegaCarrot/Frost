#!/usr/bin/env python3

n = 400_000
chunks = [[i, i + 1] for i in range(n)]
merged = [value for chunk in chunks for value in chunk]
result = sum(merged)
assert result == 160_000_000_000
print(result)
