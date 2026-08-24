#!/usr/bin/env python3

n = 3_000_000
x = 7
y = 11
z = 13

result = sum(x + y + z for _ in range(n))
assert result == 93_000_000
print(result)
