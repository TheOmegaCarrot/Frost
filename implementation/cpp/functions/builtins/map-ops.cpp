#include "builtins-common.h"

#include <ranges>

namespace frst
{

// arity is pre-checked by Builtin::call

// Map -> array of keys
// basically `map a_map with fn (k, v) -> { k }`
Value_Ptr keys(builtin_args_t args)
{
    REQUIRE_ARGS(keys, TYPES(Map));

    return Value::create(args.at(0)->raw_get<Map>() | std::views::keys |
                         std::ranges::to<std::vector>());
}

// Map -> array of values
// basically `map a_map with fn (k, v) -> { b }`
Value_Ptr values(builtin_args_t args)
{
    REQUIRE_ARGS(values, TYPES(Map));

    return Value::create(args.at(0)->raw_get<Map>() | std::views::values |
                         std::ranges::to<std::vector>());
}

void inject_map_ops(Symbol_Table& table)
{
    INJECT(keys, 1, 1);
    INJECT(values, 1, 1);
}
} // namespace frst
