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

struct Compare_Equal_Impl
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
} constexpr static compare_equal_impl;

bool Value::equal_impl(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    const auto& lhs_var = lhs->value_;
    const auto& rhs_var = rhs->value_;

    if (lhs_var.index() != rhs_var.index())
        return false; // different types are always unequal

    if (lhs->is_primitive()) // primitives use value comparison
        return std::visit(compare_equal_impl, lhs_var, rhs_var);

    return lhs == rhs;
}

bool Value::not_equal_impl(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    return !equal_impl(lhs, rhs);
}

#define DEF_ORDERING(NAME, OP)                                                 \
    struct raw_##NAME##_fn_t                                                   \
    {                                                                          \
        static bool operator()(const Frost_Primitive auto& lhs,                \
                               const Frost_Primitive auto& rhs)                \
            requires requires {                                                \
                { lhs OP rhs } -> std::same_as<bool>;                          \
            }                                                                  \
        {                                                                      \
            return lhs OP rhs;                                                 \
        }                                                                      \
        static bool operator()(const auto&, const auto&)                       \
        {                                                                      \
            THROW_UNREACHABLE;                                                 \
        }                                                                      \
    } constexpr static raw_##NAME##_fn;                                        \
                                                                               \
    bool raw_##NAME(const auto& lhs, const auto& rhs)                          \
    {                                                                          \
        return std::visit(raw_##NAME##_fn, lhs, rhs);                          \
    }                                                                          \
                                                                               \
    bool Value::NAME##_impl(const Value_Ptr& lhs, const Value_Ptr& rhs)        \
    {                                                                          \
        if ((lhs->is_numeric() && rhs->is_numeric())                           \
            || (lhs->is<String>() && rhs->is<String>()))                       \
            return raw_##NAME(lhs->value_, rhs->value_);                       \
                                                                               \
        compare_err(#OP, lhs->type_name(), rhs->type_name());                  \
    }

DEF_ORDERING(less_than, <)
DEF_ORDERING(greater_than, >)
DEF_ORDERING(less_than_or_equal, <=)
DEF_ORDERING(greater_than_or_equal, >=)

} // namespace frst
