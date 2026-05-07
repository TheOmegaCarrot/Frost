#!/usr/bin/env ruby

n = 14_000
merged = {}
(0...n).each { |i| merged[i] = i; merged["shared"] = i }
result = merged.length

raise unless result == 14_001
puts result
