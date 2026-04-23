#!/usr/bin/env python3

n_keys = 5_000
n_lookups = 500_000

m = {str(i): i * i for i in range(n_keys)}

result = sum(m[str(j % n_keys)] for j in range(n_lookups))
assert result == 4_165_416_750_000
print(result)
