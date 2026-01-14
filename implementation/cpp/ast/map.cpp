#include <ranges>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include "map.hpp"

using namespace frst;

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

Value_Ptr map_map(const Value_Ptr& map_val, const Function& op)
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
            throw Frost_Error{fmt::format(
                "Map with map input requires map intermediates, got {}",
                intermediate_val->type_name())};
        }

        const Map& intermediate = intermediate_val->raw_get<Map>();

        for (const auto& [i_k, i_v] : intermediate)
        {
            if (acc.contains(i_k))
            {
                throw Frost_Error(
                    fmt::format("Map operation key collision with key: {}",
                                i_k->to_internal_string()));
            }

            acc.insert({i_k, i_v});
        }
    }

    return Value::create(std::move(acc));
}
} // namespace

Value_Ptr ast::Map::evaluate(const Symbol_Table& syms) const
{
    const auto& structure_val = structure_->evaluate(syms);
    if (not structure_val->is_structured())
    {
        throw Frost_Error{fmt::format("Cannot map value with type {}",
                                      structure_val->type_name())};
    }

    const auto& op_val = operation_->evaluate(syms);
    if (not op_val->is<Function>())
    {
        throw Frost_Error{fmt::format("Map operation expected Function, got {}",
                                      op_val->type_name())};
    }

    if (structure_val->is<Array>())
        return map_array(structure_val, op_val->raw_get<Function>());

    if (structure_val->is<::frst::Map>())
        return map_map(structure_val, op_val->raw_get<Function>());

    THROW_UNREACHABLE;
}
