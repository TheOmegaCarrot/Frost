#!/usr/bin/env python3

n = 2_000_000
text = "".join("a" for _ in range(n))
result = len(text)
assert result == 2_000_000
print(result)
