#!/usr/bin/env python3

n = 2000
rows = [{"k": i, "v": i * i} for i in range(n)]
result = sum(row["k"] + row["v"] for row in rows)
assert result == 2_666_666_000
print(result)
