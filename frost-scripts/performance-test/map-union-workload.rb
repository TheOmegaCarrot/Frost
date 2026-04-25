#!/usr/bin/env ruby

n = 14_000
rows = (0...n).map { |i| {i => i, "shared" => i} }
merged = {}
rows.each { |row| merged.merge!(row) }
result = merged.length

raise unless result == 14_001
puts result
