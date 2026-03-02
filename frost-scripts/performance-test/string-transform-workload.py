#!/usr/bin/env python3

n = 4000
words = ["AbCdEfGhIj"] * n
lowered = [word.lower() for word in words]
uppered = [word.upper() for word in lowered]

assert uppered[0] == "ABCDEFGHIJ"

result = sum(1 for word in uppered if word == "ABCDEFGHIJ")
assert result == n
print(result)
