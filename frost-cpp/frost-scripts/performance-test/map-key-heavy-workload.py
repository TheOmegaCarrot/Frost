#!/usr/bin/env python3

n = 400_000
rows = [{"k": i, "v": i * i} for i in range(n)]
result = sum(row["k"] + row["v"] for row in rows)
assert result == 21_333_333_333_200_000
print(result)
