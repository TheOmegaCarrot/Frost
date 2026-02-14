#include <frost/ast/filter.hpp>

#include <frost/value.hpp>

using namespace frst;

Value_Ptr ast::Filter::evaluate(const Symbol_Table& syms) const
{

    const auto& structure_val = structure_->evaluate(syms);
    if (not structure_val->is_structured())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Cannot filter value with type {}", structure_val->type_name())};
    }

    const auto& op_val = operation_->evaluate(syms);
    if (not op_val->is<Function>())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Filter operation expected Function, got {}", op_val->type_name())};
    }

    return Value::do_filter(structure_val, op_val->raw_get<Function>());
}
