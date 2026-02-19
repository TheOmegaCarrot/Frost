#!/usr/bin/env python3

n = 2000
chunks = [[i, i + 1] for i in range(n)]
merged = [value for chunk in chunks for value in chunk]
result = sum(merged)
assert result == 4_000_000
print(result)
