#!/usr/bin/env python3

from functools import reduce

n = 2000
adders = list(map(lambda i: (lambda x, captured=i: x + captured), range(n)))
applied = list(map(lambda f: f(1), adders))

result = reduce(lambda acc, x: acc + x, applied, 0)
assert result == 2_001_000
print(result)
