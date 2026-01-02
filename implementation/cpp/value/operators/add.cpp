#include <frost/value.hpp>

#include "operators-common.hpp"

#include <algorithm>
#include <ranges>

namespace frst
{

[[noreturn]] void add_err(std::string_view lhs_type, std::string_view rhs_type)
{
    op_err("add", lhs_type, rhs_type);
}

struct Numeric_Add_Impl
{
    template <Frost_Numeric LHS_T, Frost_Numeric RHS_T>
    static Value operator()(const LHS_T& lhs, const RHS_T& rhs)
    {
        return lhs + rhs;
    }
    static Value operator()(const auto&, const auto&)
    {
        THROW_UNREACHABLE;
    }
} constexpr static numeric_add_impl;

struct Array_Cat_Impl
{
    static Value operator()(const Array& lhs, const Array& rhs)
    {
        return std::views::concat(lhs, rhs) | std::ranges::to<Array>();
    }
    static Value operator()(const auto&, const auto&)
    {
        THROW_UNREACHABLE;
    }
} constexpr static array_cat_impl;

struct Map_Union_Impl
{
    static Value operator()(const Map& lhs, const Map& rhs)
    {
        return std::views::concat(lhs, rhs) | std::ranges::to<Map>();
    }
    static Value operator()(const auto&, const auto&)
    {
        THROW_UNREACHABLE;
    }
} constexpr static map_union_impl;

Value_Ptr Value::add(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    const auto& lhs_var = lhs->value_;
    const auto& rhs_var = rhs->value_;

    if (lhs->is_numeric() && rhs->is_numeric())
        return Value::create(std::visit(numeric_add_impl, lhs_var, rhs_var));

    if (lhs->is<Array>() && rhs->is<Array>())
        return Value::create(std::visit(array_cat_impl, lhs_var, rhs_var));

    if (lhs->is<Map>() && rhs->is<Map>())
        return Value::create(std::visit(map_union_impl, lhs_var, rhs_var));

    add_err(lhs->type_name(), rhs->type_name());
}

} // namespace frst
