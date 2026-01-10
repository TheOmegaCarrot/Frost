#include "builtins-common.h"

#include <ranges>

namespace frst
{

// arity is pre-checked by Builtin::call

// Map -> array of keys
// basically `map a_map with fn (k, v) -> { k }`
Value_Ptr keys(builtin_args_t args)
{
    const auto& arg = args.at(0);

    if (not arg->is<Map>())
    {
        throw Frost_Error{
            fmt::format("Function keys requires Map as argument, got {}",
                        arg->type_name())};
    }

    return Value::create(arg->raw_get<Map>() | std::views::keys |
                         std::ranges::to<std::vector>());
}

// Map -> array of values
// basically `map a_map with fn (k, v) -> { b }`
Value_Ptr values(builtin_args_t args)
{
    const auto& arg = args.at(0);

    if (not arg->is<Map>())
    {
        throw Frost_Error{
            fmt::format("Function values requires Map as argument, got {}",
                        arg->type_name())};
    }

    return Value::create(arg->raw_get<Map>() | std::views::values |
                         std::ranges::to<std::vector>());
}

void inject_map_ops(Symbol_Table& table)
{
    INJECT(keys, 1, 1);
    INJECT(values, 1, 1);
}
} // namespace frst
