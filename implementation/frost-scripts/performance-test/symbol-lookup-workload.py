#!/usr/bin/env python3

from functools import reduce

n = 200_000
x = 7
y = 11
z = 13

result = reduce(lambda acc, _: acc + x + y + z, range(n), 0)
assert result == 6_200_000
print(result)
