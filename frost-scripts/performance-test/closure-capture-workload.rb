#!/usr/bin/env ruby

n = 300_000
adders = (0...n).map { |i| ->(x) { x + i } }
applied = adders.map { |f| f.(1) }
result = applied.sum
raise unless result == 45_000_150_000
puts result
