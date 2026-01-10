#include "builtins-common.hpp"

namespace frst
{
Value_Ptr pack_call(builtin_args_t args)
{
    // clang-format off
    REQUIRE_ARGS(pack_call,
            PARAM("function", TYPES(Function)),
            PARAM("args", TYPES(Array)));
    // clang-format on

    return args.at(0)->raw_get<Function>()->call(args.at(1)->raw_get<Array>());
}

void inject_pack_call(Symbol_Table& table)
{
    INJECT(pack_call, 2, 2);
}
} // namespace frst
