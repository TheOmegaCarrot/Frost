#!/usr/bin/env ruby

n = 400_000
rows = (0...n).map { |i| {"k" => i, "v" => i * i} }
result = rows.sum { |row| row["k"] + row["v"] }
raise unless result == 21_333_333_333_200_000
puts result
