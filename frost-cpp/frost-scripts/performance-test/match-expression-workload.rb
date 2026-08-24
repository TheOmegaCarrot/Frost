#!/usr/bin/env ruby

n = 300_000

values = (0...n).map do |i|
  case i % 4
  when 0 then i
  when 1 then i.to_f
  when 2 then i.to_s
  else i.even?
  end
end

def classify(v)
  case v
  when true, false then 1
  when Integer then v
  when Float then 2
  when String then v.length
  else 0
  end
end

result = values.sum { |v| classify(v) }
raise unless result == 11_250_497_223
puts result
