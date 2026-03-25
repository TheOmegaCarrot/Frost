#include <frost/builtins-common.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>
#include <thread>

namespace frst
{

namespace os
{

BUILTIN(getenv)
{
    REQUIRE_ARGS("os.getenv", TYPES(String));

    const char* val = std::getenv(GET(0, String).c_str());

    if (not val)
        return Value::null();

    return Value::create(String{val});
}

BUILTIN(exit)
{
    REQUIRE_ARGS("os.exit", TYPES(Int));

    std::exit(GET(0, Int));
}

BUILTIN(sleep)
{
    REQUIRE_ARGS("os.sleep", TYPES(Int));

    std::this_thread::sleep_for(std::chrono::milliseconds(GET(0, Int)));

    return Value::null();
}

} // namespace os

STDLIB_MODULE(os, ENTRY(getenv, 1), ENTRY(exit, 1), ENTRY(sleep, 1))

} // namespace frst
