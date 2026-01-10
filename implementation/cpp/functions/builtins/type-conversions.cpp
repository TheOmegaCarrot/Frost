#include "builtins-common.hpp"

#include <frost/builtin.hpp>

namespace frst
{
Value_Ptr to_string(builtin_args_t args)
{
    return args.at(0)->to_string();
}

Value_Ptr to_int(builtin_args_t args)
{
    return args.at(0)->to_int();
}

Value_Ptr to_float(builtin_args_t args)
{
    return args.at(0)->to_float();
}

void inject_type_conversions(Symbol_Table& table)
{
    INJECT(to_string, 1, 1);
    INJECT(to_int, 1, 1);
    INJECT(to_float, 1, 1);
}
} // namespace frst
