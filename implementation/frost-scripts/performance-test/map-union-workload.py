#!/usr/bin/env python3

n = 2500
rows = [{i: i, "shared": i} for i in range(n)]
merged = {}
for row in rows:
    merged |= row
result = len(merged)

assert result == 2501
print(result)
