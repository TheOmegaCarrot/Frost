#!/usr/bin/env python3

n = 50_000

data = [{"a": {"b": {"c": {"d": {"e": i}}}}} for i in range(n)]

result = sum(entry["a"]["b"]["c"]["d"]["e"] for entry in data)
assert result == 1_249_975_000
print(result)
