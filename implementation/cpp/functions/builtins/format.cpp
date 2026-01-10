#include "builtins-common.hpp"

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst
{
Value_Ptr pformat(builtin_args_t args)
{
    REQUIRE_ARGS(pformat, TYPES(String));
    return Value::create();
}

void inject_format(Symbol_Table& table)
{
    INJECT_V(pformat, 1);
}
} // namespace frst
