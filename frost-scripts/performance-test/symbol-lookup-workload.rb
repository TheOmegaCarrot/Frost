#!/usr/bin/env ruby

n = 3_000_000
x = 7
y = 11
z = 13

result = (0...n).sum { x + y + z }
raise unless result == 93_000_000
puts result
