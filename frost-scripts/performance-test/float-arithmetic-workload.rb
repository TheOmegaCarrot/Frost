#!/usr/bin/env ruby

n = 1_000_000

pi_quarter = 0.0
(0...n).each do |i|
  sign = i.even? ? 1.0 : -1.0
  pi_quarter += sign / (2.0 * i + 1.0)
end
result = pi_quarter * 4.0

raise unless result > 3.14159 && result < 3.14160
puts result
