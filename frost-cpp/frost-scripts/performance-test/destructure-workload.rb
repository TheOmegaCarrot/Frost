#!/usr/bin/env ruby

n = 200_000
pairs = (0...n).map { |i| [i, i * i] }

result = 0
pairs.each { |a, b| result += a + b }
raise unless result == 2_666_666_666_600_000
puts result
