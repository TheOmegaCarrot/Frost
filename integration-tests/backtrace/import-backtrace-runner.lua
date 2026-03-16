#!/usr/bin/env lua

-- Import-boundary backtrace test runner.
--
-- NOTE: This script is only correct in Lua 5.2+
--
-- Usage:
--   import-backtrace-runner.lua <frost> <importer.frst> <module_dir> <module_name>
--       --expect-inner <crash.traced.stderr>
--       [--expect-suffix <importer.traced.suffix>]
--       [-- frost_args...]
--
-- 1. Runs FROST_MODULE_PATH=<module_dir> frost [frost_args] <importer.frst> <module_name>
-- 2. Reads --expect-inner file -> inner expected lines (skip blank/# comments)
-- 3. If --expect-suffix: reads it -> suffix lines
-- 4. Expected = inner lines + suffix lines. Compare with prefix matching.

local frost = arg[1]
local importer = arg[2]
local module_dir = arg[3]
local module_name = arg[4]

local expect_inner_file = nil
local expect_suffix_file = nil
local frost_args = {}

local i = 5
while i <= #arg do
    if arg[i] == "--expect-inner" then
        expect_inner_file = arg[i + 1]
        i = i + 2
    elseif arg[i] == "--expect-suffix" then
        expect_suffix_file = arg[i + 1]
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

if not expect_inner_file then
    io.stderr:write("ERROR: --expect-inner is required\n")
    os.exit(1)
end

-- Build the command
local parts = { "FROST_MODULE_PATH=" .. module_dir, frost }
for _, a in ipairs(frost_args) do
    parts[#parts + 1] = a
end
parts[#parts + 1] = importer
parts[#parts + 1] = module_name

local cmd = table.concat(parts, " ")

-- Capture stderr via temp file
local tmpstderr = os.tmpname()
local full_cmd = cmd .. " 2>" .. tmpstderr .. " >/dev/null"
local ok = os.execute(full_cmd)

if ok then
    io.stderr:write("FAIL: expected non-zero exit, but got 0\n")
    os.remove(tmpstderr)
    os.exit(1)
end

-- Read actual stderr lines (skip blank)
local actual_lines = {}
for line in io.lines(tmpstderr) do
    if line:match("%S") then
        actual_lines[#actual_lines + 1] = line
    end
end
os.remove(tmpstderr)

-- Helper: read expected-format file, skip blank and # comments
local function read_expected(path)
    local lines = {}
    for line in io.lines(path) do
        if line:match("%S") and not line:match("^#") then
            lines[#lines + 1] = line
        end
    end
    return lines
end

-- Build expected lines: inner + suffix
local expected_lines = read_expected(expect_inner_file)

if expect_suffix_file then
    local suffix_lines = read_expected(expect_suffix_file)
    for _, line in ipairs(suffix_lines) do
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
