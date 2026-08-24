#include <frost/builtins-common.hpp>

#include <frost/builtin.hpp>

namespace frst
{
BUILTIN(to_string)
{
    REQUIRE_ARGS("to_string", ANY);
    return args.at(0)->to_string();
}

BUILTIN(pretty)
{
    REQUIRE_ARGS("pretty", ANY);
    return args.at(0)->to_pretty_string();
}

BUILTIN(to_int)
{
    REQUIRE_ARGS("to_int", ANY);
    return args.at(0)->to_int();
}

BUILTIN(to_float)
{
    REQUIRE_ARGS("to_float", ANY);
    return args.at(0)->to_float();
}

void inject_type_conversions(Symbol_Table& table)
{
    INJECT(to_string);
    INJECT(pretty);
    INJECT(to_int);
    INJECT(to_float);
}
} // namespace frst
