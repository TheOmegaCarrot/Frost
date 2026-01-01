#include <frost/value.hpp>

namespace frst
{

Value_Ptr Value::add(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    const auto& lhs_var = lhs->value_;
    const auto& rhs_var = rhs->value_;

    return {}; // PLACEHOLDER SO THIS COMPILES
}

} // namespace frst
