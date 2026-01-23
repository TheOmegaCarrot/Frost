#include "reduce.hpp"

#include <frost/value.hpp>

using namespace frst;

Value_Ptr ast::Reduce::evaluate(const Symbol_Table& syms) const
{
    const auto& structure_val = structure_->evaluate(syms);
    if (not structure_val->is_structured())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Cannot reduce value with type {}", structure_val->type_name())};
    }

    const auto& op_val = operation_->evaluate(syms);
    if (not op_val->is<Function>())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Reduce operation expected Function, got {}", op_val->type_name())};
    }

    auto init = init_.transform([&](const Expression::Ptr& expr) {
        return expr->evaluate(syms);
    });

    return Value::do_reduce(structure_val, op_val->raw_get<Function>(), init);
}
