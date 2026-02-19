#!/usr/bin/env python3

from functools import reduce

n = 80_000


def combine(a: int, b: int, c: int, d: int, e: int) -> int:
    return a + b + c + d + e


result = reduce(lambda acc, i: acc + combine(i, 1, 2, 3, 4), range(n), 0)
assert result == 3_200_760_000
print(result)
