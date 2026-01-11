#include "builtins-common.hpp"
#include "frost/value.hpp"

#include <frost/builtin.hpp>

#include <ranges>

namespace frst
{

// arity is pre-checked by Builtin::call

// Map -> array of keys
// basically `map a_map with fn (k, v) -> { k }`
Value_Ptr keys(builtin_args_t args)
{
    REQUIRE_ARGS(keys, TYPES(Map));

    return Value::create(args.at(0)->raw_get<Map>()
                         | std::views::keys
                         | std::ranges::to<std::vector>());
}

// Map -> array of values
// basically `map a_map with fn (k, v) -> { b }`
Value_Ptr values(builtin_args_t args)
{
    REQUIRE_ARGS(values, TYPES(Map));

    return Value::create(args.at(0)->raw_get<Map>()
                         | std::views::values
                         | std::ranges::to<std::vector>());
}

Value_Ptr len(builtin_args_t args)
{
    REQUIRE_ARGS(len, TYPES(Map, Array, String));

    if (const auto& arg = args.at(0); arg->is<Map>())
    {
        return Value::create(static_cast<Int>(arg->raw_get<Map>().size()));
    }
    else if (arg->is<Array>())
    {
        return Value::create(static_cast<Int>(arg->raw_get<Array>().size()));
    }
    else if (arg->is<String>())
    {
        return Value::create(static_cast<Int>(arg->raw_get<String>().size()));
    }

    THROW_UNREACHABLE;
}

void inject_map_ops(Symbol_Table& table)
{
    INJECT(keys, 1, 1);
    INJECT(values, 1, 1);
    INJECT(len, 1, 1);
}
} // namespace frst
