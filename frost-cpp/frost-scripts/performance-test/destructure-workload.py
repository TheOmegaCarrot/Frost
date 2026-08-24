#!/usr/bin/env python3

n = 200_000
pairs = [[i, i * i] for i in range(n)]

result = 0
for a, b in pairs:
    result += a + b
assert result == 2_666_666_666_600_000
print(result)
