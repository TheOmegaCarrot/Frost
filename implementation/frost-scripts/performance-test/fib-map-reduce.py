#!/usr/bin/env python3


def fib(n: int) -> int:
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)


values = list(map(fib, range(30)))
total = sum(values)
print(total)
