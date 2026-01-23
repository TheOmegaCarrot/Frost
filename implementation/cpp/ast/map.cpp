#include <ranges>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include "map.hpp"

using namespace frst;

Value_Ptr ast::Map::evaluate(const Symbol_Table& syms) const
{
    const auto& structure_val = structure_->evaluate(syms);
    if (not structure_val->is_structured())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Cannot map value with type {}", structure_val->type_name())};
    }

    const auto& op_val = operation_->evaluate(syms);
    if (not op_val->is<Function>())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Map operation expected Function, got {}", op_val->type_name())};
    }

    return Value::do_map(structure_val, op_val->raw_get<Function>(), "Map");
}
