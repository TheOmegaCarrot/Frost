#!/usr/bin/env python3

from functools import reduce

values = range(30_000)

stage1 = map(lambda x: x * 3 + 1, values)
stage2 = filter(lambda x: x % 5 == 0, stage1)
stage3 = map(lambda x: x + 7, stage2)
result = reduce(lambda acc, x: acc + x, stage3, 0)

assert result == 270_057_000
print(result)
