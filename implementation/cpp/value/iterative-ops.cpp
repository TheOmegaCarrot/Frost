#include <frost/value.hpp>

#include <algorithm>
#include <ranges>

namespace frst
{

namespace
{

Value_Ptr map_array(const Value_Ptr& arr_val, const Function& op)
{
    const auto& arr = arr_val->raw_get<Array>();

    // map empty array -> empty array back
    if (arr.empty())
        return arr_val;

    return Value::create(arr
                         | std::views::transform([&](const Value_Ptr& elem) {
                               return op->call({elem});
                           })
                         | std::ranges::to<Array>());
}

Value_Ptr map_map(const Value_Ptr& map_val, const Function& op,
                  std::string_view parent_op_name)
{
    const auto& map = map_val->raw_get<Map>();

    // map empty map -> empty map back
    if (map.empty())
        return map_val;

    Map acc;
    for (const auto& [k, v] : map)
    {
        auto intermediate_val = op->call({k, v});

        if (not intermediate_val->is<Map>())
        {
            throw Frost_Recoverable_Error{fmt::format(
                "{} with map input requires map intermediates, got {}",
                parent_op_name, intermediate_val->type_name())};
        }

        const Map& intermediate = intermediate_val->raw_get<Map>();

        for (const auto& [i_k, i_v] : intermediate)
        {
            if (acc.contains(i_k))
            {
                throw Frost_Recoverable_Error(
                    fmt::format("{} operation key collision with key: {}",
                                parent_op_name, i_k->to_internal_string()));
            }

            acc.insert({i_k, i_v});
        }
    }

    return Value::create(std::move(acc));
}

Value_Ptr filter_map(const Map& map, const Function& pred)
{
    Map acc;

    for (const auto& [k, v] : map)
    {
        if (pred->call({k, v})->as<Bool>().value())
            acc.insert({k, v});
    }

    return Value::create(std::move(acc));
}

Value_Ptr filter_array(const Array& arr, const Function& pred)
{
    Array acc;

    for (const auto& elem : arr)
    {
        if (pred->call({elem})->as<Bool>().value())
            acc.push_back(elem);
    }

    return Value::create(std::move(acc));
}

Value_Ptr reduce_array(const Array& arr, const Function& op,
                       const std::optional<Value_Ptr>& init)
{
    const auto reduction = [&](const Value_Ptr& acc, const Value_Ptr& elem) {
        return op->call({acc, elem});
    };

    if (not init)
    {
        return std::ranges::fold_left_first(arr, reduction)
            .value_or(Value::null());
    }
    else
    {
        return std::ranges::fold_left(arr, *init, reduction);
    }
}

Value_Ptr reduce_map(const Map& arr, const Function& op,
                     const std::optional<Value_Ptr>& init)
{
    const auto reduction = [&](const Value_Ptr& acc, const auto& elem) {
        const auto& [k, v] = elem;
        return op->call({acc, k, v});
    };

    if (not init)
        throw Frost_Recoverable_Error{"Map reduction requires init"};

    return std::ranges::fold_left(arr, *init, reduction);
}
} // namespace

Value_Ptr Value::do_map(Value_Ptr structure, const Function& fn,
                        std::string_view parent_op_name)
{
    if (structure->is<Array>())
        return map_array(structure, fn);

    if (structure->is<frst::Map>())
        return map_map(structure, fn, parent_op_name);

    THROW_UNREACHABLE;
}

Value_Ptr Value::do_filter(Value_Ptr structure, const Function& fn)
{
    if (structure->is<Array>())
        return filter_array(structure->raw_get<Array>(), fn);

    if (structure->is<Map>())
        return filter_map(structure->raw_get<Map>(), fn);

    THROW_UNREACHABLE;
}

Value_Ptr Value::do_reduce(Value_Ptr structure, const Function& fn,
                           std::optional<Value_Ptr> init)
{
    if (structure->is<Array>())
        return reduce_array(structure->raw_get<Array>(), fn, init);

    if (structure->is<Map>())
        return reduce_map(structure->raw_get<Map>(), fn, init);

    THROW_UNREACHABLE;
}

} // namespace frst
