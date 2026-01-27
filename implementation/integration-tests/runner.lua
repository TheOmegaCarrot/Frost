#!/usr/bin/env lua

-- I expect this to be run with Lua >= 5.3, but this is a reasonable "safety net"
local unpack = table.unpack or unpack

local frost, script = unpack(arg)

-- ok: true or nil (success/fail)
-- exit_kind: 'exit' or 'signal'
-- num: exit code if exit_kind == 'exit', or signal number if exit_kind == 'signal'
local ok, exit_kind, num = os.execute(('%s %s'):format(frost, script))

if not ok then os.exit(1) end
