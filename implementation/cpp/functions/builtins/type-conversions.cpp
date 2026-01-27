#include "builtins-common.hpp"

#include <frost/builtin.hpp>

namespace frst
{
BUILTIN(to_string)
{
    return args.at(0)->to_string();
}

BUILTIN(pretty)
{
    return args.at(0)->to_pretty_string();
}

BUILTIN(to_int)
{
    return args.at(0)->to_int();
}

BUILTIN(to_float)
{
    return args.at(0)->to_float();
}

void inject_type_conversions(Symbol_Table& table)
{
    INJECT(to_string, 1, 1);
    INJECT(pretty, 1, 1);
    INJECT(to_int, 1, 1);
    INJECT(to_float, 1, 1);
}
} // namespace frst
