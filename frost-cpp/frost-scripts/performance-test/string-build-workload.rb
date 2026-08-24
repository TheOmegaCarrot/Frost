#!/usr/bin/env ruby

n = 2_000_000
text = Array.new(n, "a").join
result = text.length
raise unless result == 2_000_000
puts result
