#!/usr/bin/env python3

n = 14_000
merged = {}
for i in range(n):
    merged[i] = i
    merged["shared"] = i
result = len(merged)

assert result == 14_001
print(result)
