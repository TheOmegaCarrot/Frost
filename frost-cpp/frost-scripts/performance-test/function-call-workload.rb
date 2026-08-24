#!/usr/bin/env ruby

n = 2_600_000

def inc(x)
  x + 1
end

result = (0...n).sum { |i| inc(i) }
raise unless result == 3_380_001_300_000
puts result
