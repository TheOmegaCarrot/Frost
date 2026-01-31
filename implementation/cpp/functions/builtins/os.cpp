#include <frost/builtins-common.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst
{

BUILTIN(getenv)
{
    REQUIRE_ARGS("getenv", TYPES(String));

    const char* val = std::getenv(GET(0, String).c_str());

    if (not val)
        return Value::null();

    return Value::create(String{val});
}

BUILTIN(exit)
{
    REQUIRE_ARGS("exit", TYPES(Int));

    std::exit(GET(0, Int));
}

void inject_os(Symbol_Table& table)
{
    INJECT(getenv, 1, 1);
    INJECT(exit, 1, 1);
}

} // namespace frst
