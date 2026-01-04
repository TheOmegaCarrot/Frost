#include <frost/value.hpp>

#include "operators-common.hpp"

namespace frst
{

[[noreturn]] void divide_err(std::string_view lhs_type,
                             std::string_view rhs_type)
{
    op_err("divide", lhs_type, rhs_type);
}

struct Numeric_Divide_Impl
{
    template <Frost_Numeric LHS_T, Frost_Numeric RHS_T>
    static Value operator()(const LHS_T& lhs, const RHS_T& rhs)
    {
        return lhs / rhs;
    }
    static Value operator()(const auto&, const auto&)
    {
        THROW_UNREACHABLE;
    }
} constexpr static numeric_divide_impl;

Value_Ptr Value::divide(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    const auto& lhs_var = lhs->value_;
    const auto& rhs_var = rhs->value_;

    if (lhs->is_numeric() && rhs->is_numeric())
        return Value::create(std::visit(numeric_divide_impl, lhs_var, rhs_var));

    divide_err(lhs->type_name(), rhs->type_name());
}

} // namespace frst
