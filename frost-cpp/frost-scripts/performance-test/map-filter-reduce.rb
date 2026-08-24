#!/usr/bin/env ruby

n = 1_800_000
total = (0...n).sum { |x| sq = x * x; sq < 1_000_000_000 ? sq : 0 }
raise unless total == 10_540_648_931_995
puts total
