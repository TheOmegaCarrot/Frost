#!/usr/bin/env python3

n = 100_000
rows = [{"k": i, "v": i * i} for i in range(n)]
result = sum(row["k"] + row["v"] for row in rows)
assert result == 333_333_333_300_000
print(result)
