#!/usr/bin/env ruby

n = 200_000
strings = (0...n).map { |i| "item_#{i}_val_#{i * i}" }
result = strings.sum(&:length)
raise unless result == 5_142_641
puts result
