#!/usr/bin/env python3

from functools import reduce

n = 6000

text = reduce(lambda acc, _: acc + "a", range(n), "")
result = len(text)

assert result == 6000
print(result)
