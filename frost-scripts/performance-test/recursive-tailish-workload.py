#!/usr/bin/env python3

import sys

sys.setrecursionlimit(10_000)

n = 2500


def sum_down(k: int, acc: int) -> int:
    if k == 0:
        return acc
    return sum_down(k - 1, acc + k)


result = sum_down(n, 0)
assert result == 3_126_250
print(result)
