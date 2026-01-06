#include <frost/value.hpp>

namespace frst
{
Value_Ptr Value::logical_and(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    if (lhs->as<Bool>())
        return rhs;
    else
        return lhs;
}

Value_Ptr Value::logical_or(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    if (lhs->as<Bool>())
        return rhs;
    else
        return lhs;
}

Value_Ptr Value::logical_not() const
{
    return Value::create(not as<Bool>());
}
} // namespace frst
