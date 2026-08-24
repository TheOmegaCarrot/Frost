#!/usr/bin/env ruby

n = 200_000

data = (0...n).map { |i| {"a" => {"b" => {"c" => {"d" => {"e" => i}}}}} }

result = data.sum { |entry| entry["a"]["b"]["c"]["d"]["e"] }
raise unless result == 19_999_900_000
puts result
