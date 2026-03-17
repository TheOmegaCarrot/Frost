#include <frost/builtins-common.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

using namespace std::literals;
namespace frst
{

void inject_error_handling(Symbol_Table& table)
{
    table.define(
        "try_call",
        Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t args) -> Value_Ptr {
                // clang-format off
                REQUIRE_ARGS("try_call",
                        PARAM("function", TYPES(Function)),
                        PARAM("args", TYPES(Array)));
                // clang-format on

                STRINGS(ok, value, error, trace);

                try
                {
                    return Value::create(
                        Value::trusted,
                        Map{{strings.value,
                             GET(0, Function)->call(GET(1, Array))},
                            {strings.ok, Value::create(true)}});
                }
                catch (Frost_Recoverable_Error& err)
                {
                    Map result_map{
                        {strings.ok, Value::create(false)},
                        {strings.error, Value::create(String{err.what()})}};

                    auto bt = err.take_backtrace();
                    if (!bt.empty())
                    {
                        Array frames;
                        frames.reserve(bt.size());
                        for (auto& frame : bt)
                            frames.push_back(
                                Value::create(std::move(frame)));
                        result_map.emplace(strings.trace,
                                           Value::create(std::move(frames)));
                    }

                    return Value::create(Value::trusted,
                                         std::move(result_map));
                }
            },
            "try_call", Builtin::Arity{.min = 2, .max = 2})}));
}

} // namespace frst
