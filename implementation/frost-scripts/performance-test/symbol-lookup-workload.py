#!/usr/bin/env python3

n = 200_000
x = 7
y = 11
z = 13

result = sum(x + y + z for _ in range(n))
assert result == 6_200_000
print(result)
