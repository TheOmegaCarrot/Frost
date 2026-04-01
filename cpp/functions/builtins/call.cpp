#include <frost/builtins-common.hpp>

namespace frst
{
BUILTIN(call)
{
    // clang-format off
    REQUIRE_ARGS("call",
            PARAM("function", TYPES(Function)),
            PARAM("args", TYPES(Array)));
    // clang-format on

    return GET(0, Function)->call(GET(1, Array));
}

void inject_call(Symbol_Table& table)
{
    INJECT(call);
}
} // namespace frst
