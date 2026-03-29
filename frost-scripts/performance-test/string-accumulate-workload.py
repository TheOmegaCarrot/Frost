#!/usr/bin/env python3

n = 70_000

text = ""
for i in range(n):
    text += "a"
result = len(text)

assert result == 70_000
print(result)
