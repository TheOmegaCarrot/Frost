#!/usr/bin/env python3

from functools import reduce

n = 120_000


def inc(x: int) -> int:
    return x + 1


result = reduce(lambda acc, i: acc + inc(i), range(n), 0)
assert result == 7_200_060_000
print(result)
