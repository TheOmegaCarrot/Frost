#!/usr/bin/env python3

n = 2_600_000


def inc(x: int) -> int:
    return x + 1


result = sum(inc(i) for i in range(n))
assert result == 3_380_001_300_000
print(result)
