#!/usr/bin/env ruby

result = (0...1_500_000).sum { |y| x = y * 3 + 1; x % 5 == 0 ? x + 7 : 0 }
raise unless result == 675_002_850_000
puts result
