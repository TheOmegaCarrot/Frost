#!/usr/bin/env ruby

n = 200_000

def compose(*fns)
  ->(x) { fns.reduce(x) { |acc, f| f.(acc) } }
end

double = ->(x) { x * 2 }
add_one = ->(x) { x + 1 }

double_then_add = compose(double, add_one)
process = compose(double_then_add, double_then_add)

result = (0...n).sum { |i| v = process.(i); v.odd? ? v : 0 }
raise unless result == 80_000_200_000
puts result
