#!/usr/bin/env python3

n = 1_400_000


def combine(a: int, b: int, c: int, d: int, e: int) -> int:
    return a + b + c + d + e


result = sum(combine(i, 1, 2, 3, 4) for i in range(n))
assert result == 980_013_300_000
print(result)
