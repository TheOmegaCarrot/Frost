#!/usr/bin/env python3

n = 200_000
strings = [f"item_{i}_val_{i * i}" for i in range(n)]
result = sum(len(s) for s in strings)
assert result == 5_142_641
print(result)
