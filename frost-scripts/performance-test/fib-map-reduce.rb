#!/usr/bin/env ruby

def fib(n)
  return n if n < 2
  fib(n - 1) + fib(n - 2)
end

total = (0...30).sum { |i| fib(i) }
raise unless total == 1_346_268
puts total
