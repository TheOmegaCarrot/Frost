#!/usr/bin/env python3

n = 1_000_000

pi_quarter = 0.0
for i in range(n):
    sign = 1.0 if i % 2 == 0 else -1.0
    pi_quarter += sign / (2.0 * i + 1.0)
result = pi_quarter * 4.0

assert 3.14159 < result < 3.14160
print(result)
