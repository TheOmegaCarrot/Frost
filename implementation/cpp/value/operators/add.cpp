#include <frost/value.hpp>

#include "operators-common.hpp"

#include <ranges>

namespace frst
{

[[noreturn]] void add_err(std::string_view lhs_type, std::string_view rhs_type)
{
    op_err("add", "+", lhs_type, rhs_type);
}

struct Add_Impl
{
    static Value operator()(const Frost_Numeric auto& lhs,
                            const Frost_Numeric auto& rhs)
    {
        return lhs + rhs;
    }
    static Value operator()(const String& lhs, const String& rhs)
    {
        return lhs + rhs;
    }
    static Value operator()(const Array& lhs, const Array& rhs)
    {
        return std::views::concat(lhs, rhs) | std::ranges::to<Array>();
    }
    static Value operator()(const Map& lhs, const Map& rhs)
    {
        Map acc;
        for (const auto& [k, v] : std::views::concat(lhs, rhs))
        {
            acc.insert_or_assign(k, v);
        }

        return acc;
    }
    template <typename T, typename U>
    static Value operator()(const T&, const U&)
    {
        add_err(type_str<T>(), type_str<U>());
    }
} constexpr static add_impl;

Value_Ptr Value::add(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    return Value::create(std::visit(add_impl, lhs->value_, rhs->value_));
}

} // namespace frst
