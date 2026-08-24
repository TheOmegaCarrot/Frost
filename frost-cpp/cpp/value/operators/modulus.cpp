#include <frost/value.hpp>

#include "operators-common.hpp"

namespace frst
{

[[noreturn]] void modulus_err(std::string_view lhs_type,
                              std::string_view rhs_type)
{
    op_err("modulus", "%", lhs_type, rhs_type);
}

struct Numeric_Modulus_Impl
{
    static Value operator()(const Int& lhs, const Int& rhs)
    {
        if (rhs == 0)
            throw Frost_Recoverable_Error{"Modulus by zero"};

        if (lhs == std::numeric_limits<Int>::min() && rhs == -1)
            throw Frost_Recoverable_Error{
                fmt::format("{} % {} is invalid", lhs, rhs)};

        return lhs % rhs;
    }
    static Value operator()(const auto&, const auto&)
    {
        THROW_UNREACHABLE;
    }
} constexpr static numeric_modulus_impl;

Value_Ptr Value::modulus(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    const auto& lhs_var = lhs->value_;
    const auto& rhs_var = rhs->value_;

    if (lhs->is<Int>() && rhs->is<Int>())
        return Value::create(
            std::visit(numeric_modulus_impl, lhs_var, rhs_var));

    modulus_err(lhs->type_name(), rhs->type_name());
}

} // namespace frst
