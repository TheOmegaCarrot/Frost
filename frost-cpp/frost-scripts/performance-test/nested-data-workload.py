#!/usr/bin/env python3

n = 200_000

data = [{"a": {"b": {"c": {"d": {"e": i}}}}} for i in range(n)]

result = sum(entry["a"]["b"]["c"]["d"]["e"] for entry in data)
assert result == 19_999_900_000
print(result)
