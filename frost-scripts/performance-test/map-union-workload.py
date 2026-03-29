#!/usr/bin/env python3

n = 14_000
rows = [{i: i, "shared": i} for i in range(n)]
merged = {}
for row in rows:
    merged |= row
result = len(merged)

assert result == 14_001
print(result)
