#include <frost/value.hpp>

#include <fmt/format.h>

namespace frst
{
struct Negate_Impl
{
    static Value operator()(const Frost_Numeric auto& value)
    {
        return -value;
    }

    static Value operator()(const auto&)
    {
        THROW_UNREACHABLE;
    }
} constexpr static negate_impl;

Value_Ptr Value::negate() const
{
    if (!is_numeric())
        throw Frost_Recoverable_Error{
            fmt::format("Invalid operand for unary - : {}", type_name())};

    return Value::create(value_.visit(negate_impl));
}
} // namespace frst
