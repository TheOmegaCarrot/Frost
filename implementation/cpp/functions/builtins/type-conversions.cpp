#include "builtins-common.hpp"

namespace frst
{
Value_Ptr to_string(builtin_args_t args)
{
    return args.at(0)->to_string();
}

void inject_type_conversions(Symbol_Table& table)
{
    INJECT(to_string, 1, 1);
}
} // namespace frst
