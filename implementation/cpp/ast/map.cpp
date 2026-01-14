#include <ranges>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include "map.hpp"

using namespace frst;

namespace
{
Value_Ptr map_array(const Array& arr, const Function& op)
{
    return Value::create(arr
                         | std::views::transform([&](const Value_Ptr& elem) {
                               return op->call({elem});
                           })
                         | std::ranges::to<Array>());
}

Value_Ptr map_map(const Map& arr, const Function& op)
{
    THROW_UNREACHABLE;
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
        return map_array(structure_val->raw_get<Array>(),
                         op_val->raw_get<Function>());

    if (structure_val->is<::frst::Map>())
        return map_map(structure_val->raw_get<::frst::Map>(),
                       op_val->raw_get<Function>());

    THROW_UNREACHABLE;
}
