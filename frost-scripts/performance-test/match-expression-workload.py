#!/usr/bin/env python3

n = 300_000

values = []
for i in range(n):
    mod = i % 4
    if mod == 0:
        values.append(i)
    elif mod == 1:
        values.append(float(i))
    elif mod == 2:
        values.append(str(i))
    else:
        values.append(i % 2 == 0)


def classify(v):
    if isinstance(v, bool):
        return 1
    elif isinstance(v, int):
        return v
    elif isinstance(v, float):
        return 2
    elif isinstance(v, str):
        return len(v)
    return 0


result = sum(classify(v) for v in values)
assert result == 11_250_497_223
print(result)
