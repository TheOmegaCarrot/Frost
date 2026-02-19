#!/usr/bin/env python3

n = 50_000
values = range(n)

squares = map(lambda x: x * x, values)
filtered = filter(lambda x: x < 1_000_000_000, squares)
total = sum(filtered)

print(total)
