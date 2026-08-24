#!/usr/bin/env ruby

depth = 2500
iters = 250

def sum_down(k, acc)
  return acc if k == 0
  sum_down(k - 1, acc + k)
end

result = (0...iters).sum { sum_down(depth, 0) }
raise unless result == 781_562_500
puts result
