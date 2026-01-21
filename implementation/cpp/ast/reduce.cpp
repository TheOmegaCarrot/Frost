#include "reduce.hpp"

#include <frost/value.hpp>

#include <algorithm>

using namespace frst;

namespace
{
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

Value_Ptr ast::Reduce::evaluate(const Symbol_Table& syms) const
{
    const auto& structure_val = structure_->evaluate(syms);
    if (not structure_val->is_structured())
    {
        throw Frost_Recoverable_Error{fmt::format("Cannot reduce value with type {}",
                                           structure_val->type_name())};
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

    if (structure_val->is<Array>())
        return reduce_array(structure_val->raw_get<Array>(),
                            op_val->raw_get<Function>(), init);

    if (structure_val->is<Map>())
        return reduce_map(structure_val->raw_get<Map>(),
                          op_val->raw_get<Function>(), init);

    THROW_UNREACHABLE;
}
