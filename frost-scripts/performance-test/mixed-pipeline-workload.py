#!/usr/bin/env python3

result = sum(x + 7 for x in (y * 3 + 1 for y in range(30_000)) if x % 5 == 0)

assert result == 270_057_000
print(result)
