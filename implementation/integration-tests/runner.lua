#!/usr/bin/env lua

-- This little runner script doesn't do much now,
-- but as integration tests grow, I may want to add
-- the ability for the test file to specify some expectations
-- (output, success/fail, etc)
-- and this runner could read a comment in the test script
-- and set its expectations accordingly

-- I expect this to be run with Lua >= 5.3, but this is a reasonable "safety net"
local unpack = table.unpack or unpack

local frost, script = unpack(arg)

-- ok: true or nil (success/fail)
-- exit_kind: 'exit' or 'signal'
-- num: exit code if exit_kind == 'exit', or signal number if exit_kind == 'signal'
local ok, exit_kind, num = os.execute(('%s %s'):format(frost, script))

if not ok then os.exit(1) end
