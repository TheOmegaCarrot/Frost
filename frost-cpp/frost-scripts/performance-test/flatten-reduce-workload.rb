#!/usr/bin/env ruby

n = 400_000
chunks = (0...n).map { |i| [i, i + 1] }
merged = chunks.flatten(1)
result = merged.sum
raise unless result == 160_000_000_000
puts result
