#include <frost/value.hpp>

#include "operators-common.hpp"

namespace frst
{

[[noreturn]] void compare_err(std::string_view op_glyph,
                              std::string_view lhs_type,
                              std::string_view rhs_type)
{
    op_err("compare", op_glyph, lhs_type, rhs_type);
}

struct Compare_Primitive_Equal_Impl
{
    template <Frost_Primitive T>
    static bool operator()(const T& lhs, const T& rhs)
    {
        return lhs == rhs;
    }
    static bool operator()(const auto&, const auto&)
    {
        THROW_UNREACHABLE;
    }
} constexpr static compare_primitive_equal_impl;

struct Compare_Primitive_Less_Than_Impl
{
    static bool operator()(const Frost_Primitive auto& lhs,
                           const Frost_Primitive auto& rhs)
        requires requires {
            { lhs < rhs } -> std::same_as<bool>;
        }
    {
        return lhs < rhs;
    }
    static bool operator()(const auto&, const auto&)
    {
        THROW_UNREACHABLE;
    }
} constexpr static compare_primitive_less_than_impl;

bool Value::equal_impl(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    const auto& lhs_var = lhs->value_;
    const auto& rhs_var = rhs->value_;

    if (lhs_var.index() != rhs_var.index())
        return false; // different types are always unequal

    if (lhs->is_primitive()) // primitives use value comparison
        return std::visit(compare_primitive_equal_impl, lhs_var, rhs_var);

    return lhs == rhs;
}

bool Value::not_equal_impl(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    return !equal_impl(lhs, rhs);
}

bool Value::less_than_impl(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    const auto& lhs_var = lhs->value_;
    const auto& rhs_var = rhs->value_;

    // different numeric types can be compared with <  (float < int is
    // well-defined)
    if (lhs->is_numeric() && rhs->is_numeric())
        return std::visit(compare_primitive_less_than_impl, lhs_var, rhs_var);

    // otherwise types must be the same
    if (lhs_var.index() != rhs_var.index())
        compare_err("<", lhs->type_name(), rhs->type_name());

    if (lhs->is<Bool>() || lhs->is<Null>())
        compare_err("<", lhs->type_name(), rhs->type_name());

    // primitives use value comparison
    if (lhs->is_primitive()) // types are the same, so only need to check one
        return std::visit(compare_primitive_less_than_impl, lhs_var, rhs_var);
    else // non-primitive types cannot be ordered
        compare_err("<", lhs->type_name(), rhs->type_name());
}

bool Value::less_than_or_equal_impl(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    if (lhs->value_.index() != rhs->value_.index())
        compare_err("<=", lhs->type_name(), rhs->type_name());

    return less_than_impl(lhs, rhs) || equal_impl(lhs, rhs);
}

bool Value::greater_than_impl(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    if (lhs->value_.index() != rhs->value_.index())
        compare_err(">", lhs->type_name(), rhs->type_name());

    return !less_than_or_equal_impl(lhs, rhs);
}

bool Value::greater_than_or_equal_impl(const Value_Ptr& lhs,
                                       const Value_Ptr& rhs)
{
    if (lhs->value_.index() != rhs->value_.index())
        compare_err(">=", lhs->type_name(), rhs->type_name());

    return greater_than_impl(lhs, rhs) || equal_impl(lhs, rhs);
}

} // namespace frst
