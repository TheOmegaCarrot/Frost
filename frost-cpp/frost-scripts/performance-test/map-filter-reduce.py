#!/usr/bin/env python3

n = 1_800_000
total = sum(square for square in (x * x for x in range(n)) if square < 1_000_000_000)

assert total == 10_540_648_931_995
print(total)
