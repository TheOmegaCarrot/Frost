#!/usr/bin/env python3

n = 200_000


def compose(*fns):
    def composed(x):
        for f in fns:
            x = f(x)
        return x
    return composed


def double(x):
    return x * 2


def add_one(x):
    return x + 1


double_then_add = compose(double, add_one)
process = compose(double_then_add, double_then_add)

result = sum(v for v in (process(i) for i in range(n)) if v % 2 != 0)
assert result == 80_000_200_000
print(result)
