#include "reduce.hpp"
#include "frost/value.hpp"

#include <algorithm>

using namespace frst;

namespace
{
Value_Ptr reduce_array(const Array& arr, const Function& op,
                       const Value_Ptr& init)
{
    const auto reduction = [&](const Value_Ptr& acc, const Value_Ptr& elem) {
        return op->call({acc, elem});
    };

    if (init->is<Null>())
    {
        return std::ranges::fold_left_first(arr, reduction)
            .or_else([&] { return std::optional{Value::create(Null{})}; })
            .value();
    }
    else
    {
        return std::ranges::fold_left(arr, init, reduction);
    }
}
} // namespace

[[nodiscard]] Value_Ptr ast::Reduce::evaluate(const Symbol_Table& syms) const
{
    const auto& structure_val = structure_->evaluate(syms);
    if (not structure_val->is_structured())
    {
        throw Frost_Error{fmt::format("Cannot reduce value with type",
                                      structure_val->type_name())};
    }

    const auto& op_val = operation_->evaluate(syms);
    if (not op_val->is<Function>())
    {
        throw Frost_Error{fmt::format("Reduce with expected Function, got {}",
                                      op_val->type_name())};
    }

    auto init = init_
                    .transform([&](const Expression::Ptr& expr) {
                        return expr->evaluate(syms);
                    })
                    .or_else([] -> std::optional<Value_Ptr> {
                        return Value::create(Null{});
                    })
                    .value();

    if (structure_val->is<Array>())
        return reduce_array(structure_val->raw_get<Array>(),
                            op_val->raw_get<Function>(), init);

    THROW_UNREACHABLE;
}
