#!/usr/bin/env python3

n = 100_000
adders = [lambda x, captured=i: x + captured for i in range(n)]
applied = [f(1) for f in adders]
result = sum(applied)
assert result == 5_000_050_000
print(result)
