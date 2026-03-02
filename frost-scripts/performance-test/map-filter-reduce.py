#!/usr/bin/env python3

n = 50_000
total = sum(square for square in (x * x for x in range(n)) if square < 1_000_000_000)

print(total)
