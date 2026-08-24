#!/usr/bin/env lua

-- Integration test runner for Frost.
--
-- NOTE: This script is only correct in Lua 5.2+
--
-- Usage:
--   runner.lua <frost> <script> [--expect <stderr_file>] [-- <frost_args...>]
--
-- Without --expect: run script, expect exit 0.
-- With --expect: run script, expect non-zero exit. Capture stderr and compare
-- line-by-line against the expected file. Each expected line must match the
-- start of the corresponding actual stderr line. Blank lines and # comment
-- lines in the expected file are skipped.

local unpack = table.unpack or unpack

-- Parse arguments
local frost = arg[1]
local script = arg[2]
local expect_file = nil
local frost_args = {}

local i = 3
while i <= #arg do
    if arg[i] == "--expect" then
        expect_file = arg[i + 1]
        i = i + 2
    elseif arg[i] == "--" then
        i = i + 1
        while i <= #arg do
            frost_args[#frost_args + 1] = arg[i]
            i = i + 1
        end
    else
        i = i + 1
    end
end

-- Build the command
local parts = { frost }
for _, a in ipairs(frost_args) do
    parts[#parts + 1] = a
end
parts[#parts + 1] = script
local cmd = table.concat(parts, " ")

if not expect_file then
    -- Simple mode: expect success
    local ok = os.execute(cmd)
    if not ok then os.exit(1) end
    os.exit(0)
end

-- Expect-stderr mode: capture stderr via temp file
local tmpfile = os.tmpname()
local full_cmd = cmd .. " 2>" .. tmpfile .. " >/dev/null"
local ok = os.execute(full_cmd)

if ok then
    io.stderr:write("FAIL: expected non-zero exit, but got 0\n")
    os.remove(tmpfile)
    os.exit(1)
end

-- Read actual stderr lines (skip blank)
local actual_lines = {}
for line in io.lines(tmpfile) do
    if line:match("%S") then
        actual_lines[#actual_lines + 1] = line
    end
end
os.remove(tmpfile)

-- Read expected lines (skip blank and # comments)
local expected_lines = {}
for line in io.lines(expect_file) do
    if line:match("%S") and not line:match("^#") then
        expected_lines[#expected_lines + 1] = line
    end
end

-- Compare: each expected line must match the start of the corresponding actual line
if #expected_lines ~= #actual_lines then
    io.stderr:write(("FAIL: expected %d lines, got %d\n"):format(
        #expected_lines, #actual_lines))
    io.stderr:write("Expected:\n")
    for _, l in ipairs(expected_lines) do
        io.stderr:write("  " .. l .. "\n")
    end
    io.stderr:write("Actual:\n")
    for _, l in ipairs(actual_lines) do
        io.stderr:write("  " .. l .. "\n")
    end
    os.exit(1)
end

for j = 1, #expected_lines do
    local exp = expected_lines[j]
    local act = actual_lines[j]
    if act:sub(1, #exp) ~= exp then
        io.stderr:write(("FAIL: line %d mismatch\n"):format(j))
        io.stderr:write("  expected: " .. exp .. "\n")
        io.stderr:write("  actual:   " .. act .. "\n")
        os.exit(1)
    end
end

os.exit(0)
