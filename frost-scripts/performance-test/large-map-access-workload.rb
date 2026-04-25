#!/usr/bin/env ruby

n_keys = 5_000
n_lookups = 500_000

m = (0...n_keys).to_h { |i| [i.to_s, i * i] }

result = (0...n_lookups).sum { |j| m[(j % n_keys).to_s] }
raise unless result == 4_165_416_750_000
puts result
