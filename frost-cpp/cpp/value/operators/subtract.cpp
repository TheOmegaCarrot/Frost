#include <frost/value.hpp>

#include "operators-common.hpp"

namespace frst
{

[[noreturn]] void subtract_err(std::string_view lhs_type,
                               std::string_view rhs_type)
{
    op_err("subtract", "-", lhs_type, rhs_type);
}

struct Numeric_Subtract_Impl
{
    template <Frost_Numeric LHS_T, Frost_Numeric RHS_T>
    static Value operator()(const LHS_T& lhs, const RHS_T& rhs)
    {
        return lhs - rhs;
    }
    static Value operator()(const auto&, const auto&)
    {
        THROW_UNREACHABLE;
    }
} constexpr static numeric_subtract_impl;

Value_Ptr Value::subtract(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    const auto& lhs_var = lhs->value_;
    const auto& rhs_var = rhs->value_;

    if (lhs->is_numeric() && rhs->is_numeric())
        return Value::create(
            std::visit(numeric_subtract_impl, lhs_var, rhs_var));

    subtract_err(lhs->type_name(), rhs->type_name());
}

} // namespace frst
