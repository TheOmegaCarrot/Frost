#!/usr/bin/env python3


def fib(n: int) -> int:
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)


total = sum(fib(i) for i in range(30))
print(total)
