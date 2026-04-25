#!/usr/bin/env ruby

n = 1_400_000

combine = ->(a, b, c, d, e) { a + b + c + d + e }

result = (0...n).sum { |i| combine.(i, 1, 2, 3, 4) }
raise unless result == 980_013_300_000
puts result
