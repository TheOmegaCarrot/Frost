#!/usr/bin/env python3

n = 500_000
text = "".join("a" for _ in range(n))
result = len(text)
assert result == 500_000
print(result)
