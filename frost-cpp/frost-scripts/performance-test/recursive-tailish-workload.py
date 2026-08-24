#!/usr/bin/env python3

import sys

sys.setrecursionlimit(5000)

depth = 2500
iters = 250


def sum_down(k: int, acc: int) -> int:
    if k == 0:
        return acc
    return sum_down(k - 1, acc + k)


result = sum(sum_down(depth, 0) for _ in range(iters))
assert result == 781_562_500
print(result)
