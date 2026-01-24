#include "builtins-common.hpp"

namespace frst
{
BUILTIN(pack_call)
{
    // clang-format off
    REQUIRE_ARGS("pack_call",
            PARAM("function", TYPES(Function)),
            PARAM("args", TYPES(Array)));
    // clang-format on

    return GET(0, Function)->call(GET(1, Array));
}

void inject_pack_call(Symbol_Table& table)
{
    INJECT(pack_call, 2, 2);
}
} // namespace frst
