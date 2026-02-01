#include <frost/builtins-common.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

using namespace std::literals;
namespace frst
{

BUILTIN(try_call)
{
    // clang-format off
    REQUIRE_ARGS("try_call",
            PARAM("function", TYPES(Function)),
            PARAM("args", TYPES(Array)));
    // clang-format on

    KEYS(ok, value, error);

    try
    {
        return Value::create(
            Map{{keys.value, GET(0, Function)->call(GET(1, Array))},
                {keys.ok, Value::create(true)}});
    }
    catch (const Frost_Recoverable_Error& err)
    {
        return Value::create(
            Map{{keys.ok, Value::create(false)},
                {keys.error, Value::create(String{err.what()})}});
    }
}

void inject_error_handling(Symbol_Table& table)
{
    INJECT(try_call, 2, 2);
}

} // namespace frst
