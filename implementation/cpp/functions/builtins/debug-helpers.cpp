#include "builtins-common.hpp"

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst
{

#pragma message("TODO: test debug_dump")
Value_Ptr debug_dump(builtin_args_t args)
{
    if (const auto& arg = args.at(0); arg->is<Function>())
    {
        return Value::create(arg->raw_get<Function>()->debug_dump());
    }
    else
    {
        return arg->to_string();
    }
}

void inject_debug_helpers(Symbol_Table& table)
{
    INJECT(debug_dump, 1, 1);
}

} // namespace frst
