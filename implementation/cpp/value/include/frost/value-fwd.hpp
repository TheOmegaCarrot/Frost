#ifndef FROST_VALUE_FWD_HPP
#define FROST_VALUE_FWD_HPP

#include <memory>

namespace frst
{

class Value;
using Value_Ptr = std::shared_ptr<const Value>;

namespace impl
{
//! An implementation of less-than ONLY for comparing keys in a map
struct Value_Ptr_Less
{
    static bool operator()(const Value_Ptr& lhs, const Value_Ptr& rhs);
};
} // namespace impl

} // namespace frst

#endif
