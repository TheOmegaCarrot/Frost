#include "frost/value.hpp"
#include <frost/builtins-common.hpp>

#include <frost/builtin.hpp>

#include <algorithm>
#include <ranges>

namespace frst
{

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
    REQUIRE_ARITY("range", 1, 3);

    using std::views::iota, std::views::transform, std::views::stride,
        std::views::reverse, std::ranges::to;
    constexpr auto make = [](Int arg) {
        return Value::create(auto{arg});
    };
    if (args.size() == 1)
    {
        REQUIRE_ARGS("range", PARAM("upper bound", TYPES(Int)));

        auto stop = GET(0, Int);

        if (stop <= 0)
            return Value::create(Array{});

        return Value::create(
            iota(0, stop) | transform(make) | to<std::vector>());
    }
    else if (args.size() == 2)
    {
        REQUIRE_ARGS("range", PARAM("lower bound", TYPES(Int)),
                     PARAM("upper bound", TYPES(Int)));

        auto start = GET(0, Int);
        auto stop = GET(1, Int);

        if (stop <= start)
            return Value::create(Array{});

        return Value::create(
            iota(start, stop) | transform(make) | to<std::vector>());
    }
    else
    {
        REQUIRE_ARGS("range", PARAM("start", TYPES(Int)),
                     PARAM("stop", TYPES(Int)), PARAM("step", TYPES(Int)));

        auto start = GET(0, Int);
        auto stop = GET(1, Int);
        auto step = GET(2, Int);

        if (step == 0)
            throw Frost_Recoverable_Error{"Function range requires step != 0"};

        if (step > 0)
        {
            if (start >= stop)
                return Value::create(Array{});
            return Value::create(iota(start, stop)
                                 | stride(step)
                                 | transform(make)
                                 | to<std::vector>());
        }
        else
        {
            if (start <= stop)
                return Value::create(Array{});
            return Value::create(iota(stop + 1, start + 1)
                                 | reverse
                                 | stride(-step)
                                 | transform(make)
                                 | to<std::vector>());
        }
    }
}

BUILTIN(nulls)
{
    REQUIRE_ARGS("nulls", TYPES(Int));

    const auto count = GET(0, Int);

    if (count < 0)
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Function nulls requires positive argument, got {}", count)};
    }

    return Value::create(std::views::repeat(Value::null())
                         | std::views::take(count)
                         | std::ranges::to<Array>());
}

BUILTIN(id)
{
    REQUIRE_ARGS("id", ANY);
    return args.at(0);
}

BUILTIN(has)
{
    REQUIRE_ARGS("has", PARAM("structure", TYPES(Array, Map)),
                 PARAM("index", TYPES(String, Int, Float, Bool)));

    if (IS(0, Map))
    {
        const auto& map = GET(0, Map);
        return Value::create(map.find(args.at(1)) != map.end());
    }
    else
    {
        if (not IS(1, Int))
            throw Frost_Recoverable_Error{fmt::format(
                "Function has with Array requires Int as argument 2, got: {}",
                args.at(1)->type_name())};

        const auto& array = GET(0, Array);

        return Value::create(
            Value::index_array(array, GET(1, Int)).has_value());
    }
}

BUILTIN(includes)
{
    REQUIRE_ARGS("includes", TYPES(Array), ANY);

    const auto& arr = GET(0, Array);

    const auto itr = std::ranges::find_if(arr, [&](const Value_Ptr& elem) {
        return Value::equal(elem, args.at(1))->truthy();
    });

    return Value::create(itr != arr.end());
}

BUILTIN(to_entries)
{
    REQUIRE_ARGS("to_entries", TYPES(Map));
    STRINGS(key, value);

    const auto& in = GET(0, Map);

    return Value::create(in
                         | std::views::transform([&](const auto& pair) {
                               return Value::create(
                                   Value::trusted,
                                   Map{
                                       {strings.key, pair.first},
                                       {strings.value, pair.second},
                                   });
                           })
                         | std::ranges::to<Array>());
}

BUILTIN(from_entries)
{
    REQUIRE_ARGS("from_entries", TYPES(Array));
    STRINGS(key, value);

    const auto& in = GET(0, Array);

    Array ks;
    Array vs;
    ks.reserve(in.size());
    vs.reserve(in.size());
    Map result{std::sorted_unique, std::move(ks), std::move(vs)};
    for (const auto& [i, elem] : std::views::enumerate(in))
    {
        if (not elem->is<Map>())
            throw Frost_Recoverable_Error{
                fmt::format("from_entries: element {} is {}, expected Map", i,
                            elem->type_name())};

        const auto& entry = elem->raw_get<Map>();

        const auto key_itr = entry.find(strings.key);
        if (key_itr == entry.end())
            throw Frost_Recoverable_Error{
                fmt::format("from_entries: element {} is missing 'key'", i)};

        const auto& k = key_itr->second;
        if (k->visit([&]<typename T>(const T&) {
                return not Frost_Map_Key<T>;
            }))
            throw Frost_Recoverable_Error{
                fmt::format("from_entries: element {} has invalid key type: {}",
                            i, k->type_name())};

        const auto value_itr = entry.find(strings.value);
        if (value_itr == entry.end())
            throw Frost_Recoverable_Error{
                fmt::format("from_entries: element {} is missing 'value'", i)};

        const auto& v = value_itr->second;

        result.insert_or_assign(k, v);
    }

    return Value::create(Value::trusted, std::move(result));
}

BUILTIN(dissoc)
{
    REQUIRE_ARGS("dissoc", TYPES(Map),
                 VARIADIC_REST(1, "key", TYPES(Int, Float, Bool, String)));

    auto input = GET(0, Map);

    for (const auto& val : args | std::views::drop(1))
    {
        input.erase(val); // maybe no-op and that's ok
    }

    return Value::create(Value::trusted, std::move(input));
}

void inject_structure_ops(Symbol_Table& table)
{
    INJECT(keys);
    INJECT(values);
    INJECT(len);
    INJECT(range);
    INJECT(nulls);
    INJECT(id);
    INJECT(has);
    INJECT(includes);
    INJECT(to_entries);
    INJECT(from_entries);
    INJECT(dissoc);
}
} // namespace frst
