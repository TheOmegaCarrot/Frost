#include "builtins-common.hpp"
#include "frost/value.hpp"

#include <frost/builtin.hpp>

#include <ranges>

namespace frst
{

// arity is pre-checked by Builtin::call

// Map -> array of keys
// basically `map a_map with fn (k, v) -> { k }`
BUILTIN(keys)
{
    REQUIRE_ARGS("keys", TYPES(Map));

    return Value::create(
        GET(0, Map) | std::views::keys | std::ranges::to<std::vector>());
}

// Map -> array of values
// basically `map a_map with fn (k, v) -> { b }`
BUILTIN(values)
{
    REQUIRE_ARGS("values", TYPES(Map));

    return Value::create(
        GET(0, Map) | std::views::values | std::ranges::to<std::vector>());
}

BUILTIN(len)
{
    REQUIRE_ARGS("len", TYPES(Map, Array, String));

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

BUILTIN(range)
{
    using std::views::iota, std::views::transform, std::ranges::to;
    constexpr auto make = [](Int arg) {
        return Value::create(auto{arg});
    };
    if (args.size() == 1)
    {
        REQUIRE_ARGS("range", PARAM("upper bound", TYPES(Int)));

        auto upper_bound = GET(0, Int);

        if (upper_bound <= 0)
            return Value::create(Array{});

        return Value::create(
            iota(0, upper_bound) | transform(make) | to<std::vector>());
    }
    else
    {
        REQUIRE_ARGS("range", PARAM("lower bound", TYPES(Int)),
                     PARAM("upper bound", TYPES(Int)));

        auto lower_bound = GET(0, Int);
        auto upper_bound = GET(1, Int);

        if (upper_bound <= lower_bound)
            return Value::create(Array{});

        return Value::create(iota(lower_bound, upper_bound)
                             | transform(make)
                             | to<std::vector>());
    }
}

void inject_structure_ops(Symbol_Table& table)
{
    INJECT(keys, 1, 1);
    INJECT(values, 1, 1);
    INJECT(len, 1, 1);
    INJECT(range, 1, 2);
}
} // namespace frst
