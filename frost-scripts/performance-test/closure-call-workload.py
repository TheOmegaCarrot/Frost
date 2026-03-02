#!/usr/bin/env python3

n = 120_000


def inc(x: int) -> int:
    return x + 1


result = sum(inc(i) for i in range(n))
assert result == 7_200_060_000
print(result)
