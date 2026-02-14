#include <frost/value.hpp>

#include <variant>

namespace frst::impl
{

bool impl::Value_Ptr_Less::operator()(const Value_Ptr& lhs,
                                      const Value_Ptr& rhs)
{
    const auto lhs_index = lhs->value_.index();
    const auto rhs_index = rhs->value_.index();

    if (lhs_index != rhs_index)
        return lhs_index < rhs_index;

    if (lhs->is_primitive())
    {
        return std::visit(Overload{
                              []<Frost_Primitive T>(const T& a, const T& b) {
                                  return a < b;
                              },
                              [](const auto&, const auto&) -> bool {
                                  THROW_UNREACHABLE;
                              },
                          },
                          lhs->value_, rhs->value_);
    }

    return lhs.get() < rhs.get();
}

} // namespace frst::impl
