#!/usr/bin/env python3

n = 2000
adders = [lambda x, captured=i: x + captured for i in range(n)]
applied = [f(1) for f in adders]
result = sum(applied)
assert result == 2_001_000
print(result)
