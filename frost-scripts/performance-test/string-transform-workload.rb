#!/usr/bin/env ruby

n = 250_000
words = Array.new(n, "AbCdEfGhIj")
lowered = words.map(&:downcase)
uppered = lowered.map(&:upcase)

raise unless uppered[0] == "ABCDEFGHIJ"

result = uppered.count("ABCDEFGHIJ")
raise unless result == n
puts result
