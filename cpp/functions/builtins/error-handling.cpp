#include <frost/builtins-common.hpp>

#include <frost/backtrace.hpp>
#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

using namespace std::literals;
namespace frst
{

namespace
{

Value_Ptr format_backtrace_as_value(const Backtrace& bt)
{
    Array frames;
    for (const auto& frame : bt.frames() | std::views::reverse)
    {
        std::string text = std::visit(
            Overload{
                [](const Resolved_AST_Frame& f) {
                    return fmt::format("{} [{}]", f.node_label,
                                       f.source_range);
                },
                [](const Call_Frame& f) {
                    return fmt::format("Call ({})", f.function_name);
                },
                [](const Import_Frame& f) {
                    return fmt::format("Import Boundary ({})", f.module_spec);
                },
                [](const Iterative_Frame& f) {
                    return fmt::format("{} ({})", f.operation, f.function_name);
                },
            },
            frame);
        frames.push_back(Value::create(std::move(text)));
    }
    return Value::create(std::move(frames));
}

} // namespace

void inject_error_handling(Symbol_Table& table, Backtrace_State* bt)
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

                    if (auto snapshot = err.pilfer_backtrace())
                    {
                        result_map.emplace(
                            strings.trace,
                            format_backtrace_as_value(*snapshot));
                    }

                    return Value::create(Value::trusted,
                                         std::move(result_map));
                }
            },
            "try_call", Builtin::Arity{.min = 2, .max = 2})}));
}

} // namespace frst
