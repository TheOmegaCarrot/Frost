#include <numeric>
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

    auto transform = [&](const auto& kv) {
        const auto& [k, v] = kv;
        return op->call({k, v});
    };

    return std::transform_reduce(map.begin(), map.end(), Value::create(Map{}),
                                 &Value::add, transform);
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
