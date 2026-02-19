#!/usr/bin/env python3

n = 4000
words = list(map(lambda _: "AbCdEfGhIj", range(n)))
lowered = list(map(str.lower, words))
uppered = list(map(str.upper, lowered))

assert uppered[0] == "ABCDEFGHIJ"

result = sum(map(lambda s: 1 if s == "ABCDEFGHIJ" else 0, uppered))
assert result == n
print(result)
